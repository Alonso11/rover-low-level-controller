//! Modulo de control de motores
//! Sigue principios SOLID para permitir diferentes drivers de hardware.

mod h_bridge;
pub use h_bridge::HBridge;

/// Define las capacidades basicas de cualquier motor del rover.
pub trait Motor {
    /// Establece la velocidad y direccion.
    /// - `speed`: de -255 a 255 (negativo es reversa, 0 es stop).
    fn set_speed(&mut self, speed: i16);

    /// Detiene el motor inmediatamente.
    fn stop(&mut self);
}

/// Representa el sistema de traccion del rover (ej. Diferencial).
pub struct DriveTrain<L: Motor, R: Motor> {
    pub left: L,
    pub right: R,
}

impl<L: Motor, R: Motor> DriveTrain<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }

    /// Mueve el rover (tanque/diferencial)
    pub fn drive(&mut self, left_speed: i16, right_speed: i16) {
        self.left.set_speed(left_speed);
        self.right.set_speed(right_speed);
    }

    pub fn stop(&mut self) {
        self.left.stop();
        self.right.stop();
    }
}
