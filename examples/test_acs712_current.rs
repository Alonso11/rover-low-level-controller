// Version: v1.0
//! # Sensor de corriente ACS712-30A — lectura y reporte por serial
//!
//! ## Conexión
//! | ACS712 | Arduino Mega |
//! |--------|--------------|
//! | VCC    | 5V           |
//! | GND    | GND          |
//! | OUT    | A0 (PC0)     |
//!
//! ## Especificaciones ACS712ELCTR-30A
//! - Rango:       ±30 A
//! - Sensibilidad: 66 mV/A
//! - V_out@0A:    VCC/2 = 2.5 V (≈ ADC 512 / 1023 con AVCC=5V)
//! - ADC:         10 bits → 5000 mV / 1023 ≈ 4.887 mV por cuenta
//!
//! ## Fórmula
//! ```
//! V_mv        = (adc * 5000) / 1023
//! delta_mv    = V_mv - 2500          // desviación respecto al cero
//! corriente_mA = (delta_mv * 1000) / 66
//! ```
//!
//! ## Cómo leer la salida
//! Conectar con `cargo run --example test_acs712_current` o cualquier monitor
//! serie a 115200 baud.  Positivo = corriente entrando por IP+; negativo = saliente.
//!
//! ## Calibración del offset
//! Con el canal desconectado (0 A) anotar el ADC crudo que imprime el programa
//! y ajustar `ZERO_MV` si difiere de 2500.

#![no_std]
#![no_main]

use panic_halt as _;

/// Voltaje de offset a corriente cero en mV.
/// Teóricamente VCC/2 = 2500 mV; ajustar con calibración real si es necesario.
const ZERO_MV: i32 = 2500;

/// Sensibilidad del ACS712-30A: 66 mV por amperio.
const SENSITIVITY_MV_PER_A: i32 = 66;

/// Número de muestras para el promedio (reduce ruido ADC).
const NUM_SAMPLES: u8 = 16;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let a0 = pins.a0.into_analog_input(&mut adc);

    ufmt::uwriteln!(&mut serial, "--- ACS712-30A en A0 | 115200 baud ---\r").unwrap();
    ufmt::uwriteln!(&mut serial, "ZERO_MV={} mV  SENS={} mV/A  MUESTRAS={}\r",
        ZERO_MV, SENSITIVITY_MV_PER_A, NUM_SAMPLES).unwrap();

    loop {
        // Promedio de NUM_SAMPLES lecturas para reducir ruido ADC.
        let mut sum: u32 = 0;
        for _ in 0..NUM_SAMPLES {
            sum += a0.analog_read(&mut adc) as u32;
            arduino_hal::delay_us(100);
        }
        let adc_avg = (sum / NUM_SAMPLES as u32) as u16;

        // Conversión ADC → voltaje (mV), sin punto flotante.
        // 5000 mV de fondo de escala, 1023 cuentas máximas.
        let v_mv = (adc_avg as u32 * 5000) / 1023;

        // Desviación respecto al punto de cero (puede ser negativa).
        let delta_mv = v_mv as i32 - ZERO_MV;

        // Corriente en mA: delta / sensibilidad.
        // Negativo = corriente en sentido contrario (salida IP+→IP-).
        let current_ma = (delta_mv * 1000) / SENSITIVITY_MV_PER_A;

        // Signo explícito para ufmt (no soporta {:+} ni {:02}).
        let sign: u8 = if current_ma < 0 { b'-' } else { b'+' };
        let abs_ma = if current_ma < 0 { -current_ma } else { current_ma } as u32;
        let abs_a      = abs_ma / 1000;
        let abs_frac   = (abs_ma % 1000) / 10; // 0-99

        // Padding manual del decimal (ufmt no soporta {:02}).
        if abs_frac < 10 {
            ufmt::uwriteln!(&mut serial,
                "ADC={} V={}mV delta={}mV I={}{}.0{}A ({}mA)\r",
                adc_avg, v_mv, delta_mv, sign as char, abs_a, abs_frac, current_ma
            ).unwrap();
        } else {
            ufmt::uwriteln!(&mut serial,
                "ADC={} V={}mV delta={}mV I={}{}.{}A ({}mA)\r",
                adc_avg, v_mv, delta_mv, sign as char, abs_a, abs_frac, current_ma
            ).unwrap();
        }

        arduino_hal::delay_ms(500);
    }
}
