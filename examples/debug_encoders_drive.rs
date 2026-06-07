// Version: v1.0 (HW DEBUG — 6 motores + 6 encoders, foco en verificar encoders)
//! Maneja los 6 motores (BTS7960, config real bringup 2026-05-31) a 30% y
//! imprime las cuentas de los 6 encoders cada 200 ms. Sin ACS, sin MSM.
//! Pensado para verificar el encoder FL (y los demás) cuando el motor gira,
//! ya que las ruedas no se pueden mover a mano (mucha reducción).
//!
//! Secuencia: countdown → rampa 0→30% → hold 8 s (imprime ticks) → rampa→0 → stop.
//!
//! ## Pinout (motores + encoders, config real)
//! | Motor | RPWM | LPWM | R_EN | L_EN | inv | Enc A | Enc B |
//! |-------|------|------|------|------|-----|-------|-------|
//! | FR    | D9   | D44  | D23  | D25  | T   | D21   | A13   |
//! | FL    | D10  | D45  | D22  | D24  | T   | D20   | A14   |
//! | CR    | D5   | D12  | D30  | D31  | F   | D19   | D46   |
//! | CL    | D6   | D11  | D28  | D29  | T   | D18   | D47   |
//! | RR    | D7   | D13  | D34  | D35  | F   | D2    | D48   |
//! | RL    | D8   | D4   | D36  | D37  | T   | D3    | D49   |
//!
//! CSV: t_ms,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl   (ticks acumulados de cada encoder)
//!
//! Flash: `make flash-debug-encoders-drive PORT=/dev/ttyACM0`  (ROVER ELEVADO)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler,
    Timer0Pwm, Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::sensors::{Encoder, QuadratureEncoder};

static ENCODER_FR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_FL: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_CR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_CL: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_RR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_RL: QuadratureEncoder = QuadratureEncoder::new();

const PIND_ADDR: *const u8 = 0x29  as *const u8;
const PINE_ADDR: *const u8 = 0x2C  as *const u8;
const PINL_ADDR: *const u8 = 0x109 as *const u8;
const PINK_ADDR: *const u8 = 0x106 as *const u8;

#[avr_device::interrupt(atmega2560)]
fn INT0() {
    let pind = unsafe { core::ptr::read_volatile(PIND_ADDR) };
    let b = (unsafe { core::ptr::read_volatile(PINK_ADDR) } & (1 << 5)) != 0;
    ENCODER_FR.on_edge((pind & (1 << 0)) != 0, b);
}
#[avr_device::interrupt(atmega2560)]
fn INT1() {
    let pind = unsafe { core::ptr::read_volatile(PIND_ADDR) };
    let b = (unsafe { core::ptr::read_volatile(PINK_ADDR) } & (1 << 6)) != 0;
    ENCODER_FL.on_edge((pind & (1 << 1)) != 0, b);
}
#[avr_device::interrupt(atmega2560)]
fn INT2() {
    let pind = unsafe { core::ptr::read_volatile(PIND_ADDR) };
    let pinl = unsafe { core::ptr::read_volatile(PINL_ADDR) };
    ENCODER_CR.on_edge((pind & (1 << 2)) != 0, (pinl & (1 << 3)) != 0);
}
#[avr_device::interrupt(atmega2560)]
fn INT3() {
    let pind = unsafe { core::ptr::read_volatile(PIND_ADDR) };
    let pinl = unsafe { core::ptr::read_volatile(PINL_ADDR) };
    ENCODER_CL.on_edge((pind & (1 << 3)) != 0, (pinl & (1 << 2)) != 0);
}
#[avr_device::interrupt(atmega2560)]
fn INT4() {
    let pine = unsafe { core::ptr::read_volatile(PINE_ADDR) };
    let pinl = unsafe { core::ptr::read_volatile(PINL_ADDR) };
    ENCODER_RR.on_edge((pine & (1 << 4)) != 0, (pinl & (1 << 1)) != 0);
}
#[avr_device::interrupt(atmega2560)]
fn INT5() {
    let pine = unsafe { core::ptr::read_volatile(PINE_ADDR) };
    let pinl = unsafe { core::ptr::read_volatile(PINL_ADDR) };
    ENCODER_RL.on_edge((pine & (1 << 5)) != 0, (pinl & (1 << 0)) != 0);
}

const SPEED: i16        = 30;
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;
const HOLD_MS: u16      = 8000;
const PRINT_MS: u16     = 200;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_encoders_drive v1.0");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl");

    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    let mut fr = BTS7960Motor::new(
        pins.d9 .into_output().into_pwm(&mut t2), pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), true);
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2), pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true);
    let mut cr = BTS7960Motor::new(
        pins.d5 .into_output().into_pwm(&mut t3), pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), false);
    let mut cl = BTS7960Motor::new(
        pins.d6 .into_output().into_pwm(&mut t4), pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), true);
    let mut rr = BTS7960Motor::new(
        pins.d7 .into_output().into_pwm(&mut t4), pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false);
    let mut rl = BTS7960Motor::new(
        pins.d8 .into_output().into_pwm(&mut t4), pins.d4 .into_output().into_pwm(&mut t0),
        pins.d36.into_output(), pins.d37.into_output(), true);

    // Encoders: fase A en INTs, fase B pull-up.
    let _a_fr = pins.d21.into_pull_up_input();
    let _a_fl = pins.d20.into_pull_up_input();
    let _a_cr = pins.d19.into_pull_up_input();
    let _a_cl = pins.d18.into_pull_up_input();
    let _a_rr = pins.d2 .into_pull_up_input();
    let _a_rl = pins.d3 .into_pull_up_input();
    let _b_fr = pins.a13.into_pull_up_input();
    let _b_fl = pins.a14.into_pull_up_input();
    let _b_cr = pins.d46.into_pull_up_input();
    let _b_cl = pins.d47.into_pull_up_input();
    let _b_rr = pins.d48.into_pull_up_input();
    let _b_rl = pins.d49.into_pull_up_input();

    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x55) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });
    unsafe { avr_device::interrupt::enable() };

    macro_rules! print_ticks {
        ($t:expr) => {{
            let _ = ufmt::uwriteln!(&mut serial, "{},{},{},{},{},{},{}", $t,
                ENCODER_FR.get_counts(), ENCODER_FL.get_counts(),
                ENCODER_CR.get_counts(), ENCODER_CL.get_counts(),
                ENCODER_RR.get_counts(), ENCODER_RL.get_counts());
        }};
    }

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    fr.enable(); fl.enable(); cr.enable(); cl.enable(); rr.enable(); rl.enable();

    let _ = ufmt::uwriteln!(&mut serial, "# ramp_up");
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        fr.set_speed(duty); fl.set_speed(duty); cr.set_speed(duty);
        cl.set_speed(duty); rr.set_speed(duty); rl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    let _ = ufmt::uwriteln!(&mut serial, "# hold - imprimiendo ticks");
    let mut t_ms: u32 = 0;
    for _ in 0..(HOLD_MS / PRINT_MS) {
        print_ticks!(t_ms);
        arduino_hal::delay_ms(PRINT_MS as u32);
        t_ms += PRINT_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty); cr.set_speed(duty);
        cl.set_speed(duty); rr.set_speed(duty); rl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
    fr.stop(); fl.stop(); cr.stop(); cl.stop(); rr.stop(); rl.stop();
    fr.disable(); fl.disable(); cr.disable(); cl.disable(); rr.disable(); rl.disable();

    let _ = ufmt::uwriteln!(&mut serial, "# done - ticks finales:");
    print_ticks!(t_ms);
    loop { arduino_hal::delay_ms(1000); }
}
