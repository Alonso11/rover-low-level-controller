// Version: v1.4
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

    /// Detiene el motor inmediatamente (coast / rueda libre).
    fn stop(&mut self);

    /// Freno activo. Drivers sin freno hardware (ej. L298N) delegan a `stop()`.
    /// El BTS7960 cortocircuita los terminales del motor para máximo frenado.
    fn brake(&mut self) { self.stop(); }

    /// Habilita el driver. No-op para drivers siempre activos (ej. L298N).
    /// Debe llamarse una vez tras `new()` en drivers que arrancan deshabilitados (BTS7960).
    fn enable(&mut self) {}

    /// Deshabilita el driver (Hi-Z). No-op para drivers siempre activos.
    /// Llama a `stop()` internamente antes de deshabilitar para evitar back-EMF.
    fn disable(&mut self) {}
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

/// Motor con encoder asociado — mezcla cualquier `Motor` + cualquier `Encoder`.
pub mod motor_with_encoder;
pub use motor_with_encoder::MotorWithEncoder;

use crate::config::MIN_MOTOR_SPEED;

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
    ///
    /// Aplica dead-band: velocidades con |speed| < MIN_MOTOR_SPEED se convierten a 0
    /// para evitar energizar el motor sin que supere la fricción estática.
    pub fn set_speeds(&mut self, left_speed: i16, right_speed: i16) {
        let l = if left_speed.abs()  < MIN_MOTOR_SPEED { 0 } else { left_speed };
        let r = if right_speed.abs() < MIN_MOTOR_SPEED { 0 } else { right_speed };
        self.frontal_left.set_speed(l);
        self.center_left.set_speed(l);
        self.rear_left.set_speed(l);
        self.frontal_right.set_speed(r);
        self.center_right.set_speed(r);
        self.rear_right.set_speed(r);
    }

    /// Detiene los 6 motores simultáneamente (coast).
    pub fn stop(&mut self) {
        self.frontal_left.stop();
        self.center_left.stop();
        self.rear_left.stop();
        self.frontal_right.stop();
        self.center_right.stop();
        self.rear_right.stop();
    }

    /// Freno activo en los 6 motores. En L298N equivale a `stop()`; en BTS7960
    /// cortocircuita los terminales para máximo frenado (útil en modo CLB).
    pub fn brake_all(&mut self) {
        self.frontal_left.brake();
        self.center_left.brake();
        self.rear_left.brake();
        self.frontal_right.brake();
        self.center_right.brake();
        self.rear_right.brake();
    }

    /// Habilita los 6 drivers. Necesario tras `new()` para BTS7960; no-op para L298N.
    pub fn enable_all(&mut self) {
        self.frontal_left.enable();
        self.center_left.enable();
        self.rear_left.enable();
        self.frontal_right.enable();
        self.center_right.enable();
        self.rear_right.enable();
    }

    /// Deshabilita los 6 drivers (R_EN/L_EN → LOW, salidas en Hi-Z). No-op para L298N.
    ///
    /// De-energiza por completo los puentes-H: a diferencia de `stop()`/`brake_all()`
    /// (que dejan el BTS7960 con EN alto e IN bajos = freno activo permanente), esto
    /// corta la habilitación del chip. Es el estado de reposo SEGURO probado en el
    /// debug `debug_all_motors_bts` (termina con `disable()`), donde ninguna rueda
    /// gira sola. Usar en estados sin tracción (Fault/Safe/Standby).
    pub fn disable_all(&mut self) {
        self.frontal_left.disable();
        self.center_left.disable();
        self.rear_left.disable();
        self.frontal_right.disable();
        self.center_right.disable();
        self.rear_right.disable();
    }
}
