// Version: v3.1 (BANCO DE MEDICIÓN — DOS bancos, SOLO Mega + 2 divisores)
//! Firmware STANDALONE para un 2º Arduino Mega dedicado a medir el voltaje de
//! LAS DOS baterías (autonomía RNF-007). Usa DOS ADC del Mega con un divisor por
//! banco. SIN INA, SIN I2C. Emite CSV por Serial → capturar_bateria.py.
//!
//! Banco 1 (lógica)  → divisor → A0
//! Banco 2 (motores) → divisor → A1
//!
//! Divisor por banco (un POTENCIÓMETRO de 10k cada uno):
//!     V_bat(+) ── extremo 1 ─┐
//!                     (wiper) ┼──► A0 (Banco 1) / A1 (Banco 2)
//!     GND ──────── extremo 2 ─┘
//!   Factor = (R1+R2)/R2 = 3.7 (4S 16.8 V → ~4.5 V en el ADC, seguro).
//!   Calibración: aplica 12 V conocidos a cada divisor y gira su pot hasta que
//!   el serial marque 12.0 V en esa columna.
//!
//! ⚠️ NINGÚN ADC debe pasar de 5 V → la batería SIEMPRE por el divisor.
//!
//! CSV (1/s, 9600 baud):  tiempo_s,banco1_V,banco2_V
//!
//! Flash: `make flash-medir-bateria PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;

// Divisor R1 (arriba, a V_bat) / R2 (abajo, a GND). Factor = (R1+R2)/R2 = 3.7.
const R1_OHM:  u32 = 27000;
const R2_OHM:  u32 = 10000;
const VREF_MV: u32 = 5000;
const ADC_MAX: u32 = 1023;

/// Lee un pin ADC (promedio de 8) y lo convierte a mV de batería vía el divisor.
macro_rules! read_vbat_mv {
    ($pin:expr, $adc:expr) => {{
        let mut acc: u32 = 0;
        for _ in 0..8 {
            acc += $pin.analog_read(&mut $adc) as u32;
        }
        let raw = acc / 8;
        let v_adc_mv = raw * VREF_MV / ADC_MAX;
        v_adc_mv * (R1_OHM + R2_OHM) / R2_OHM
    }};
}

/// Emite un valor en mV como "X.XXX" (voltios con 3 decimales, sin float).
macro_rules! emit_v {
    ($s:expr, $mv:expr) => {{
        let mv: u32 = $mv;
        let _ = ufmt::uwrite!($s, "{}.", mv / 1000);
        let f = mv % 1000;
        if f < 100 { let _ = ufmt::uwrite!($s, "0"); }
        if f < 10  { let _ = ufmt::uwrite!($s, "0"); }
        let _ = ufmt::uwrite!($s, "{}", f);
    }};
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 9600);

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let a0 = pins.a0.into_analog_input(&mut adc); // Banco 1
    let a1 = pins.a1.into_analog_input(&mut adc); // Banco 2

    let _ = ufmt::uwriteln!(&mut serial, "# medir_bateria v3.1 (solo Mega, 2 divisores A0/A1, factor 3.7)");
    let _ = ufmt::uwriteln!(&mut serial, "tiempo_s,banco1_V,banco2_V");

    let mut t_s: u32 = 0;
    loop {
        let v1_mv = read_vbat_mv!(a0, adc); // Banco 1
        let v2_mv = read_vbat_mv!(a1, adc); // Banco 2

        let _ = ufmt::uwrite!(&mut serial, "{},", t_s);
        emit_v!(&mut serial, v1_mv);
        let _ = ufmt::uwrite!(&mut serial, ",");
        emit_v!(&mut serial, v2_mv);
        let _ = ufmt::uwriteln!(&mut serial, "");

        t_s += 1;
        arduino_hal::delay_ms(1000);
    }
}
