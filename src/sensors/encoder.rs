// Version: v1.0
//! # Módulo de Encoders
//!
//! Este módulo proporciona la abstracción y los drivers para la lectura de encoders
//! de efecto Hall (magnéticos) para medir la posición de los motores.

use avr_device::interrupt::Mutex;
use core::cell::Cell;

/// Interfaz común para cualquier tipo de encoder.
pub trait Encoder {
    /// Obtiene el número total de pulsos contados.
    fn get_counts(&self) -> i32;
    
    /// Reinicia el contador de pulsos a cero.
    fn reset(&self);
}

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
        avr_device::interrupt::free(|cs| {
            self.counts.borrow(cs).get()
        })
    }

    fn reset(&self) {
        avr_device::interrupt::free(|cs| {
            self.counts.borrow(cs).set(0);
        });
    }
}
