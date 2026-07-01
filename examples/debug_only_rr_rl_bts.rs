// Version: v1.0 (HW DEBUG — SOLO RR + RL, AMBOS BTS7960)
//! Diagnóstico del par trasero con **los dos como BTS7960** (RL ya NO usa el
//! L298N sustituto: el módulo BTS de RL resultó sano en el bringup 2026-05-31).
//! Son los únicos motores que se inicializan.
//!
//! Un solo disparo: ambas ADELANTE 30 % durante 3 s, luego paran. Resetear para repetir.
//!
//! ## Pinout (ambos BTS7960)
//! | Motor | RPWM | LPWM | R_EN | L_EN | Timer RPWM/LPWM       | inverted |
//! |-------|------|------|------|------|-----------------------|----------|
//! | RR    | D7   | D13  | D34  | D35  | T4 OC4B / T1 OC1C     | false    |
//! | RL    | D8   | D4   | D36  | D37  | T4 OC4C / T0 OC0B     | true     |
//!
//! ⚠️ RL como BTS usa D4 (LPWM) además de D8/D36/D37. Conectar el módulo BTS RL.
//!
//! ## Precauciones
//! - ROVER ELEVADO, ruedas RR y RL en el aire. 12 V (B+) presente.
//! - NO alimentar sensores desde el Mega que carguen el riel (causó sag antes).
//!
//! Flash: `make flash-debug-only-rr-rl-bts PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer0Pwm, Timer1Pwm, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha hacia adelante (3 s)

/// Rampa de `from` a `to` aplicando set_speed a los dos motores en cada paso.
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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr_rl_bts] init - RR BTS (D7/D13/D34/D35) + RL BTS (D8/D4/D36/D37)");

    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // RR — BTS7960: RPWM=D7(T4), LPWM=D13(T1), R_EN=D34, L_EN=D35.
    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(),
        pins.d35.into_output(),
        false,
    );
    // RL — BTS7960 (módulo sano): RPWM=D8(T4), LPWM=D4(T0), R_EN=D36, L_EN=D37, inverted.
    let mut rl = BTS7960Motor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d4.into_output().into_pwm(&mut t0),
        pins.d36.into_output(),
        pins.d37.into_output(),
        true,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr_rl_bts] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    rr.enable();
    rl.enable();

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> RR y RL (ambos BTS)");
    ramp_both(&mut rr, &mut rl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_both(&mut rr, &mut rl, SPEED, 0);

    // --- STOP definitivo ---
    rr.stop();  rl.stop();
    rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parados y deshabilitados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
