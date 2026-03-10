//! Módulo de control de motores
//! Proporciona una interfaz común (Trait Motor) y diferentes implementaciones de hardware.

/// Interfaz común para cualquier controlador de motor DC.
#[allow(dead_code)]
pub trait Motor {
    /// Establece la velocidad del motor.
    /// speed: Rango de -100 (retroceso total) a 100 (avance total). 0 para detener.
    fn set_speed(&mut self, speed: i16);
    
    /// Detiene el motor inmediatamente.
    fn stop(&mut self);
}

/// Interfaz para servomotores (control de posición).
pub trait Servo {
    /// Establece el ángulo del servo (0 a 180 grados).
    fn set_angle(&mut self, angle: u8);
}

/// Implementación para el driver Puente-H L298N
pub mod l298n;

/// Implementación para el driver de alta potencia BTS7960
pub mod bts7960;

/// Implementación para servomotores estándar
pub mod servo;
