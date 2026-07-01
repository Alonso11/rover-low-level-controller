// Version: v1.1
//! # Módulo de Encoders
//!
//! `HallEncoder`      — canal único, solo cuenta pulsos (sin dirección).
//! `QuadratureEncoder`— canales A+B, decodifica dirección automáticamente.
//!
//! El trait `Encoder` vive en `sensors::mod` para ser testeable en x86.

use avr_device::interrupt::Mutex;
use core::cell::Cell;
use crate::sensors::Encoder;

/// Implementación de un encoder de efecto Hall simple.
/// Utiliza un Mutex de AVR para ser seguro entre el hilo principal e interrupciones.
pub struct HallEncoder {
    counts: Mutex<Cell<i32>>,
}

impl HallEncoder {
    /// Crea una nueva instancia de encoder con el contador en cero.
    pub const fn new() -> Self {
        Self {
            counts: Mutex::new(Cell::new(0)),
        }
    }

    /// Método diseñado para ser llamado desde una Rutina de Servicio de Interrupción (ISR).
    /// Incrementa el contador en 1.
    pub fn pulse(&self) {
        avr_device::interrupt::free(|cs| {
            let cell = self.counts.borrow(cs);
            cell.set(cell.get() + 1);
        });
    }
    
    /// Incrementa o decrementa según la dirección.
    pub fn update(&self, forward: bool) {
        avr_device::interrupt::free(|cs| {
            let cell = self.counts.borrow(cs);
            if forward {
                cell.set(cell.get() + 1);
            } else {
                cell.set(cell.get() - 1);
            }
        });
    }
}

impl Encoder for HallEncoder {
    fn get_counts(&self) -> i32 {
        avr_device::interrupt::free(|cs| self.counts.borrow(cs).get())
    }
    fn reset(&self) {
        avr_device::interrupt::free(|cs| self.counts.borrow(cs).set(0));
    }
}

// ─── QuadratureEncoder ───────────────────────────────────────────────────────

/// Encoder incremental de cuadratura (canales A + B) para motores NFP-5840-31ZY-EN.
///
/// Llama a `on_edge(a_high, b_high)` desde la ISR cada vez que el canal A
/// cambia de estado (flanco de subida O bajada).
///
/// Decodificación x2 (ambos flancos de A):
///   forward = a_high XOR b_high
///   - Subida A + B=0 → adelante  (A sube antes que B en rotación CW)
///   - Subida A + B=1 → atrás
///   - Bajada A + B=1 → adelante
///   - Bajada A + B=0 → atrás
pub struct QuadratureEncoder {
    counts: Mutex<Cell<i32>>,
}

impl QuadratureEncoder {
    pub const fn new() -> Self {
        Self { counts: Mutex::new(Cell::new(0)) }
    }

    /// Llama desde la ISR en cualquier flanco del canal A.
    /// Pasa el estado actual de A y B en ese instante.
    pub fn on_edge(&self, a_high: bool, b_high: bool) {
        avr_device::interrupt::free(|cs| {
            let cell = self.counts.borrow(cs);
            if a_high ^ b_high {
                cell.set(cell.get() + 1);
            } else {
                cell.set(cell.get() - 1);
            }
        });
    }
}

impl Encoder for QuadratureEncoder {
    fn get_counts(&self) -> i32 {
        avr_device::interrupt::free(|cs| self.counts.borrow(cs).get())
    }
    fn reset(&self) {
        avr_device::interrupt::free(|cs| self.counts.borrow(cs).set(0));
    }
}
