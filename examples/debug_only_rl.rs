// Version: v1.0 (HW DEBUG — SOLO RL via L298N, HOLD largo para medir)
//! Diagnóstico de la rueda **RL** (trasero izquierdo) en aislamiento total.
//! RL es el sustituto con **L298N** (el BTS7960 original se quemó). Es el ÚNICO
//! motor que se inicializa; ningún otro pin se configura.
//!
//! Un solo disparo: ADELANTE 30 % con HOLD de 15 s (tiempo para medir), luego
//! para. Resetear el Mega para repetir.
//!
//! ## Pinout RL (L298N, según main.rs)
//! | Señal | Pin  | Detalle             |
//! |-------|------|---------------------|
//! | ENA   | D8   | PWM velocidad (Timer4 OC4C) |
//! | IN1   | D36  | dirección           |
//! | IN2   | D37  | dirección           |
//!
//! Nota: el L298N NO tiene enable tipo BTS — `enable()` es no-op. En reposo
//! `stop()` deja IN1=IN2=0 y duty=0. Adelante: IN1=5 V, IN2=0 V, ENA conmuta.
//!
//! ## Checklist de medición durante el HOLD (RL no se mueve → buscar dónde)
//!  1. B+ del módulo L298N RL  ................  ¿~12 V?  (si 0 V → potencia/XL4015)
//!  2. Jumper ENA del módulo  .................  DEBE estar QUITADO (usamos D8 para PWM)
//!  3. ENA (D8)  ..............................  conmuta (~1.5 V prom.)  ← velocidad
//!  4. IN1 (D36) / IN2 (D37)  .................  IN1≈5 V, IN2≈0 V (adelante)
//!  5. M+ / M−  ...............................  M+ con tensión, M−≈0
//!     - control OK + M+ = 0 V  → L298N dañado
//!     - M+ con tensión pero no gira → motor desconectado o dañado
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda RL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-rl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_rl] init - SOLO RL via L298N (ENA=D8 IN1=D36 IN2=D37)");

    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    let mut rl = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d36.into_output(),
        pins.d37.into_output(),
        false,
    );
    rl.stop(); // L298N no arranca deshabilitado: forzar parado.

    let _ = ufmt::uwriteln!(&mut serial, "[debug_rl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    rl.enable(); // no-op para L298N

    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (HOLD 15s) -> MIDE: B+, ENA=D8, IN1=D36, IN2=D37, M+/M-");
    ramp_to(&mut rl, 0, SPEED);
    arduino_hal::delay_ms(HOLD_MS as u32);
    ramp_to(&mut rl, SPEED, 0);

    rl.stop();
    rl.disable(); // no-op para L298N (stop ya dejó IN1=IN2=0, duty=0)
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
