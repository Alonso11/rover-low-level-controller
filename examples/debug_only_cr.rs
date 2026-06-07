// Version: v1.2 (HW DEBUG — SOLO CR, HOLD largo, pinout REAL corregido)
//! Diagnóstico de la rueda **CR** (central derecho) en aislamiento total:
//! es el ÚNICO motor que se inicializa. Ningún otro pin se configura.
//!
//! Un solo disparo: ADELANTE 30 % con HOLD de 15 s (tiempo para medir todos
//! los pines con multímetro), luego para y deshabilita. Resetear para repetir.
//!
//! ## Pinout CR — CABLEADO REAL (CR↔CL intercambiados salvo RPWM D5/D6)
//! | Señal | Pin  | Timer        |
//! |-------|------|--------------|
//! | RPWM  | D5   | Timer3 OC3A  |  ← canal activo (inverted=false)
//! | LPWM  | D12  | Timer1 OC1B  |
//! | R_EN  | D30  | digital out  |
//! | L_EN  | D31  | digital out  |
//!
//! ## Checklist de medición durante el HOLD (CR no se mueve → buscar dónde)
//!  1. B+ del módulo CR  ......................  ¿~12 V?  (si 0 V → falta potencia/XL4015)
//!  2. R_EN (D30) y L_EN (D31)  ...............  ¿~5 V?   (si 0 V → cable enable suelto)
//!  3. RPWM (D5)  .............................  conmuta (~1.5 V prom.)  ← canal ACTIVO
//!  4. LPWM (D12)  ............................  ~0 V
//!  5. M+ / M− del motor  .....................  M+ con tensión, M−≈0
//!     - control OK + M+ = 0 V  → BTS de la CR dañado (como el RL quemado)
//!     - M+ con tensión pero no gira → motor desconectado o dañado
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda CR en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-cr PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer3Pwm};
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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_cr] init - SOLO CR (RPWM=D5 LPWM=D12 R_EN=D30 L_EN=D31) [pinout real]");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);

    // CABLEADO REAL: RPWM=D5(T3), LPWM=D12(T1 OC1B), R_EN=D30, L_EN=D31.
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(),
        pins.d31.into_output(),
        false,
    );

    let _ = ufmt::uwriteln!(&mut serial, "[debug_cr] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    cr.enable();

    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (HOLD 15s) -> MIDE: B+, R_EN=D30, L_EN=D31, RPWM=D5, LPWM=D12, M+/M-");
    ramp_to(&mut cr, 0, SPEED);
    arduino_hal::delay_ms(HOLD_MS as u32);
    ramp_to(&mut cr, SPEED, 0);

    cr.stop();
    cr.disable();
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado y deshabilitado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
