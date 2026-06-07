// Version: v1.0 (HW DEBUG — SOLO RR motor + ACS712 A4, diagnóstico del sensor)
//! Mueve SOLO la rueda RR (BTS7960) y muestrea su ACS712 (A4) para decidir si
//! el sensor está dañado o solo tiene offset. Imprime ADC crudo + mA.
//!
//! Si el ADC crudo es ERRÁTICO (salta sin relación con el motor) o satura →
//! chip dañado. Si es ESTABLE con un offset y sube un poco al girar → solo
//! offset (calibrable con calibrate_zero).
//!
//! Secuencia: baseline 2 s (motor off) → rampa 0→50% → hold 10 s → rampa→0 → idle 2 s.
//! Pinout RR: RPWM=D7(T4), LPWM=D13(T1), R_EN=D34, L_EN=D35. ACS712-20A en A4.
//!
//! CSV: t_ms,duty,adc_raw,ma
//!
//! Flash: `make flash-debug-rr-acs PORT=/dev/ttyACM0`  (ROVER ELEVADO)

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer1Pwm, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::sensors::ACS712;

const SPEED: i16        = 50;
const RAMP_STEP: i16    = 2;
const STEP_MS: u16      = 40;
const HOLD_MS: u16      = 10000;
const BASELINE_MS: u16  = 2000;
const IDLE_MS: u16      = 2000;
const ADC_AVG: u8       = 4;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_rr_acs v1.0 - RR motor + ACS712 A4");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,duty,adc_raw,ma");

    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    let mut rr = BTS7960Motor::new(
        pins.d7.into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );

    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_pin = pins.a4.into_analog_input(&mut adc);
    let acs = ACS712::new_20a();

    macro_rules! emit {
        ($t:expr, $duty:expr) => {{
            let mut raw: u16 = 0;
            for _ in 0..ADC_AVG { raw += acs_pin.analog_read(&mut adc); }
            raw /= ADC_AVG as u16;
            let ma = acs.read_ma(raw);
            let _ = ufmt::uwriteln!(&mut serial, "{},{},{},{}", $t, $duty, raw, ma);
        }};
    }

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    let mut t_ms: u32 = 0;
    let _ = ufmt::uwriteln!(&mut serial, "# event=baseline");
    for _ in 0..(BASELINE_MS / STEP_MS) {
        emit!(t_ms, 0); arduino_hal::delay_ms(STEP_MS as u32); t_ms += STEP_MS as u32;
    }

    rr.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        rr.set_speed(duty);
        emit!(t_ms, duty); arduino_hal::delay_ms(STEP_MS as u32); t_ms += STEP_MS as u32;
    }
    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    for _ in 0..(HOLD_MS / STEP_MS) {
        emit!(t_ms, duty); arduino_hal::delay_ms(STEP_MS as u32); t_ms += STEP_MS as u32;
    }
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        rr.set_speed(duty);
        emit!(t_ms, duty); arduino_hal::delay_ms(STEP_MS as u32); t_ms += STEP_MS as u32;
    }
    rr.stop(); rr.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# event=idle");
    for _ in 0..(IDLE_MS / STEP_MS) {
        emit!(t_ms, 0); arduino_hal::delay_ms(STEP_MS as u32); t_ms += STEP_MS as u32;
    }
    let _ = ufmt::uwriteln!(&mut serial, "# done");
    loop { emit!(t_ms, 0); arduino_hal::delay_ms(500); t_ms += 500; }
}
