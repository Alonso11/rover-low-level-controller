// Version: v1.0
//! # Sensor de temperatura LM335 — lectura y reporte por serial
//!
//! ## Circuito
//! El LM335 es un zener de 2 terminales; necesita corriente de polarización.
//!
//! ```
//!  5V ──┬── R (2 kΩ) ──┬── A1 (ADC)
//!       │               │
//!       │            LM335 (+)
//!       │            LM335 (-)
//!       └───────────────┴── GND
//! ```
//!
//! R ≈ 2 kΩ fija la corriente de bias en ~1 mA a 25 °C (salida ≈ 2.98 V).
//! Rango válido de corriente: 400 µA – 5 mA.
//!
//! ## Especificaciones LM335
//! - Salida:       10 mV / K  (milivoltios por kelvin)
//! - V @ 0 °C:    2731 mV  (273.15 K × 10 mV/K)
//! - V @ 25 °C:   2981 mV  (298.15 K × 10 mV/K)
//! - V @ 100 °C:  3731 mV
//! - ADC:          10 bits, AVCC = 5 V → 4.887 mV/count
//!
//! ## Fórmula
//! ```
//! V_mv     = (adc * 5000) / 1023
//! T_kelvin = V_mv / 10
//! T_celsius = T_kelvin - 273
//! ```
//!
//! ## Calibración
//! Si la lectura difiere de un termómetro de referencia, ajustar `OFFSET_K`.
//! Ejemplo: si el sensor marca 28 °C y la referencia es 25 °C → OFFSET_K = -3.

#![no_std]
#![no_main]

use panic_halt as _;

/// Offset de calibración en kelvin (se suma al resultado final).
/// Ajustar si hay error sistemático respecto a una referencia conocida.
const OFFSET_K: i32 = 0;

/// Número de muestras para el promedio (reduce ruido ADC).
const NUM_SAMPLES: u8 = 16;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let a1 = pins.a1.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "--- LM335 en A1 | 10mV/K | AVCC=5V ---\r").unwrap();

    loop {
        // Promedio de NUM_SAMPLES lecturas.
        let mut sum: u32 = 0;
        for _ in 0..NUM_SAMPLES {
            sum += a1.analog_read(&mut adc) as u32;
            arduino_hal::delay_us(100);
        }
        let adc_avg = (sum / NUM_SAMPLES as u32) as u16;

        // ADC → voltaje en mV (5000 mV fondo de escala, 1023 cuentas).
        let v_mv = (adc_avg as u32 * 5000) / 1023;

        // Conversión a temperatura.
        let t_kelvin = (v_mv / 10) as i32 + OFFSET_K;
        let t_celsius = t_kelvin - 273;

        // Decimal: (v_mv % 10) da la décima de kelvin (≈ 0.1 °C).
        let frac = (v_mv % 10) as u8;

        ufmt::uwriteln!(&mut serial,
            "ADC={} V={}mV T={}K  T={}.{}C\r",
            adc_avg, v_mv, t_kelvin, t_celsius, frac
        ).unwrap();

        arduino_hal::delay_ms(1000);
    }
}
