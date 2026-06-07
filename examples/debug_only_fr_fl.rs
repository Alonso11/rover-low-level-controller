// Version: v1.0 (HW DEBUG — SOLO FR + FL, frente del rover)
//! Diagnóstico de las dos ruedas delanteras (**FR** y **FL**) en aislamiento:
//! son los únicos motores que se inicializan. Ningún otro pin se configura.
//!
//! Un solo disparo: ambas ADELANTE 30 % durante 3 s, luego paran y deshabilitan.
//! Resetear el Mega para repetir. FL va con `inverted=true` (igual que main.rs),
//! así las dos ruedas giran en el MISMO sentido de marcha del rover.
//!
//! ## Pinout (idéntico a main.rs, BTS7960 bajo all-bts7960)
//! | Motor | RPWM | LPWM | R_EN | L_EN | Timer RPWM/LPWM     | inverted |
//! |-------|------|------|------|------|---------------------|----------|
//! | FR    | D9   | D44  | D23  | D25  | Timer2 OC2B / T5 OC5C | false  |
//! | FL    | D10  | D45  | D22  | D24  | Timer2 OC2A / T5 OC5B | **true** |
//!
//! ## Qué medir durante los 3 s (multímetro / FT232H)
//! - Ambas: R_EN=L_EN=5 V. RPWM conmuta, LPWM=0 V (el sentido físico lo ajusta
//!   `inverted` en FL, pero a nivel de pines FL maneja D10/D45).
//! - En STOP: todos los PWM a 0 V, las dos ruedas quietas.
//!
//! ## Precauciones
//! - ROVER ELEVADO, ruedas FR y FL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-fr-fl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha hacia adelante (3 s)

/// Rampa de `from` a `to` aplicando set_speed en cada paso a los dos motores.
fn ramp_both<A: Motor, B: Motor>(a: &mut A, b: &mut B, from: i16, to: i16) {
    let mut v = from;
    let step = if to >= from { RAMP_STEP } else { -RAMP_STEP };
    while v != to {
        v += step;
        if (step > 0 && v > to) || (step < 0 && v < to) { v = to; }
        a.set_speed(v);
        b.set_speed(v);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_fr_fl] init - SOLO FR (D9/D44) + FL (D10/D45)");

    // Solo los timers que usan FR y FL.
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // FR (RPWM=D9, LPWM=D44, R_EN=D23, L_EN=D25).
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(),
        pins.d25.into_output(),
        false,
    );
    // FL (RPWM=D10, LPWM=D45, R_EN=D22, L_EN=D24).
    // inverted=FALSE: el cableado M+/M- actual de la FL ya NO requiere inversión
    // (cambió al recablear la tierra). Con esto FR y FL giran en el MISMO sentido.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(),
        pins.d24.into_output(),
        false,
    );

    // Margen para soltar conectores si algo está mal.
    let _ = ufmt::uwriteln!(&mut serial, "[debug_fr_fl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    fr.enable();
    fl.enable();

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> FR y FL");
    ramp_both(&mut fr, &mut fl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_both(&mut fr, &mut fl, SPEED, 0);

    // --- STOP definitivo ---
    fr.stop();  fl.stop();
    fr.disable(); fl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parados y deshabilitados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
