// Version: v1.2
//! # Módulo de Control de Motores
//!
//! Traits `Motor` / `Servo`, `SixWheelRover` y `ErasedMotor` son lógica pura
//! (sin dependencias de `arduino-hal`) y están disponibles también en x86
//! para tests unitarios.
//!
//! Los submodules de drivers (`l298n`, `bts7960`, `servo`) sí dependen del HAL
//! y solo compilan con `feature = "avr"`.

/// Interfaz común para cualquier controlador de motor DC.
/// Permite que la lógica de alto nivel sea independiente del driver concreto.
#[allow(dead_code)]
pub trait Motor {
    /// Establece la velocidad del motor.
    /// `speed`: −100 (retroceso total) … 0 (parado) … 100 (avance total).
    fn set_speed(&mut self, speed: i16);

    /// Detiene el motor inmediatamente.
    fn stop(&mut self);
}

/// Interfaz para servomotores (control de posición angular).
pub trait Servo {
    /// Establece el ángulo del servo (0–180 grados).
    fn set_angle(&mut self, angle: u8);
}

// ─── Drivers de hardware — solo con feature "avr" ────────────────────────────

/// Driver para el puente-H L298N.
#[cfg(feature = "avr")]
pub mod l298n;

/// Driver de alta potencia BTS7960 (IBT-2).
#[cfg(feature = "avr")]
pub mod bts7960;

/// Servomotor estándar por software.
#[cfg(feature = "avr")]
pub mod servo;

// ─── Lógica pura — siempre disponible ────────────────────────────────────────

/// Type-erased motor wrapper para arrays homogéneos sin heap.
pub mod erased;
pub use erased::ErasedMotor;

// ─── SixWheelRover ───────────────────────────────────────────────────────────

/// Chasis diferencial de 6 ruedas. Genérico sobre cualquier implementación de `Motor`.
///
/// El orden de motores es: FR (frontal derecho), FL (frontal izquierdo),
/// CR (central derecho), CL (central izquierdo), RR (trasero derecho), RL (trasero izquierdo).
#[allow(dead_code)]
pub struct SixWheelRover<M1, M2, M3, M4, M5, M6> {
    pub frontal_right: M1,
    pub frontal_left:  M2,
    pub center_right:  M3,
    pub center_left:   M4,
    pub rear_right:    M5,
    pub rear_left:     M6,
}

impl<M1, M2, M3, M4, M5, M6> SixWheelRover<M1, M2, M3, M4, M5, M6>
where
    M1: Motor, M2: Motor, M3: Motor, M4: Motor, M5: Motor, M6: Motor,
{
    /// Construye el rover con los 6 motores ya configurados.
    pub fn new(fr: M1, fl: M2, cr: M3, cl: M4, rr: M5, rl: M6) -> Self {
        Self {
            frontal_right: fr, frontal_left: fl,
            center_right:  cr, center_left:  cl,
            rear_right:    rr, rear_left:    rl,
        }
    }

    /// Control diferencial (tanque): aplica `left_speed` a los 3 motores izquierdos
    /// y `right_speed` a los 3 derechos.
    pub fn set_speeds(&mut self, left_speed: i16, right_speed: i16) {
        self.frontal_left.set_speed(left_speed);
        self.center_left.set_speed(left_speed);
        self.rear_left.set_speed(left_speed);
        self.frontal_right.set_speed(right_speed);
        self.center_right.set_speed(right_speed);
        self.rear_right.set_speed(right_speed);
    }

    /// Detiene los 6 motores simultáneamente.
    pub fn stop(&mut self) {
        self.frontal_left.stop();
        self.center_left.stop();
        self.rear_left.stop();
        self.frontal_right.stop();
        self.center_right.stop();
        self.rear_right.stop();
    }
}
