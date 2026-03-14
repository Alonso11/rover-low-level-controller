// Version: v1.0
//! # Módulo de Sensores
//! 
//! Este módulo contiene las implementaciones para los diferentes sensores del Rover,
//! como encoders, sensores de proximidad, IMU, etc.

/// Módulo para encoders de posición (Efecto Hall, Quadrature, etc.)
pub mod encoder;

pub use encoder::{Encoder, HallEncoder};
