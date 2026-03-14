// Version: v1.0
//! # Módulo de Control de Motores
//! 
//! Este módulo proporciona la abstracción necesaria para controlar diferentes tipos de motores
//! (DC con L298N, BTS7960 y Servomotores) de forma unificada mediante Traits.

/// Interfaz común para cualquier controlador de motor DC.
/// Permite que la lógica de alto nivel (como el control del Rover) sea independiente
/// del hardware específico del driver.
#[allow(dead_code)]
pub trait Motor {
    /// Establece la velocidad del motor.
    /// 
    /// # Parámetros
    /// * `speed`: Rango de -100 (retroceso total) a 100 (avance total). 
    ///   Un valor de 0 detiene el motor.
    fn set_speed(&mut self, speed: i16);
    
    /// Detiene el motor inmediatamente poniendo todas las señales a nivel bajo.
    fn stop(&mut self);
}

/// Interfaz para servomotores (control de posición).
pub trait Servo {
    /// Establece el ángulo del servo.
    /// 
    /// # Parámetros
    /// * `angle`: Ángulo en grados (típicamente de 0 a 180).
    fn set_angle(&mut self, angle: u8);
}

/// Implementación para el driver Puente-H L298N (común en robots pequeños/medianos).
pub mod l298n;

/// Implementación para el driver de alta potencia BTS7960 (IBT-2).
pub mod bts7960;

/// Implementación para servomotores estándar mediante control por software.
pub mod servo;
