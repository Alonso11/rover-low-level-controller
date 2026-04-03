// Version: v1.1
//! Interfaz de comandos por puerto Serie
//! Gestiona la comunicación protocolizada con la Raspberry Pi 5.

pub mod rx_buffer;
pub use rx_buffer::RxRingBuffer;

use arduino_hal::hal::usart::UsartOps;
use arduino_hal::prelude::*;

/// Capacidad máxima del buffer de comandos (en bytes)
const BUFFER_SIZE: usize = 32;

pub struct CommandInterface<USART, RX, TX> 
where 
    USART: UsartOps<arduino_hal::hal::Atmega, RX, TX>,
{
    serial: arduino_hal::Usart<USART, RX, TX>,
    buffer: [u8; BUFFER_SIZE],
    index: usize,
}

impl<USART, RX, TX> CommandInterface<USART, RX, TX> 
where 
    USART: UsartOps<arduino_hal::hal::Atmega, RX, TX>,
{
    /// Crea una nueva interfaz sobre cualquier puerto USART compatible.
    pub fn new(serial: arduino_hal::Usart<USART, RX, TX>) -> Self {
        Self {
            serial,
            buffer: [0; BUFFER_SIZE],
            index: 0,
        }
    }

    /// Intenta leer un comando. Devuelve true si se recibió un comando completo (\n).
    pub fn poll_command(&mut self) -> bool {
        while let Ok(byte) = self.serial.read() {
            if byte == b'\n' || byte == b'\r' {
                if self.index > 0 {
                    return true;
                }
            } else if self.index < BUFFER_SIZE - 1 {
                self.buffer[self.index] = byte;
                self.index += 1;
            }
        }
        false
    }

    /// Obtiene el comando actual como texto y resetea el buffer.
    pub fn get_command(&mut self) -> &[u8] {
        let len = self.index;
        self.index = 0;
        &self.buffer[..len]
    }

    /// Envía un mensaje de log a la RPi 5 (añade \n automáticamente).
    pub fn log(&mut self, msg: &str) {
        for b in msg.as_bytes() {
            let _ = nb::block!(self.serial.write(*b));
        }
        let _ = nb::block!(self.serial.write(b'\n'));
    }

    /// Envía un frame de respuesta pre-formateado (ya incluye \n).
    /// Usar con `format_response()` del módulo `state_machine`.
    pub fn send_response(&mut self, bytes: &[u8]) {
        for &b in bytes {
            let _ = nb::block!(self.serial.write(b));
        }
    }

    /// Versión interrupt-driven de `poll_command`.
    ///
    /// Lee bytes del `RxRingBuffer` (llenado por la ISR USART_RX) en lugar
    /// de leer directamente del FIFO hardware. Permite que el loop principal
    /// use `delay_ms()` sin perder bytes: la ISR los almacena en el buffer
    /// de 64 bytes mientras el CPU está ocupado.
    ///
    /// Retorna `true` cuando se recibe un comando completo (`\n` o `\r`).
    /// El comando queda accesible via `get_command()`.
    pub fn poll_from_ring(&mut self, rx: &RxRingBuffer) -> bool {
        while let Some(byte) = rx.pop() {
            if byte == b'\n' || byte == b'\r' {
                if self.index > 0 {
                    return true;
                }
            } else if self.index < BUFFER_SIZE - 1 {
                self.buffer[self.index] = byte;
                self.index += 1;
            }
        }
        false
    }
}
