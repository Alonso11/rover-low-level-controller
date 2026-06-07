// Version: v1.0 (HW DEBUG — SOLO CR + CL, centro del rover, pinout REAL)
//! Diagnóstico del par central (**CR** y **CL**) en aislamiento: son los únicos
//! motores que se inicializan. Ningún otro pin se configura.
//!
//! Un solo disparo: ambas ADELANTE 30 % durante 3 s, luego paran y deshabilitan.
//! Resetear el Mega para repetir.
//!
//! ## Pinout — CABLEADO REAL (CR↔CL intercambiados salvo RPWM D5/D6)
//! | Motor | RPWM | LPWM | R_EN | L_EN | Timer RPWM/LPWM       | inverted |
//! |-------|------|------|------|------|-----------------------|----------|
//! | CR    | D5   | D12  | D30  | D31  | Timer3 OC3A / T1 OC1B | false    |
//! | CL    | D6   | D11  | D28  | D29  | Timer4 OC4A / T1 OC1A | false    |
//!
//! Si CL gira al revés que CR, cambiar su `inverted` a true (como pasó con la FL).
//!
//! ## Precauciones
//! - ROVER ELEVADO, ruedas CR y CL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-cr-cl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer3Pwm, Timer4Pwm};
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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_cr_cl] init - SOLO CR (D5/D12/D30/D31) + CL (D6/D11/D28/D29)");

    // Timers que usan CR y CL: Timer1 (LPWM ambos), Timer3 (RPWM CR), Timer4 (RPWM CL).
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // CR — CABLEADO REAL: RPWM=D5(T3), LPWM=D12(T1 OC1B), R_EN=D30, L_EN=D31.
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(),
        pins.d31.into_output(),
        false,
    );
    // CL — CABLEADO REAL: RPWM=D6(T4), LPWM=D11(T1 OC1A), R_EN=D28, L_EN=D29.
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(),
        pins.d29.into_output(),
        false,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_cr_cl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    cr.enable();
    cl.enable();

    // --- ADELANTE 3 s ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> CR y CL");
    ramp_both(&mut cr, &mut cl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_both(&mut cr, &mut cl, SPEED, 0);

    // --- STOP definitivo ---
    cr.stop();  cl.stop();
    cr.disable(); cl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parados y deshabilitados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
