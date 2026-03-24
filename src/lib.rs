// Version: v1.3
#![no_std]
#![cfg_attr(target_arch = "avr", feature(abi_avr_interrupt))]

// nb re-exportado para que los ejemplos AVR lo usen directamente.
#[cfg(feature = "avr")]
pub use nb;

// Módulos que dependen del HAL de Arduino — solo disponibles con feature "avr".
#[cfg(feature = "avr")]
pub mod motor_control;
#[cfg(feature = "avr")]
pub mod command_interface;
#[cfg(feature = "avr")]
pub mod sensors;
#[cfg(feature = "avr")]
pub mod controller;

// Módulo de lógica pura (sin HAL) — siempre disponible, testeable en x86.
pub mod state_machine;
