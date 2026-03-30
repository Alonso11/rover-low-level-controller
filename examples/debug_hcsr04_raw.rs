// Version: v1.1
//! # Diagnóstico HC-SR04 — lectura cruda para sensores inestables
//!
//! Este ejemplo permite verificar si el sensor HC-SR04 responde físicamente.
//! Se han ajustado los tiempos (Trigger 20us) para sensores de baja calidad
//! que presentan fallos intermitentes de "timeout (no sube)".
//!
//! ### Diagnóstico de Hardware:
//! - **ECHO timeout (no sube):** El sensor ignoró el disparo o no tiene energía.
//! - **ECHO timeout (no baja):** El eco se perdió o el objeto está fuera de rango (>4m).
//! - **LED D13:** Parpadea en cada ciclo de medición para confirmar que el código no está bloqueado.
//!
//! Pines por defecto: Trigger D40, Echo D41.

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut led = pins.d13.into_output();
    let mut trig = pins.d40.into_output();
    let echo = pins.d41.into_floating_input();

    ufmt::uwriteln!(&mut serial, "--- HC-SR04 raw diag D40(T)/D41(E) + LED D13 --- \r").unwrap();

    loop {
        led.set_high();
        // Pulso trigger: LOW 10us, HIGH 20us, LOW
        trig.set_low();
        arduino_hal::delay_us(10);
        trig.set_high();
        arduino_hal::delay_us(20);
        trig.set_low();

        // Espera a que echo suba (timeout ~20000 iteraciones ≈ 5ms)
        let mut wait = 0u32;
        let echo_started = loop {
            if echo.is_high() { break true; }
            wait += 1;
            if wait > 20000 { break false; }
        };

        if !echo_started {
            ufmt::uwriteln!(&mut serial, "ECHO timeout (no sube)\r").unwrap();
            led.set_low();
            arduino_hal::delay_ms(200);
            continue;
        }

        // Mide cuánto tiempo permanece en alto (timeout 30ms)
        let mut duration: u32 = 0;
        let echo_ok = loop {
            if echo.is_low() { break true; }
            duration += 1;
            arduino_hal::delay_us(1);
            if duration > 30000 { break false; }
        };

        if !echo_ok {
            ufmt::uwriteln!(&mut serial, "ECHO timeout (no baja) dur>30000us\r").unwrap();
        } else {
            let mm = (duration * 1715) / 10000;
            ufmt::uwriteln!(&mut serial, "ECHO {}us -> {} mm\r", duration, mm).unwrap();
        }

        led.set_low();
        arduino_hal::delay_ms(500);
    }
}
