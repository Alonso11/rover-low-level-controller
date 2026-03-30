// Version: v1.3
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

// Módulo de lógica pura (sin HAL) — siempre disponible, testeable en x86.
pub mod state_machine;
