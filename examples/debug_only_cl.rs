// Version: v1.1 (HW DEBUG — SOLO CL, pinout REAL corregido)
//! Diagnóstico de la rueda **CL** (central izquierdo) en aislamiento total:
//! es el ÚNICO motor que se inicializa. Ningún otro pin se configura.
//!
//! Un solo disparo: ADELANTE 30 % durante 3 s, luego para y deshabilita.
//! Resetear el Mega para repetir.
//!
//! ## Pinout CL — CABLEADO REAL (CR↔CL intercambiados salvo RPWM D5/D6)
//! | Señal | Pin  | Timer        |
//! |-------|------|--------------|
//! | RPWM  | D6   | Timer4 OC4A  |  ← canal activo (inverted=false)
//! | LPWM  | D11  | Timer1 OC1A  |
//! | R_EN  | D28  | digital out  |
//! | L_EN  | D29  | digital out  |
//!
//! NOTA: main.rs tiene CL con inverted=true; si en la integración con CR
//!       gira al revés, lo cambiamos (igual que pasó con la FL).
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda CL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-cl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha hacia adelante (3 s)

fn ramp_to<M: Motor>(m: &mut M, from: i16, to: i16) {
    let mut v = from;
    let step = if to >= from { RAMP_STEP } else { -RAMP_STEP };
    while v != to {
        v += step;
        if (step > 0 && v > to) || (step < 0 && v < to) { v = to; }
        m.set_speed(v);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_cl] init - SOLO CL (RPWM=D6 LPWM=D11 R_EN=D28 L_EN=D29) [pinout real]");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // CABLEADO REAL: RPWM=D6(T4), LPWM=D11(T1 OC1A), R_EN=D28, L_EN=D29.
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(),
        pins.d29.into_output(),
        false,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_cl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    cl.enable();

    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (3s) -> RPWM=D6 conmuta, LPWM=D11=0");
    ramp_to(&mut cl, 0, SPEED);
    arduino_hal::delay_ms(RUN_MS as u32);
    ramp_to(&mut cl, SPEED, 0);

    cl.stop();
    cl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado y deshabilitado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
