// Version: v1.0 (HW DEBUG — SOLO RR + RL, parte trasera)
//! Diagnóstico del par trasero juntos: **RR** (BTS7960) y **RL** (L298N
//! sustituto). Son los únicos motores que se inicializan; ningún otro pin se toca.
//!
//! Un solo disparo: ambas ADELANTE 30 % durante 3 s, luego paran. Resetear para repetir.
//!
//! ## Pinout (según main.rs)
//! | Motor | Driver | RPWM/ENA | LPWM/IN1 | R_EN/IN2 | L_EN | Timer        | inverted |
//! |-------|--------|----------|----------|----------|------|--------------|----------|
//! | RR    | BTS7960| D7       | D13      | D34(R_EN)| D35  | T4 OC4B/T1 OC1C | false  |
//! | RL    | L298N  | D8 (ENA) | D36(IN1) | D37(IN2) | —    | T4 OC4C      | false    |
//!
//! Si uno gira al revés que el otro, cambiar su `inverted` a true.
//!
//! ## Precauciones
//! - ROVER ELEVADO, ruedas RR y RL en el aire. 12 V (B+) presente.
//! - El L298N (RL) es más débil que el BTS — sin carga pesada.
//!
//! Flash: `make flash-debug-only-rr-rl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr_rl] init - RR BTS (D7/D13/D34/D35) + RL L298N (D8/D36/D37)");

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
    // RL — L298N: ENA=D8(T4), IN1=D36, IN2=D37.
    let mut rl = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d36.into_output(),
        pins.d37.into_output(),
        false,
    );
    rl.stop();

    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr_rl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    rr.enable();
    rl.enable(); // no-op para L298N

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> RR y RL");
    ramp_both(&mut rr, &mut rl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_both(&mut rr, &mut rl, SPEED, 0);

    // --- STOP definitivo ---
    rr.stop();  rl.stop();
    rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
