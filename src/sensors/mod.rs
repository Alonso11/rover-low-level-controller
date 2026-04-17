// Version: v1.3
//! # Módulo de Sensores
//!
//! Este módulo contiene las implementaciones para los diferentes sensores del Rover,
//! como encoders, sensores de proximidad, IMU, etc.

// ── Drivers puro Rust (sin HAL) — siempre disponibles, testeables en x86 ────
/// Módulo para el sensor de corriente ACS712-30A
pub mod acs712;
/// Módulo para el sensor de temperatura LM335
pub mod lm335;
/// Módulo para el módulo termistor NTC AD36958 (B=3950, 10kΩ)
pub mod ntc_thermistor;
/// Driver TF02 LiDAR UART largo alcance (Benewake DELiDAR TF02)
pub mod tf02;

pub use acs712::ACS712;
pub use lm335::LM335;
pub use ntc_thermistor::NTCThermistor;
pub use tf02::TF02;

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
/// I2C por software (bit-bang) en D42/D43 — evita conflicto con TWI hardware
#[cfg(feature = "avr")]
pub mod soft_i2c;
/// Driver para el sensor Time-of-Flight VL53L0X (GY-VL53L0XV2) vía soft I2C
#[cfg(feature = "avr")]
pub mod vl53l0x;
/// Driver para el monitor de potencia INA226 vía soft I2C (bus compartido con VL53L0X)
#[cfg(feature = "avr")]
pub mod ina226;
/// Driver para el sensor inercial MPU-6050 vía soft I2C
#[cfg(feature = "avr")]
pub mod mpu6050;

#[cfg(feature = "avr")]
pub use encoder::{Encoder, HallEncoder};
#[cfg(feature = "avr")]
pub use hc_sr04::HCSR04;
#[cfg(feature = "avr")]
pub use tf_luna::TFLuna;
#[cfg(feature = "avr")]
pub use vl53l0x::VL53L0X;
#[cfg(feature = "avr")]
pub use ina226::INA226;
#[cfg(feature = "avr")]
pub use mpu6050::MPU6050;

/// Error de lectura de un sensor de proximidad.
#[cfg(feature = "avr")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensorError {
    /// No hay medición nueva disponible — no fatal, reintentar en el próximo ciclo.
    NotReady,
    /// El sensor no respondió en el tiempo esperado.
    Timeout,
    /// La medición está fuera del rango de detección válido del sensor.
    OutOfRange,
    /// Los datos recibidos no superaron la verificación de integridad.
    ChecksumError,
}

/// Interfaz común para sensores de proximidad/distancia.
#[cfg(feature = "avr")]
pub trait ProximitySensor {
    fn get_distance_mm(&mut self) -> Result<u16, SensorError>;
}

// ─── Validación de plausibilidad ─────────────────────────────────────────────

/// Valida que una lectura de temperatura esté dentro de un rango físicamente
/// plausible. Retorna `Some(t)` si es válida, `None` si probablemente indica
/// sensor desconectado o averiado (equivalente a `sanitize_float` del STM32).
///
/// # Casos de fallo detectados
/// - `LM335` con ADC=0 (pin flotante a GND): `read_celsius(0)` → -273 °C → `None`
/// - `NTCThermistor` con ADC alto (~1023, pin flotante): `read_celsius(1023)` → -20 °C
///   Si `min_c = -20` exactamente, el límite es inclusivo y pasa. Usar -19 para excluirlo.
///
/// # Ejemplo
/// ```
/// use rover_low_level_controller::sensors::check_temp_c;
/// assert_eq!(check_temp_c(25, -40, 80), Some(25));
/// assert_eq!(check_temp_c(-273, -40, 80), None);
/// ```
pub fn check_temp_c(t: i32, min_c: i32, max_c: i32) -> Option<i32> {
    if t >= min_c && t <= max_c { Some(t) } else { None }
}
