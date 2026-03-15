// Version: v1.0
//! # Módulo de Sensores
//! 
//! Este módulo contiene las implementaciones para los diferentes sensores del Rover,
//! como encoders, sensores de proximidad, IMU, etc.

/// Módulo para encoders de posición (Efecto Hall, Quadrature, etc.)
pub mod encoder;
/// Módulo para el sensor ultrasónico HC-SR04
pub mod hc_sr04;
/// Módulo para el sensor LiDAR TF-Luna
pub mod tf_luna;

pub use encoder::{Encoder, HallEncoder};
pub use hc_sr04::HCSR04;
pub use tf_luna::TFLuna;

/// Interfaz común para sensores de proximidad/distancia.
pub trait ProximitySensor {
    /// Obtiene la distancia medida en milímetros (mm).
    fn get_distance_mm(&mut self) -> Option<u16>;
}
