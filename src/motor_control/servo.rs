//! Driver para servomotores estándar (0-180 grados)
//! Basado en pulsos precisos de microsegundos (50Hz).

use arduino_hal::hal::port::{Pin, PinOps};
use arduino_hal::hal::port::mode::Output;
use crate::motor_control::Servo;

/// Implementación de un servo motor usando control por software (Precisión de µs).
pub struct StandardServo<PIN> {
    pin: Pin<Output, PIN>,
}

impl<PIN> StandardServo<PIN>
where
    PIN: PinOps,
{
    /// Crea un nuevo servo en un pin digital estándar.
    pub fn new(pin: Pin<Output, PIN>) -> Self {
        Self { pin }
    }

    /// Envía un único pulso al servo. 
    /// Esta función debe llamarse frecuentemente (aprox. cada 20ms) 
    /// para mantener la posición del servo.
    pub fn pulse(&mut self, microseconds: u16) {
        self.pin.set_high();
        arduino_hal::delay_us(microseconds as u32);
        self.pin.set_low();
    }
}

impl<PIN> Servo for StandardServo<PIN>
where
    PIN: PinOps,
{
    fn set_angle(&mut self, mut angle: u8) {
        if angle > 180 {
            angle = 180;
        }

        // Mapeo estándar: 
        // 0°   -> 1000 µs
        // 90°  -> 1500 µs
        // 180° -> 2000 µs
        let pulse_width = 1000 + (u32::from(angle) * 1000 / 180) as u16;
        
        // Generamos el pulso
        self.pulse(pulse_width);
    }
}
