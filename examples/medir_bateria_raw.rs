// Version: v1.0 — RAW ADC output (sin factor hardcodeado)
//! Firmware STANDALONE para 2º Mega midiendo voltaje de batería.
//! Emite ADC crudo (0–1023). El factor lo define el Python.
//!
//! Banco 1 (lógica)  → divisor → A0
//! Banco 2 (motores) → divisor → A1
//!
//! CSV (1/s, 9600 baud):  tiempo_s,raw_b1,raw_b2
//!
//! Flash: `make flash-medir-bateria-raw PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 9600);

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let a0 = pins.a0.into_analog_input(&mut adc);
    let a1 = pins.a1.into_analog_input(&mut adc);

    let _ = ufmt::uwriteln!(&mut serial, "# medir_bateria_raw v1.0 (ADC crudo, factor en Python)");

    let mut t_s: u32 = 0;
    loop {
        let mut acc0: u32 = 0;
        let mut acc1: u32 = 0;
        for _ in 0..8 {
            acc0 += a0.analog_read(&mut adc) as u32;
            acc1 += a1.analog_read(&mut adc) as u32;
        }
        let r0 = acc0 / 8;
        let r1 = acc1 / 8;

        let _ = ufmt::uwriteln!(&mut serial, "{},{},{}", t_s, r0, r1);

        t_s += 1;
        arduino_hal::delay_ms(1000);
    }
}
