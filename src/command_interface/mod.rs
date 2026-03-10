//! Interfaz de comandos por puerto Serie
//! Se encarga de recibir, almacenar y procesar paquetes de datos desde la Raspberry Pi 5.

use arduino_hal::hal::port::Dynamic;
use arduino_hal::hal::port::mode::Input;
use arduino_hal::hal::port::Pin;
use arduino_hal::pac::USART0;
use arduino_hal::port::mode::Input as HalInput;

pub struct SerialInterface {
    serial: arduino_hal::Usart<USART0, Pin<HalInput, Dynamic>, Pin<arduino_hal::port::mode::Output, Dynamic>>,
}

impl SerialInterface {
    /// Crea una nueva interfaz serial a 115200 baudios (estándar para RPi5)
    pub fn new(serial: arduino_hal::Usart<USART0, Pin<HalInput, Dynamic>, Pin<arduino_hal::port::mode::Output, Dynamic>>) -> Self {
        Self { serial }
    }

    /// Lee un byte del puerto serie si está disponible.
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.serial.void_read().is_ok() {
            Some(self.serial.read())
        } else {
            None
        }
    }

    /// Envía un mensaje de texto de vuelta a la Raspberry Pi.
    pub fn send_str(&mut self, message: &str) {
        for b in message.as_bytes() {
            self.serial.write(*b);
        }
    }
}
