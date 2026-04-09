// Version: v1.4
#![no_std]
#![cfg_attr(target_arch = "avr", feature(abi_avr_interrupt))]

// nb re-exportado para que los ejemplos AVR lo usen directamente.
#[cfg(feature = "avr")]
pub use nb;

// Módulos que dependen del HAL de Arduino — solo disponibles con feature "avr".
// motor_control expone Motor/Servo/ErasedMotor/SixWheelRover sin gate (lógica pura);
// los submodules de drivers (l298n, bts7960, servo) llevan su propio gate interno.
pub mod motor_control;
#[cfg(feature = "avr")]
pub mod command_interface;
#[cfg(feature = "avr")]
pub mod controller;

// Módulo de sensores: los drivers analógicos (ACS712, LM335) son puro Rust y
// siempre disponibles. Los drivers con HAL (HC-SR04, encoders, TF-Luna) solo
// compilan con feature "avr".
pub mod sensors;
/// Filtro de Kalman Extendido para fusión sensorial
pub mod ekf;

// Módulo de lógica pura (sin HAL) — siempre disponible, testeable en x86.
pub mod state_machine;

// Rampa de velocidad para soft-stop/soft-start — lógica pura, sin HAL.
pub mod ramp;

// Constantes de configuración — sin dependencias de HAL, siempre disponibles.
pub mod config;
