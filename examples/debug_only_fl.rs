// Version: v1.0 (HW DEBUG — SOLO FL, HOLD largo para medir)
//! Diagnóstico de la rueda **FL** (frontal izquierdo) en aislamiento total:
//! es el ÚNICO motor que se inicializa. Ningún otro pin se configura.
//!
//! Un solo disparo: ADELANTE 30 % con HOLD de 15 s (tiempo para medir todos
//! los pines con multímetro), luego para y deshabilita. Resetear para repetir.
//! FL va con `inverted=true` (igual que main.rs); a nivel de pines maneja
//! D10/D45 — la inversión solo cambia el sentido físico de giro.
//!
//! ## Pinout FL (idéntico a main.rs, BTS7960 bajo all-bts7960)
//! | Señal | Pin  | Timer        |
//! |-------|------|--------------|
//! | RPWM  | D10  | Timer2 OC2A  |
//! | LPWM  | D45  | Timer5 OC5B  |
//! | R_EN  | D22  | digital out  |
//! | L_EN  | D24  | digital out  |
//!
//! ## Checklist de medición durante el HOLD (FL no se mueve → buscar dónde)
//!  OJO: FL es `inverted=true`. Con velocidad +adelante, el PWM sale por LPWM
//!       (D45), NO por RPWM (D10). Por eso D10 = 0 V es lo ESPERADO aquí.
//!  1. B+ del módulo BTS de la FL  ............  ¿~12 V?  (si 0 V → falta potencia/XL4015)
//!  2. R_EN (D22) y L_EN (D24)  ...............  ¿~5 V?   (si 0 V → cable enable suelto)
//!  3. LPWM (D45)  ............................  conmuta (~1.5 V prom.)  ← canal ACTIVO (inv.)
//!  4. RPWM (D10)  ............................  ~0 V (correcto, no es falla)
//!  5. M+ / M− del motor  .....................  M−/M+ con tensión según sentido
//!     - control OK (D45 conmuta) + salida = 0 V  → BTS de la FL dañado (como el RL quemado)
//!     - salida con tensión pero no gira → motor desconectado o dañado
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda FL en el aire. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-fl PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const SPEED: i16        = 30;    // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;    // rampa ~600 ms
const HOLD_MS: u16      = 15000; // HOLD largo para alcanzar a medir

/// Rampa de `from` a `to` aplicando set_speed en cada paso.
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
    let _ = ufmt::uwriteln!(&mut serial, "[debug_fl] init - SOLO FL (RPWM=D10 LPWM=D45 R_EN=D22 L_EN=D24)");

    // Solo los timers que usa la FL.
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // ÚNICO motor inicializado. BTS7960::new() arranca DESHABILITADO.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(),
        pins.d24.into_output(),
        true, // invertido (como main.rs)
    );

    // Margen para soltar conectores si algo está mal.
    let _ = ufmt::uwriteln!(&mut serial, "[debug_fl] ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    fl.enable(); // R_EN=L_EN=HIGH

    // --- ADELANTE con HOLD largo ---
    let _ = ufmt::uwriteln!(&mut serial, "ADELANTE 30% (HOLD 15s) -> FL es inv.: PWM sale por LPWM=D45 (D10=0 OK). Mide B+, D22, D24, D45, M+/M-");
    ramp_to(&mut fl, 0, SPEED);
    arduino_hal::delay_ms(HOLD_MS as u32);
    ramp_to(&mut fl, SPEED, 0);

    // --- STOP definitivo ---
    fl.stop();
    fl.disable(); // R_EN=L_EN=LOW (Hi-Z)
    let _ = ufmt::uwriteln!(&mut serial, "STOP -> parado y deshabilitado. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
