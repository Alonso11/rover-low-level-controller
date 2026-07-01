// Version: v1.0 (HW DEBUG — SOLO RL via BTS7960, HOLD largo para medir)
//! Diagnóstico de la rueda **RL** (trasero izquierdo) usando el módulo
//! **BTS7960** (el que se creyó quemado), NO el L298N sustituto. Sirve para
//! confirmar si ese BTS realmente está dañado o estaba bien.
//!
//! Es el ÚNICO motor que se inicializa. Un solo disparo: ADELANTE 30 % con
//! HOLD de 15 s, luego para y deshabilita. Resetear para repetir.
//!
//! ## Pinout RL como BTS7960 (según main.rs sin rl-l298n)
//! | Señal | Pin  | Timer        |
//! |-------|------|--------------|
//! | RPWM  | D8   | Timer4 OC4C  |  ← canal activo (inverted=false)
//! | LPWM  | D4   | Timer0 OC0B  |
//! | R_EN  | D36  | digital out  |
//! | L_EN  | D37  | digital out  |
//!
//! ⚠️ CABLEADO: en modo BTS, D36/D37 son R_EN/L_EN (NO IN1/IN2 del L298N) y
//!    D4 es LPWM. Conectar el módulo BTS7960 RL a D8/D4/D36/D37 antes de probar.
//!
//! ## Checklist durante el HOLD (si no se mueve, buscar dónde)
//!  1. B+ del módulo BTS RL  ..................  ¿~12 V?  (si 0 V → potencia/XL4015)
//!  2. R_EN (D36) y L_EN (D37)  ...............  ¿~5 V?   (si 0 V → enable suelto)
//!  3. RPWM (D8)  .............................  conmuta (~1.5 V prom.)  ← canal ACTIVO
//!  4. LPWM (D4)  .............................  ~0 V
//!  5. M+ / M−  ...............................  M+ con tensión, M−≈0
//!     - control OK + M+ = 0 V  → BTS RL realmente dañado
//!     - todo OK y gira         → el BTS estaba sano; el problema era otro
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda RL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-rl-bts PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer0Pwm, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;    // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;    // rampa ~600 ms
const HOLD_MS: u16      = 15000; // HOLD largo para alcanzar a medir

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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_rl_bts] init - SOLO RL via BTS7960 (RPWM=D8 LPWM=D4 R_EN=D36 L_EN=D37)");

    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    let mut rl = BTS7960Motor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d4.into_output().into_pwm(&mut t0),
        pins.d36.into_output(),
        pins.d37.into_output(),
        true, // invertido
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_rl_bts] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    rl.enable(); // R_EN=L_EN=HIGH

    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (HOLD 15s) -> MIDE: B+, R_EN=D36, L_EN=D37, RPWM=D8, LPWM=D4, M+/M-");
    ramp_to(&mut rl, 0, SPEED);
    arduino_hal::delay_ms(HOLD_MS as u32);
    ramp_to(&mut rl, SPEED, 0);

    rl.stop();
    rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado y deshabilitado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
