// Version: v1.0 (HW DEBUG — FR+FL+CR+CL, frente + centro, pinout REAL)
//! Diagnóstico de las cuatro ruedas delanteras y centrales juntas. Solo estos
//! cuatro motores se inicializan (RR y RL no se tocan).
//!
//! Un solo disparo: las 4 ADELANTE 30 % durante 3 s, luego paran y deshabilitan.
//! Resetear el Mega para repetir.
//!
//! ## Pinout — CABLEADO REAL (ya con las correcciones del bringup)
//! | Motor | RPWM | LPWM | R_EN | L_EN | Timer RPWM/LPWM       | inverted |
//! |-------|------|------|------|------|-----------------------|----------|
//! | FR    | D9   | D44  | D23  | D25  | Timer2 OC2B / T5 OC5C | true     |
//! | FL    | D10  | D45  | D22  | D24  | Timer2 OC2A / T5 OC5B | true     |
//! | CR    | D5   | D12  | D30  | D31  | Timer3 OC3A / T1 OC1B | false    | ← referencia (LPWM/EN cambiados)
//! | CL    | D6   | D11  | D28  | D29  | Timer4 OC4A / T1 OC1A | true     | ← (LPWM/EN cambiados)
//!
//! Todas deben girar en el MISMO sentido. Si alguna va al revés, ajustar su
//! `inverted`.
//!
//! ## Precauciones
//! - ROVER ELEVADO, las 4 ruedas en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-front-center PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler, Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha hacia adelante (3 s)

/// Rampa de `from` a `to` aplicando set_speed a los cuatro motores en cada paso.
fn ramp_all<A: Motor, B: Motor, C: Motor, D: Motor>(
    a: &mut A, b: &mut B, c: &mut C, d: &mut D, from: i16, to: i16,
) {
    let mut v = from;
    let step = if to >= from { RAMP_STEP } else { -RAMP_STEP };
    while v != to {
        v += step;
        if (step > 0 && v > to) || (step < 0 && v < to) { v = to; }
        a.set_speed(v);
        b.set_speed(v);
        c.set_speed(v);
        d.set_speed(v);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_fc] init - FR+FL+CR+CL (pinout real)");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // FR — RPWM=D9(T2), LPWM=D44(T5), R_EN=D23, L_EN=D25. inverted=true.
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), true,
    );
    // FL — RPWM=D10(T2), LPWM=D45(T5), R_EN=D22, L_EN=D24. inverted=true.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );
    // CR — CABLEADO REAL: RPWM=D5(T3), LPWM=D12(T1 OC1B), R_EN=D30, L_EN=D31. inverted=false (referencia).
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), false,
    );
    // CL — CABLEADO REAL: RPWM=D6(T4), LPWM=D11(T1 OC1A), R_EN=D28, L_EN=D29. inverted=true.
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), true,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_fc] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    fr.enable(); fl.enable();
    cr.enable(); cl.enable();

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> FR FL CR CL (mismo sentido)");
    ramp_all(&mut fr, &mut fl, &mut cr, &mut cl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_all(&mut fr, &mut fl, &mut cr, &mut cl, SPEED, 0);

    // --- STOP definitivo ---
    fr.stop();  fl.stop();  cr.stop();  cl.stop();
    fr.disable(); fl.disable(); cr.disable(); cl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parados y deshabilitados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
