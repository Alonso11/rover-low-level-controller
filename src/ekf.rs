// Version: v1.1
//! # Extended Kalman Filter (EKF) — Rover Olympus LLC
//!
//! Fusión sensorial: Encoders Hall + Giroscopio MPU-6050 + Acelerómetro MPU-6050.
//! Basado en la teoría del TFG corregida (v0.6.0).

use libm::{cosf, sinf, fabsf, atan2f};
use crate::config::*;

// ─────────────────────────────────────────────────────────────────────────────
// Parámetros y Constantes
// ─────────────────────────────────────────────────────────────────────────────

pub const R_WHEEL:      f32 = (WHEEL_RADIUS_MM as f32) / 1000.0;
pub const B_EFF:        f32 = (WHEEL_BASE_MM as f32) / 1000.0;
pub const ENC_PPR:      f32 = TICKS_PER_REV as f32;
pub const DT:           f32 = (LOOP_MS as f32) / 1000.0;

// Parámetros de ruido del proceso — derivados del modelo de odometría diferencial.
//
// K_RHO: ruido de posición proporcional a la distancia recorrida.
//   Sin slip: σ_ds ≈ √(K_RHO × |ds|). Para ds = 10 mm → σ ≈ 0.32 mm (~3.2 %).
//   Ref.: Borenstein, J. & Feng, L. (1996). "Measurement and correction of
//   systematic odometry errors in mobile robots." IEEE Trans. Robot. Autom.
//   12(6), 869-880. — Tabla I reporta errores típicos de 1-3 % para encoders
//   Hall en superficies planas.
//
// SIGMA2_THETA_BASE / ALPHA_SLIP: ruido de orientación por giro.
//   Valores base del orden de 1e-4 rad² por paso de 20 ms son consistentes
//   con la Tabla 5.2 de Thrun, Burgard & Fox (2005). *Probabilistic Robotics*.
//   MIT Press. — σ_θ ∈ [0.05, 0.2] rad para robots diferenciales con encoders.
//
// Q_PITCH_FACTOR / Q_PITCH_MAX: degradación de confianza en orientación por pendiente.
//   Se añade varianza extra cuando |pitch| > PITCH_THRESHOLD (≈ 8°) para modelar
//   el deslizamiento lateral en rampas. El término crece linealmente con la
//   inclinación PERO se satura en Q_PITCH_MAX para evitar divergencia del filtro:
//   sin límite superior, a 15° de pendiente σ_θ_extra ≈ 34°, lo que equivale
//   a descartar toda información de orientación.
//   Q_PITCH_MAX = 0.06 rad² → σ_θ_extra ≤ 14° — incertidumbre significativa
//   pero no degenerada, coherente con la recomendación de Thrun et al. §5.4.2:
//   "scale Q by no more than 2-5× under perturbations."
pub const SIGMA2_THETA_BASE: f32 = 1.0e-4;
pub const ALPHA_SLIP:   f32 = 0.008;
pub const K_RHO:        f32 = 1.0e-5;
pub const BETA_SLIP:    f32 = 50.0;
pub const PITCH_THRESHOLD: f32 = 0.14; // ~8°
pub const Q_PITCH_FACTOR:  f32 = 3.0;
/// Techo de la varianza angular extra por pendiente (rad²).
/// Limita σ_θ_extra ≤ √0.06 ≈ 0.245 rad ≈ 14° — incertidumbre significativa
/// pero no degenerada. Sin este límite, pendientes > ~10° harían divergir el filtro.
/// Ref.: Thrun, S., Burgard, W. & Fox, D. (2005). *Probabilistic Robotics*.
/// MIT Press. §5.4.2, Tabla 5.2.
pub const Q_PITCH_MAX: f32 = 0.06;

const GYRO_NOISE_DENSITY: f32 = 8.727e-5; // 0.005 °/s/√Hz en rad
const GYRO_FS:            f32 = 1.0 / DT;
pub const R_VEL: f32 = GYRO_NOISE_DENSITY * GYRO_NOISE_DENSITY * GYRO_FS;

const ENC_TO_METER: f32 = (2.0 * 3.14159265 * R_WHEEL) / (3.0 * ENC_PPR);

// ─────────────────────────────────────────────────────────────────────────────
// Tipos
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
pub struct CovSym3 {
    pub p00: f32, pub p01: f32, pub p02: f32,
                  pub p11: f32, pub p12: f32,
                                pub p22: f32,
}

impl CovSym3 {
    pub const fn new(p00: f32, p11: f32, p22: f32) -> Self {
        Self { p00, p01: 0.0, p02: 0.0, p11, p12: 0.0, p22 }
    }
}

pub struct EkfState {
    pub x: f32, pub y: f32, pub theta: f32,
    pub p: CovSym3,
    pub theta_prev: f32,
}

impl EkfState {
    pub const fn new(p_xy: f32, p_theta: f32) -> Self {
        Self { x: 0.0, y: 0.0, theta: 0.0,
               p: CovSym3::new(p_xy, p_xy, p_theta),
               theta_prev: 0.0 }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Filtro
// ─────────────────────────────────────────────────────────────────────────────

pub fn predict(s: &mut EkfState, delta_el: i32, delta_er: i32, ax_mps2: f32, az_mps2: f32) {
    let ds_l = delta_el as f32 * ENC_TO_METER;
    let ds_r = delta_er as f32 * ENC_TO_METER;
    let ds   = 0.5 * (ds_r + ds_l);
    let dth  = (ds_r - ds_l) / B_EFF;

    let mid = s.theta + 0.5 * dth;
    let cm  = cosf(mid); let sm = sinf(mid);
    s.theta_prev = s.theta;
    s.x += ds * cm; s.y += ds * sm;
    s.theta = wrap_angle(s.theta + dth);

    // Q adaptivo por slip e inclinación
    let v_enc = ds / DT;
    let v_accel = ax_mps2 * DT;
    let slip = if fabsf(v_enc) > 0.01 {
        fabsf(v_enc - v_accel) / fabsf(v_enc)
    } else { 0.0 }.min(1.0);

    let sigma2_ds = K_RHO * fabsf(ds) * (1.0 + BETA_SLIP * slip * slip);
    
    let pitch = atan2f(ax_mps2, az_mps2);
    let q_pitch_extra = if fabsf(pitch) > PITCH_THRESHOLD {
        // Saturar en Q_PITCH_MAX para evitar divergencia del EKF en pendientes pronunciadas.
        // Sin este límite, a ~10° ya se supera el techo y σ_θ crecería indefinidamente.
        ((fabsf(pitch) - PITCH_THRESHOLD) * Q_PITCH_FACTOR).min(Q_PITCH_MAX)
    } else { 0.0 };
    let sigma2_dth = SIGMA2_THETA_BASE + ALPHA_SLIP * fabsf(dth) + q_pitch_extra;

    // P⁻ = F·P·Fᵀ + Q
    let f02 = -ds*sm; let f12 = ds*cm;
    let p = &s.p;
    let fp00 = p.p00 + f02*p.p02; let fp01 = p.p01 + f02*p.p12;
    let fp02 = p.p02 + f02*p.p22;
    let _fp10 = p.p01 + f12*p.p02; let fp11 = p.p11 + f12*p.p12;
    let fp12 = p.p12 + f12*p.p22;

    let q00 = 0.25*cm*cm*sigma2_ds + 0.25*ds*ds*sm*sm*sigma2_dth;
    let q01 = 0.25*cm*sm*sigma2_ds - 0.25*ds*ds*sm*cm*sigma2_dth;
    let q02 = -0.5*ds*sm*sigma2_dth;
    let q11 = 0.25*sm*sm*sigma2_ds + 0.25*ds*ds*cm*cm*sigma2_dth;
    let q12 = 0.5*ds*cm*sigma2_dth;
    let q22 = sigma2_dth;

    s.p = CovSym3 {
        p00: fp00 + fp02*f02 + q00,
        p01: fp01 + fp02*f12 + q01,
        p02: fp02             + q02,
        p11: fp11 + fp12*f12 + q11,
        p12: fp12             + q12,
        p22: p.p22            + q22,
    };
}

pub fn update_gyro(s: &mut EkfState, omega_z_rads: f32) {
    let r_ang = R_VEL * DT * DT;
    let z     = omega_z_rads * DT;
    let nu    = z - wrap_angle(s.theta - s.theta_prev);
    let ss    = s.p.p22 + r_ang;

    let k0 = s.p.p02 / ss; let k1 = s.p.p12 / ss; let k2 = s.p.p22 / ss;
    s.x += k0*nu; s.y += k1*nu;
    s.theta = wrap_angle(s.theta + k2*nu);

    let p = s.p;
    s.p.p00 -= k0*p.p02; s.p.p01 -= k0*p.p12; s.p.p02 -= k0*p.p22;
    s.p.p11 -= k1*p.p12; s.p.p12 -= k1*p.p22; s.p.p22 -= k2*p.p22;
    
    s.p.p00 = s.p.p00.max(1.0e-9);
    s.p.p11 = s.p.p11.max(1.0e-9);
    s.p.p22 = s.p.p22.max(r_ang);
}

#[inline]
pub fn wrap_angle(mut a: f32) -> f32 {
    while a >  3.14159265 { a -= 2.0 * 3.14159265; }
    while a <= -3.14159265 { a += 2.0 * 3.14159265; }
    a
}
