// Version: v1.0 (HW SANITY TEST — all-bts7960)
//! Test seguro de los 6 motores del rover con la variante all-bts7960.
//!
//! Mueve los 6 motores hacia ADELANTE con rampa suave 0 -> 40% (800 ms),
//! mantiene 3 s, rampa 40% -> 0% (800 ms), para. Sin ACS712, sin encoders.
//!
//! ## Cableado esperado (idéntico a Subsistema 2 del PDF cableado_rover_olympus_all_bts.pdf)
//!
//! | Motor | RPWM | LPWM | R_EN | L_EN | Timer RPWM / LPWM    |
//! |-------|------|------|------|------|----------------------|
//! | FR    | D9   | D44  | D23  | D25  | Timer2 OC2B / Timer5 OC5C |
//! | FL    | D10  | D45  | D22  | D24  | Timer2 OC2A / Timer5 OC5B |
//! | CR    | D5   | D11  | D28  | D29  | Timer3 OC3A / Timer1 OC1A |
//! | CL    | D6   | D12  | D30  | D31  | Timer4 OC4A / Timer1 OC1B |
//! | RR    | D7   | D13  | D34  | D35  | Timer4 OC4B / Timer1 OC1C |
//! | RL    | D8   | D4   | D36  | D37  | Timer4 OC4C / Timer0 OC0B |
//!
//! ## Precauciones antes de flashear
//! - Star ground verificado (Subsistema 0 del PDF).
//! - Contactores conectados a packs correctos.
//! - Rover ELEVADO con las 6 ruedas SIN tocar el suelo (primer arranque).
//! - Multímetro entre B+/B- de un BTS para confirmar 12 V de los XL4015.
//! - Si algún motor gira al revés, intercambiar M+/M- en ese motor (no en software).
//!
//! ## Banner serial esperado (115200 baudios)
//! ```
//! [test_6_motors] init
//! [test_6_motors] arranque en 3...2...1
//! [test_6_motors] rampa 0->40
//! [test_6_motors] hold 3s
//! [test_6_motors] rampa 40->0
//! [test_6_motors] done — motores deshabilitados
//! ```

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler,
    Timer0Pwm, Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

const HOLD_MS: u16        = 3000;  // duración del tramo a velocidad fija
const RAMP_TARGET: i16    = 40;    // duty máximo (0..100). Suave para primer test.
const RAMP_STEP: i16      = 2;     // incremento por iteración
const RAMP_STEP_MS: u16   = 40;    // delay entre iteraciones -> rampa total ~800 ms

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] init");

    // --- Timers (Prescale64 -> ~490 Hz Phase Correct 8-bit en todos) ---
    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // --- FR (Frontal Derecho) — RPWM=D9 (T2), LPWM=D44 (T5) ---
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(),
        pins.d25.into_output(),
        false,
    );

    // --- FL (Frontal Izquierdo) — RPWM=D10 (T2), LPWM=D45 (T5) ---
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(),
        pins.d24.into_output(),
        false,
    );

    // --- CR (Central Derecho) — RPWM=D5 (T3), LPWM=D11 (T1) ---
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(),
        pins.d29.into_output(),
        false,
    );

    // --- CL (Central Izquierdo) — RPWM=D6 (T4), LPWM=D12 (T1) ---
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(),
        pins.d31.into_output(),
        false,
    );

    // --- RR (Trasero Derecho) — RPWM=D7 (T4), LPWM=D13 (T1) ---
    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(),
        pins.d35.into_output(),
        false,
    );

    // --- RL (Trasero Izquierdo) — RPWM=D8 (T4), LPWM=D4 (T0) ---
    let mut rl = BTS7960Motor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d4.into_output().into_pwm(&mut t0),
        pins.d36.into_output(),
        pins.d37.into_output(),
        false,
    );

    // Margen antes de habilitar los drivers — da tiempo al operador para
    // soltar conectores si algo está mal.
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] arranque en 3...2...1");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] 2");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] 1");
    arduino_hal::delay_ms(1000);

    // Habilitar R_EN/L_EN en los 6 BTS7960.
    fr.enable(); fl.enable();
    cr.enable(); cl.enable();
    rr.enable(); rl.enable();

    // --- Rampa 0 -> RAMP_TARGET ---
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] rampa 0->40");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET {
        duty += RAMP_STEP;
        if duty > RAMP_TARGET { duty = RAMP_TARGET; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty); rl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    // --- Hold 3 s a RAMP_TARGET ---
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] hold 3s");
    arduino_hal::delay_ms(HOLD_MS as u32);

    // --- Rampa RAMP_TARGET -> 0 ---
    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] rampa 40->0");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty); rl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    // Parada explícita y deshabilitación de los drivers.
    fr.stop(); fl.stop();
    cr.stop(); cl.stop();
    rr.stop(); rl.stop();
    fr.disable(); fl.disable();
    cr.disable(); cl.disable();
    rr.disable(); rl.disable();

    let _ = ufmt::uwriteln!(&mut serial, "[test_6_motors] done — motores deshabilitados");

    // Quedar en idle. Para repetir el test hay que resetear el Mega.
    loop { arduino_hal::delay_ms(1000); }
}
