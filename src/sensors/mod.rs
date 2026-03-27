// Version: v1.0
//! # Módulo de Sensores
//! 
//! Este módulo contiene las implementaciones para los diferentes sensores del Rover,
//! como encoders, sensores de proximidad, IMU, etc.

// ── Drivers puro Rust (sin HAL) — siempre disponibles, testeables en x86 ────
/// Módulo para el sensor de corriente ACS712-30A
pub mod acs712;
/// Módulo para el sensor de temperatura LM335
pub mod lm335;

pub use acs712::ACS712;
pub use lm335::LM335;

// ── Drivers con dependencia de arduino-hal — solo con feature "avr" ──────────
/// Módulo para encoders de posición (Efecto Hall)
#[cfg(feature = "avr")]
pub mod encoder;
/// Módulo para el sensor ultrasónico HC-SR04
#[cfg(feature = "avr")]
pub mod hc_sr04;
/// Módulo para el sensor LiDAR TF-Luna
#[cfg(feature = "avr")]
pub mod tf_luna;

#[cfg(feature = "avr")]
pub use encoder::{Encoder, HallEncoder};
#[cfg(feature = "avr")]
pub use hc_sr04::HCSR04;
#[cfg(feature = "avr")]
pub use tf_luna::TFLuna;

/// Interfaz común para sensores de proximidad/distancia.
#[cfg(feature = "avr")]
pub trait ProximitySensor {
    fn get_distance_mm(&mut self) -> Option<u16>;
}
