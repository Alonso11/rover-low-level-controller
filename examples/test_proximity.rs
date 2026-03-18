// Version: v1.0
//! # Ejemplo: Test de Sensores de Proximidad (HC-SR04 y TF-Luna)
//!
//! Este programa de ejemplo demuestra cómo utilizar simultáneamente un sensor
//! ultrasónico y un sensor LiDAR para la detección de obstáculos en el Rover.
//!
//! ## Conexiones Sugeridas (Arduino Mega 2560):
//! * **HC-SR04 (Ultrasonido):** 
//!     - Trigger -> Pin Digital D14
//!     - Echo    -> Pin Digital D15
//! * **TF-Luna (LiDAR):**
//!     - RX (del sensor) -> Pin D16 (TX2 del Arduino)
//!     - TX (del sensor) -> Pin D17 (RX2 del Arduino)
//! * **Debug Serial (USB):**
//!     - 115200 baudios vía puerto USB estándar.

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::sensors::{HCSR04, TFLuna, ProximitySensor};

#[arduino_hal::entry]
fn main() -> ! {
    // Adquisición de periféricos y pines
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Inicialización de la consola para depuración
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    ufmt::uwriteln!(&mut serial, "--- Olympus Rover: Sistema de Proximidad v1.0 ---\r").unwrap();

    // Configuración del sensor HC-SR04 (Ultrasonido)
    // Utilizamos forget_imode() para convertir el pin Echo en un tipo de entrada genérico.
    let mut hc_sr04 = HCSR04::new(
        pins.d14.into_output(),
        pins.d15.into_floating_input().forget_imode(),
    );
    ufmt::uwriteln!(&mut serial, "[INFO] HC-SR04 listo en pines D14(T)/D15(E)\r").unwrap();

    // Configuración del sensor TF-Luna (LiDAR) en el puerto Serial 2 (USART2)
    // El LiDAR requiere 115200 baudios para su comunicación por defecto.
    let serial2 = arduino_hal::Usart::new(
        dp.USART2,
        pins.d17.into_floating_input(),
        pins.d16.into_output(),
        115200.into(),
    );
    let mut tf_luna = TFLuna::new(serial2);
    ufmt::uwriteln!(&mut serial, "[INFO] TF-Luna listo en USART2 (D17/D16)\r").unwrap();

    loop {
        // Lectura del sensor ultrasónico
        match hc_sr04.get_distance_mm() {
            Some(dist) => {
                ufmt::uwrite!(&mut serial, "US: {} mm | ", dist).unwrap();
            }
            None => {
                ufmt::uwrite!(&mut serial, "US:  --  mm | ").unwrap();
            }
        }

        // Lectura del sensor LiDAR
        match tf_luna.get_distance_mm() {
            Some(dist) => {
                ufmt::uwriteln!(&mut serial, "LiDAR: {} mm\r", dist).unwrap();
            }
            None => {
                ufmt::uwriteln!(&mut serial, "LiDAR:  --  mm\r").unwrap();
            }
        }

        // Pausa breve para no saturar la consola y dar estabilidad a los sensores.
        arduino_hal::delay_ms(100);
    }
}
