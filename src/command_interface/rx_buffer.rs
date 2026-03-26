// Version: v1.0
//! # Ring Buffer para recepción USART por interrupción
//!
//! Soluciona el desbordamiento del FIFO hardware USART (3 bytes en ATmega2560)
//! cuando el loop principal usa `delay_ms()` u otras operaciones bloqueantes.
//!
//! ## Modelo productor/consumidor
//!
//! ```text
//!  ISR USART_RX  ──push()──►  RxRingBuffer  ──pop()──►  poll_from_ring()
//!  (interrupción)              [64 bytes]                (loop principal)
//! ```
//!
//! ## Invariantes de seguridad (AVR single-core)
//!
//! - `push()` solo se llama desde la ISR USART_RX: interrupciones globales
//!   deshabilitadas implícitamente durante la ISR → acceso exclusivo a `head`.
//! - `pop()` solo se llama desde el loop principal → acceso exclusivo a `tail`.
//! - Lecturas cruzadas (`head` en main, `tail` en ISR) son de u8 → atómicas en AVR.
//! - No se necesita Mutex porque el AVR es single-core y no hay preempción en main.
//!
//! En caso de overflow (buffer lleno), `push()` descarta el byte silenciosamente.
//! Con 64 bytes de capacidad y comandos de ≤12 bytes, el overflow no ocurre
//! en condiciones normales incluso con `delay_ms(20)`.

use core::cell::UnsafeCell;

/// Capacidad del ring buffer en bytes.
/// Debe ser potencia de 2 para permitir optimización con máscara de bits.
const RX_BUF_SIZE: usize = 64;
const RX_BUF_MASK: usize = RX_BUF_SIZE - 1;

/// Ring buffer lock-free para recepción USART via interrupción.
pub struct RxRingBuffer {
    buf:  UnsafeCell<[u8; RX_BUF_SIZE]>,
    /// Índice de escritura — modificado solo por la ISR.
    head: UnsafeCell<u8>,
    /// Índice de lectura — modificado solo por el loop principal.
    tail: UnsafeCell<u8>,
}

// Safety: single-core AVR. Ver invariantes de módulo.
unsafe impl Sync for RxRingBuffer {}

impl RxRingBuffer {
    /// Crea una instancia vacía. Usar como `static`.
    pub const fn new() -> Self {
        Self {
            buf:  UnsafeCell::new([0; RX_BUF_SIZE]),
            head: UnsafeCell::new(0),
            tail: UnsafeCell::new(0),
        }
    }

    /// Escribe un byte en el buffer.
    ///
    /// # Safety
    /// Llamar únicamente desde la ISR USART_RX. Las interrupciones globales
    /// están implícitamente deshabilitadas durante una ISR en AVR, garantizando
    /// acceso exclusivo a `head`.
    pub unsafe fn push(&self, byte: u8) {
        let head = *self.head.get() as usize;
        let next = (head + 1) & RX_BUF_MASK;
        // Descartar si el buffer está lleno (next == tail)
        if next != *self.tail.get() as usize {
            (*self.buf.get())[head] = byte;
            *self.head.get() = next as u8;
        }
    }

    /// Lee el siguiente byte disponible.
    ///
    /// Retorna `None` si el buffer está vacío.
    /// Llamar únicamente desde el loop principal.
    pub fn pop(&self) -> Option<u8> {
        // Safety: head es u8 → lectura atómica en AVR.
        let head = unsafe { *self.head.get() } as usize;
        let tail = unsafe { *self.tail.get() } as usize;
        if tail == head {
            return None;
        }
        let byte = unsafe { (*self.buf.get())[tail] };
        unsafe { *self.tail.get() = ((tail + 1) & RX_BUF_MASK) as u8; }
        Some(byte)
    }
}
