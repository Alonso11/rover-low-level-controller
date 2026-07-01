// Version: v1.0 (HW DEBUG — SOLO RR, HOLD largo para medir)
//! Diagnóstico de la rueda **RR** (trasero derecho) en aislamiento total:
//! es el ÚNICO motor que se inicializa. Ningún otro pin se configura.
//!
//! Un solo disparo: ADELANTE 30 % con HOLD de 15 s (tiempo para medir todos
//! los pines con multímetro), luego para y deshabilita. Resetear para repetir.
//!
//! ## Pinout RR (según main.rs, BTS7960 bajo all-bts7960)
//! | Señal | Pin  | Timer        |
//! |-------|------|--------------|
//! | RPWM  | D7   | Timer4 OC4B  |  ← canal activo (inverted=false)
//! | LPWM  | D13  | Timer1 OC1C  |
//! | R_EN  | D34  | digital out  |
//! | L_EN  | D35  | digital out  |
//!
//! ⚠️ OJO: CR/CL resultaron tener LPWM/EN cruzados respecto a main.rs. Verificar
//!    si RR/RL también lo están. Si RR no se mueve, medir el checklist.
//!
//! ## Checklist de medición durante el HOLD (RR no se mueve → buscar dónde)
//!  1. B+ del módulo RR  ......................  ¿~12 V?  (si 0 V → falta potencia/XL4015)
//!  2. R_EN (D34) y L_EN (D35)  ...............  ¿~5 V?   (si 0 V → enable suelto o pin cruzado)
//!  3. RPWM (D7)  .............................  conmuta (~1.5 V prom.)  ← canal ACTIVO
//!  4. LPWM (D13)  ............................  ~0 V
//!  5. M+ / M− del motor  .....................  M+ con tensión, M−≈0
//!     - control OK + M+ = 0 V  → BTS de la RR dañado (recordar: ACS RR estaba dañado)
//!     - M+ con tensión pero no gira → motor desconectado o dañado
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda RR en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-rr PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer4Pwm};
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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr] init - SOLO RR (RPWM=D7 LPWM=D13 R_EN=D34 L_EN=D35)");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(),
        pins.d35.into_output(),
        false,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_rr] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    rr.enable();

    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (HOLD 15s) -> MIDE: B+, R_EN=D34, L_EN=D35, RPWM=D7, LPWM=D13, M+/M-");
    ramp_to(&mut rr, 0, SPEED);
    arduino_hal::delay_ms(HOLD_MS as u32);
    ramp_to(&mut rr, SPEED, 0);

    rr.stop();
    rr.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado y deshabilitado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
