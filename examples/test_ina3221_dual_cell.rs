// Version: v1.0
//! # Ejemplo: Medición de Voltaje de Batería (INA3221)
//!
//! Este programa mide el voltaje de dos bancos de batería (aprox. 15V cada uno)
//! utilizando el sensor INA3221 conectado a través de I2C por software.
//!
//! ## Conexiones (Arduino Mega 2560):
//! - **INA3221:**
//!     - SDA -> Pin Digital D42 (PL7)
//!     - SCL -> Pin Digital D43 (PL6)
//!     - VCC -> 5V
//!     - GND -> GND común con las baterías
//!     - CH1 IN+ -> (+) Batería 1 (15V)
//!     - CH2 IN+ -> (+) Batería 2 (15V)
//!
//! ADVERTENCIA: El INA3221 tiene un límite de 26V para el voltaje del bus.
//! Asegúrate de que tus baterías no excedan este valor.

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::sensors::INA3221;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    ufmt::uwriteln!(&mut serial, "--- Test Voltaje Dual INA3221 ---\r").unwrap();

    let mut ina = INA3221::new();
    let ina_ok = ina.init();

    if !ina_ok {
        ufmt::uwriteln!(&mut serial, "[ERROR] No se pudo inicializar INA3221 en D42/D43\r").unwrap();
    } else {
        ufmt::uwriteln!(&mut serial, "[INFO] INA3221 inicializado correctamente\r").unwrap();
    }

    let mut t_ms: u32 = 0;
    let step_ms: u32 = 1000;

    ufmt::uwriteln!(&mut serial, "# t_ms,b1_mv,b2_mv\r").unwrap();

    loop {
        if ina.ready {
            let v1_mv = ina.read_bank1_mv();
            let v2_mv = ina.read_bank2_mv();

            // Formato CSV para facilitar el logueo
            ufmt::uwriteln!(
                &mut serial, 
                "{},{},{}\r", 
                t_ms, v1_mv, v2_mv
            ).unwrap();
        }

        arduino_hal::delay_ms(step_ms);
        t_ms += step_ms;
    }
}
