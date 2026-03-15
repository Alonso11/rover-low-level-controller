// Version: v1.0
//! # Driver para el sensor LiDAR de corto rango TF-Luna.
//!
//! Este driver permite la comunicación serie con el sensor LiDAR TF-Luna de Benewake.
//! El sensor envía paquetes de 9 bytes de forma continua con datos de distancia,
//! intensidad de señal y temperatura.

use arduino_hal::hal::usart::Usart;
use arduino_hal::prelude::*;
use crate::sensors::ProximitySensor;

/// Estructura para el sensor LiDAR TF-Luna.
/// 
/// Encapsula un puerto UART (USART) del ATmega2560 para la lectura de datos.
pub struct TFLuna<USART, RX, TX, CLOCK>
where
    USART: arduino_hal::hal::usart::UsartOps<arduino_hal::hal::Atmega, RX, TX>,
{
    /// Periférico serie utilizado para recibir los frames del sensor.
    serial: Usart<USART, RX, TX, CLOCK>,
}

impl<USART, RX, TX, CLOCK> TFLuna<USART, RX, TX, CLOCK>
where
    USART: arduino_hal::hal::usart::UsartOps<arduino_hal::hal::Atmega, RX, TX>,
    CLOCK: arduino_hal::hal::clock::Clock,
{
    /// Crea una nueva instancia del sensor TF-Luna.
    ///
    /// # Parámetros
    /// * `serial`: Una instancia de USART configurada a 115200 baudios.
    pub fn new(serial: Usart<USART, RX, TX, CLOCK>) -> Self {
        Self { serial }
    }

    /// Intenta leer un paquete de datos completo del sensor.
    /// 
    /// Formato del frame (9 bytes):
    /// [0x59, 0x59, Dist_L, Dist_H, Strength_L, Strength_H, Temp_L, Temp_H, Checksum]
    /// 
    /// Retorna la distancia en milímetros (mm).
    pub fn read_packet(&mut self) -> Option<u16> {
        let mut header_count = 0;
        let mut timeout = 0;

        // Fase 1: Sincronización con la cabecera del frame (0x59 0x59).
        while header_count < 2 {
            if let Ok(byte) = self.serial.read() {
                if byte == 0x59 {
                    header_count += 1;
                } else {
                    header_count = 0;
                }
            }
            timeout += 1;
            if timeout > 2000 { return None; }
        }

        // Fase 2: Lectura de los datos restantes (7 bytes).
        let mut data = [0u8; 7];
        let mut sum: u16 = 0x59 + 0x59; // La suma del checksum incluye la cabecera.

        for i in 0..7 {
            let mut sub_timeout = 0;
            loop {
                if let Ok(byte) = self.serial.read() {
                    data[i] = byte;
                    // Sumamos los primeros 8 bytes para validar el checksum posterior.
                    if i < 6 { sum += byte as u16; }
                    break;
                }
                sub_timeout += 1;
                if sub_timeout > 1000 { return None; }
            }
        }

        // Fase 3: Validación del Checksum (Byte 9).
        let checksum = data[6];
        if (sum & 0xFF) as u8 != checksum {
            return None; // Frame corrupto o error de transmisión.
        }

        // Fase 4: Interpretación de la distancia (Bytes 2 y 3).
        // El valor viene en centímetros (cm).
        let dist_cm = (data[1] as u16) << 8 | (data[0] as u16);
        
        // Convertimos a milímetros para consistencia con el Trait ProximitySensor.
        Some(dist_cm * 10)
    }
}

impl<USART, RX, TX, CLOCK> ProximitySensor for TFLuna<USART, RX, TX, CLOCK>
where
    USART: arduino_hal::hal::usart::UsartOps<arduino_hal::hal::Atmega, RX, TX>,
    CLOCK: arduino_hal::hal::clock::Clock,
{
    /// Obtiene la distancia actual del sensor LiDAR en mm.
    fn get_distance_mm(&mut self) -> Option<u16> {
        self.read_packet()
    }
}
