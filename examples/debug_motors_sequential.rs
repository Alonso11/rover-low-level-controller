// Version: v1.0 (HW DEBUG — un motor a la vez, pinout idéntico al MSM)
//! Diagnóstico secuencial: prueba **un motor a la vez** respetando el pinout
//! real del controlador principal (`main.rs`, variante all-bts7960 + rl-l298n).
//!
//! Mientras se prueba un motor, los otros 5 quedan **DESHABILITADOS** (R_EN/L_EN
//! en LOW → salidas Hi-Z en los BTS; IN1=IN2=0 y duty=0 en el L298N del RL).
//! Así se descarta cada canal de control sin crosstalk PWM ni ambigüedad.
//!
//! Secuencia por motor: habilita → rampa 0→30 % → HOLD 5 s → rampa →0 → stop →
//! disable → pausa 3 s. Durante el HOLD el serial te dice qué pines medir.
//!
//! ## Pinout (idéntico a main.rs, all-bts7960 + rl-l298n)
//!
//! | Motor | Driver | RPWM/ENA | LPWM/IN | R_EN/IN1 | L_EN/IN2 | inverted |
//! |-------|--------|----------|---------|----------|----------|----------|
//! | FR    | BTS7960| D9 (T2)  | D44 (T5)| D23      | D25      | false    |
//! | FL    | BTS7960| D10 (T2) | D45 (T5)| D22      | D24      | **true** |
//! | CR    | BTS7960| D5 (T3)  | D11 (T1)| D28      | D29      | false    |
//! | CL    | BTS7960| D6 (T4)  | D12 (T1)| D30      | D31      | **true** |
//! | RR    | BTS7960| D7 (T4)  | D13 (T1)| D34      | D35      | false    |
//! | RL    | L298N  | D8/ENA(T4)| D36/IN1| D37/IN2  | —        | false    |
//!
//! ## Lectura esperada con multímetro/analizador durante el HOLD
//! - BTS adelante: R_EN=L_EN=5 V, RPWM conmutando (PWM ~30 %), LPWM=0 V.
//!   M+ ≈ tensión PWM, M− ≈ 0 V → el motor gira.
//! - Si vieras RPWM **y** LPWM en alto a la vez → eso es freno, no marcha.
//! - L298N (RL): ENA conmutando (PWM), IN1=5 V, IN2=0 V (o al revés).
//!
//! ## Precauciones
//! - ROVER ELEVADO, ruedas sin tocar el suelo.
//! - 12 V (B+) presente en los módulos; star-ground verificado.
//! - Para repetir, resetear el Mega (botón) — el test corre una sola vez.
//!
//! Flash: `make flash-debug-motors-seq PORT=/dev/ttyACM0`

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler,
    Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

const HOLD_MS: u16      = 5000; // tiempo de marcha fija — para alcanzar a medir
const RAMP_TARGET: i16  = 30;   // duty máximo (suave)
const RAMP_STEP: i16    = 2;    // incremento por iteración
const RAMP_STEP_MS: u16 = 40;   // delay por paso → rampa ~600 ms
const PAUSE_MS: u16     = 3000; // pausa entre motores (todo apagado)

/// Prueba un motor: habilita, rampa, hold, rampa, stop, disable, pausa.
/// `$name` y `$pins` son etiquetas que se imprimen para guiar la medición.
macro_rules! run_one {
    ($serial:expr, $m:expr, $name:expr, $pins:expr) => {{
        let _ = ufmt::uwriteln!(&mut $serial, "");
        let _ = ufmt::uwriteln!(&mut $serial, "=== {} ===", $name);
        let _ = ufmt::uwriteln!(&mut $serial, "    pines: {}", $pins);
        let _ = ufmt::uwriteln!(&mut $serial, "    habilita + rampa 0->{}", RAMP_TARGET);
        $m.enable();
        let mut duty: i16 = 0;
        while duty < RAMP_TARGET {
            duty += RAMP_STEP;
            if duty > RAMP_TARGET { duty = RAMP_TARGET; }
            $m.set_speed(duty);
            arduino_hal::delay_ms(RAMP_STEP_MS as u32);
        }
        let _ = ufmt::uwriteln!(&mut $serial, "    HOLD {}ms -> MIDE LOS PINES AHORA", HOLD_MS);
        arduino_hal::delay_ms(HOLD_MS as u32);
        while duty > 0 {
            duty -= RAMP_STEP;
            if duty < 0 { duty = 0; }
            $m.set_speed(duty);
            arduino_hal::delay_ms(RAMP_STEP_MS as u32);
        }
        $m.stop();
        $m.disable();
        let _ = ufmt::uwriteln!(&mut $serial, "    {} listo - parado y deshabilitado. Pausa {}ms", $name, PAUSE_MS);
        arduino_hal::delay_ms(PAUSE_MS as u32);
    }};
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_seq] init - prueba 1 motor a la vez (pinout MSM)");

    // Timers (Prescale64 → ~490 Hz Phase Correct 8-bit, igual que test_6_motors).
    // Timer0 no se usa: el RL es L298N con ENA=D8 (Timer4), no el BTS con D4 (Timer0).
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // --- Construcción de los 6 motores con el pinout EXACTO del MSM ---
    // BTS7960::new() arranca DESHABILITADO (R_EN/L_EN = LOW) → seguro.
    let mut fr = BTS7960Motor::new(
        pins.d9.into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), false,
    );
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true, // invertido (como main.rs)
    );
    let mut cr = BTS7960Motor::new(
        pins.d5.into_output().into_pwm(&mut t3),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), false,
    );
    let mut cl = BTS7960Motor::new(
        pins.d6.into_output().into_pwm(&mut t4),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), true, // invertido (como main.rs)
    );
    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );
    // RL = L298N (sustituto): ENA=D8 (T4), IN1=D36, IN2=D37.
    let mut rl = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut t4),
        pins.d36.into_output(), pins.d37.into_output(), false,
    );
    rl.stop(); // L298N no arranca deshabilitado: forzar parado explícito.

    // Margen para soltar conectores si algo está mal.
    let _ = ufmt::uwriteln!(&mut serial, "[debug_seq] ROVER ELEVADO. arranque en 3...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_seq] 2...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "[debug_seq] 1...");
    arduino_hal::delay_ms(1000);

    // --- Un motor a la vez (los demás siguen deshabilitados) ---
    run_one!(serial, fr, "FR (frontal der)",   "RPWM=D9  LPWM=D44 R_EN=D23 L_EN=D25");
    run_one!(serial, fl, "FL (frontal izq)",   "RPWM=D10 LPWM=D45 R_EN=D22 L_EN=D24 [inv]");
    run_one!(serial, cr, "CR (central der)",   "RPWM=D5  LPWM=D11 R_EN=D28 L_EN=D29");
    run_one!(serial, cl, "CL (central izq)",   "RPWM=D6  LPWM=D12 R_EN=D30 L_EN=D31 [inv]");
    run_one!(serial, rr, "RR (trasero der)",   "RPWM=D7  LPWM=D13 R_EN=D34 L_EN=D35");
    run_one!(serial, rl, "RL (trasero izq)",   "ENA=D8   IN1=D36  IN2=D37 [L298N]");

    let _ = ufmt::uwriteln!(&mut serial, "");
    let _ = ufmt::uwriteln!(&mut serial, "[debug_seq] DONE - los 6 probados. Resetear Mega para repetir.");

    loop { arduino_hal::delay_ms(1000); }
}
