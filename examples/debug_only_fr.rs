// Version: v3.0 (HW DEBUG — SOLO FR, ambos sentidos + ENCODER + test de fuga/chip)
//! Diagnóstico de la rueda **FR** (frontal derecho) en total aislamiento, con
//! conteo de su encoder en cuadratura → da una REFERENCIA cuantitativa (ticks por
//! sentido) además de la observación visual/multímetro.
//!
//! Pensado para depurar el síntoma "**el FR solo se mueve hacia un lado**".
//! Recorre en LOOP cinco fases, emitiendo CSV por serial y un neto por fase:
//!   1. FRENO   (EN=5V, duty 0)  → quieta. neto ~0. 🚩 si !=0 → fuga gateada por EN.
//!   2. ADELANTE (+30%, 3 s)     → gira adelante → enc_delta POSITIVO.
//!   3. STOP    (1 s)
//!   4. ATRÁS   (−30%, 3 s)      → gira atrás → enc_delta NEGATIVO.
//!                                  🚩 si REV≈0 y FWD>0 → el FR SOLO gira un sentido.
//!   5. DESENERGIZADO (EN=0V, Hi-Z, 3 s) → quieta. neto ~0.
//!                                  🚩 si gira con EN=LOW → chip BTS7960 FR DAÑADO → reemplazar.
//!
//! ## Pinout (idéntico a main.rs, BTS7960 all-bts7960)
//! | Señal | Pin  | Timer / IRQ           |
//! |-------|------|-----------------------|
//! | RPWM  | D9   | Timer2 OC2B           |
//! | LPWM  | D44  | Timer5 OC5C           |
//! | R_EN  | D23  | digital out           |
//! | L_EN  | D25  | digital out           |
//! | Enc A | D21  | INT0 (any-edge)       |
//! | Enc B | A13  | PK5 (leído en la ISR) |
//!
//! CSV: `t_ms,fase,duty,enc_ticks,enc_delta`  (fase: FRENO/FWD/REV/OFF)
//!
//! ## Precauciones
//! - ROVER ELEVADO, rueda FR sin tocar el suelo. 12 V (B+) presente.
//!
//! Flash: `make flash-debug-only-fr PORT=/dev/ttyACM0`

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::sensors::{Encoder, QuadratureEncoder};

static ENCODER_FR: QuadratureEncoder = QuadratureEncoder::new();

const PIND_ADDR: *const u8 = 0x29  as *const u8; // D21 = PD0
const PINK_ADDR: *const u8 = 0x106 as *const u8; // A13 = PK5

#[avr_device::interrupt(atmega2560)]
fn INT0() {
    let a = (unsafe { core::ptr::read_volatile(PIND_ADDR) } & (1 << 0)) != 0;
    let b = (unsafe { core::ptr::read_volatile(PINK_ADDR) } & (1 << 5)) != 0;
    ENCODER_FR.on_edge(a, b);
}

const SPEED: i16        = 30;   // duty (%) suave
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;   // rampa ~600 ms
const RUN_MS: u16       = 3000; // marcha por sentido (3 s)
const HOLD_MS: u16      = 3000; // observación en reposo
const PRINT_MS: u16     = 250;  // periodo de muestreo CSV

/// Rampa de `from` a `to` aplicando set_speed en cada paso (respeta el signo).
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

/// Muestrea el encoder durante `dur_ms`, emite una fila CSV cada PRINT_MS y
/// retorna el neto de ticks de la ventana.
fn sample_window<W: ufmt::uWrite>(
    serial: &mut W, t_ms: &mut u32, fase: &str, duty: i16, dur_ms: u16,
) -> i32 {
    let start = ENCODER_FR.get_counts();
    let mut prev = start;
    for _ in 0..(dur_ms / PRINT_MS) {
        arduino_hal::delay_ms(PRINT_MS as u32);
        *t_ms += PRINT_MS as u32;
        let now = ENCODER_FR.get_counts();
        let _ = ufmt::uwriteln!(serial, "{},{},{},{},{}", *t_ms, fase, duty, now, now - prev);
        prev = now;
    }
    ENCODER_FR.get_counts() - start
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "# debug_only_fr v3.0 - SOLO FR, ambos sentidos + encoder");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,fase,duty,enc_ticks,enc_delta");

    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // ÚNICO motor inicializado. BTS7960::new() arranca DESHABILITADO.
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(),
        pins.d25.into_output(),
        true, // invertido: +speed = adelante
    );

    // Encoder FR: fase A = D21/INT0, fase B = A13/PK5.
    let _a_fr = pins.d21.into_pull_up_input();
    let _b_fr = pins.a13.into_pull_up_input();
    // INT0 any-edge: EICRA ISC01:00 = 01 → 0x01. EIMSK = 0x01.
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x01) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x01) });
    unsafe { avr_device::interrupt::enable() };

    let _ = ufmt::uwriteln!(&mut serial, "# ROVER ELEVADO. arranque en 3...2...1");
    arduino_hal::delay_ms(3000);

    let mut t_ms: u32 = 0;
    loop {
        // 1. FRENO: EN alto + IN 0 (estado sostenido que hacía girar al FR).
        fr.enable();
        fr.stop();
        let _ = ufmt::uwriteln!(&mut serial, "# 1) FRENO (EN=5V, duty 0): neto ~0; si !=0 -> fuga high-side gateada por EN");
        let f = sample_window(&mut serial, &mut t_ms, "FRENO", 0, HOLD_MS);

        // 2. ADELANTE.
        let _ = ufmt::uwriteln!(&mut serial, "# 2) ADELANTE +30%: enc_delta debe ser POSITIVO");
        ramp_to(&mut fr, 0, SPEED);
        let fwd = sample_window(&mut serial, &mut t_ms, "FWD", SPEED, RUN_MS);
        ramp_to(&mut fr, SPEED, 0);

        // 3. STOP breve.
        fr.stop();
        let _ = ufmt::uwriteln!(&mut serial, "# 3) STOP (1s)");
        arduino_hal::delay_ms(1000);

        // 4. ATRAS.
        let _ = ufmt::uwriteln!(&mut serial, "# 4) ATRAS -30%: enc_delta debe ser NEGATIVO");
        ramp_to(&mut fr, 0, -SPEED);
        let rev = sample_window(&mut serial, &mut t_ms, "REV", -SPEED, RUN_MS);
        ramp_to(&mut fr, -SPEED, 0);

        // 5. DESENERGIZADO: EN low (Hi-Z) = corte real.
        fr.stop();
        fr.disable();
        let _ = ufmt::uwriteln!(&mut serial, "# 5) DESENERGIZADO (EN=0V, Hi-Z): neto ~0; si !=0 con EN=LOW -> chip FR DANADO");
        let off = sample_window(&mut serial, &mut t_ms, "OFF", 0, HOLD_MS);

        let _ = ufmt::uwriteln!(&mut serial,
            "# === resumen: FRENO={} FWD={} REV={} OFF={} (si REV~0 y FWD>0 -> FR solo un sentido) ===",
            f, fwd, rev, off);
    }
}
