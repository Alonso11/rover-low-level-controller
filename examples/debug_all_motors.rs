// Version: v1.0 (HW DEBUG — LOS 6 MOTORES, pinout + inverts REALES del bringup)
//! Integración final: los 6 motores juntos con el cableado e inversiones reales
//! determinados durante el bringup (rueda por rueda).
//!
//! Un solo disparo: los 6 ADELANTE 30 % durante 3 s, luego paran y deshabilitan.
//! Resetear el Mega para repetir. SIN MSM, SIN watchdog, SIN brake_all.
//!
//! ## Pinout + inversión — CABLEADO REAL (bringup 2026-05-31)
//! | Motor | Driver | RPWM/ENA | LPWM/IN1 | R_EN/IN2 | L_EN | Timer        | inverted |
//! |-------|--------|----------|----------|----------|------|--------------|----------|
//! | FR    | BTS7960| D9       | D44      | D23      | D25  | T2 OC2B/T5 OC5C | true   |
//! | FL    | BTS7960| D10      | D45      | D22      | D24  | T2 OC2A/T5 OC5B | true   |
//! | CR    | BTS7960| D5       | D12      | D30      | D31  | T3 OC3A/T1 OC1B | false  | ← ref (LPWM/EN cambiados)
//! | CL    | BTS7960| D6       | D11      | D28      | D29  | T4 OC4A/T1 OC1A | true   | ← (LPWM/EN cambiados)
//! | RR    | BTS7960| D7       | D13      | D34      | D35  | T4 OC4B/T1 OC1C | false  |
//! | RL    | L298N  | D8 (ENA) | D36(IN1) | D37(IN2) | —    | T4 OC4C      | false    |
//!
//! Todas deben girar en el MISMO sentido (rover hacia adelante). Si alguna va
//! al revés, ajustar su `inverted`.
//!
//! ## Precauciones
//! - ROVER ELEVADO, las 6 ruedas en el aire. 12 V (B+) presente.
//! - NO alimentar sensores desde el Mega que carguen el riel durante la prueba.
//!
//! Flash: `make flash-debug-all PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler, Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha hacia adelante (3 s)

/// Rampa de `from` a `to` aplicando set_speed a los seis motores en cada paso.
#[allow(clippy::too_many_arguments)]
fn ramp6<A: Motor, B: Motor, C: Motor, D: Motor, E: Motor, F: Motor>(
    a: &mut A, b: &mut B, c: &mut C, d: &mut D, e: &mut E, f: &mut F, from: i16, to: i16,
) {
    let mut v = from;
    let step = if to >= from { RAMP_STEP } else { -RAMP_STEP };
    while v != to {
        v += step;
        if (step > 0 && v > to) || (step < 0 && v < to) { v = to; }
        a.set_speed(v); b.set_speed(v); c.set_speed(v);
        d.set_speed(v); e.set_speed(v); f.set_speed(v);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_all] init - 6 motores (pinout+inverts reales)");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // FR — BTS: RPWM=D9(T2), LPWM=D44(T5), R_EN=D23, L_EN=D25, inverted=true.
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), true,
    );
    // FL — BTS: RPWM=D10(T2), LPWM=D45(T5), R_EN=D22, L_EN=D24, inverted=true.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );
    // CR — BTS (cableado real): RPWM=D5(T3), LPWM=D12(T1), R_EN=D30, L_EN=D31, inverted=false (ref).
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), false,
    );
    // CL — BTS (cableado real): RPWM=D6(T4), LPWM=D11(T1), R_EN=D28, L_EN=D29, inverted=true.
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), true,
    );
    // RR — BTS: RPWM=D7(T4), LPWM=D13(T1), R_EN=D34, L_EN=D35, inverted=false.
    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );
    // RL — L298N: ENA=D8(T4), IN1=D36, IN2=D37, inverted=false.
    let mut rl = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d36.into_output(), pins.d37.into_output(), false,
    );
    rl.stop();

    let _ = ufmt::uwriteln!(&mut serial, "[debug_all] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    fr.enable(); fl.enable(); cr.enable();
    cl.enable(); rr.enable(); rl.enable(); // rl.enable() es no-op (L298N)

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> FR FL CR CL RR RL (mismo sentido)");
    ramp6(&mut fr, &mut fl, &mut cr, &mut cl, &mut rr, &mut rl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp6(&mut fr, &mut fl, &mut cr, &mut cl, &mut rr, &mut rl, SPEED, 0);

    // --- STOP definitivo ---
    fr.stop(); fl.stop(); cr.stop(); cl.stop(); rr.stop(); rl.stop();
    fr.disable(); fl.disable(); cr.disable(); cl.disable(); rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> los 6 parados y deshabilitados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
