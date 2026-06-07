// Version: v1.0 (HW DEBUG — mueve SOLO FL, pero con los 6 INT habilitados)
//! Test decisivo para el encoder FL: mueve únicamente el motor FL, pero
//! configura los 6 encoders y habilita los 6 interrupts (EICRA=0x55, EICRB=0x05,
//! EIMSK=0x3F) igual que el firmware/combinado. Imprime las cuentas de FL (INT1)
//! y FR (INT0).
//!
//! Interpretación (solo FL gira):
//!   - FL cuenta, FR=0      → routing de INTs OK → el FL=0 del combinado era
//!                            el motor FL sin girar (potencia/conector).
//!   - FL=0, FR cuenta      → los flancos de fase A de FL disparan INT0 → cable
//!                            de fase A de FL en D21, o puente D20-D21.
//!
//! Pinout FL motor: RPWM=D10(T2), LPWM=D45(T5), R_EN=D22, L_EN=D24, inverted=true.
//! Encoders: fase A FR=D21/INT0, FL=D20/INT1; fase B FR=A13/PK5, FL=A14/PK6.
//!
//! CSV: t_ms,fl_ticks,fr_ticks
//!
//! Flash: `make flash-debug-fl-allint PORT=/dev/ttyACM0`  (ROVER ELEVADO)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::sensors::{ACS712, Encoder, QuadratureEncoder};

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

    let _ = ufmt::uwriteln!(&mut serial, "# debug_fl_allint v1.1 - mueve SOLO FL, 6 INT habilitados, + corriente FL");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,fl_ticks,fr_ticks,fl_ma");

    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // SOLO el motor FL.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );

    // ACS712 de FL en A1 (para saber si el motor energiza).
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fl_pin = pins.a1.into_analog_input(&mut adc);
    let acs_fl = ACS712::new_20a();

    // Todos los pines de encoder como entrada pull-up (como el combinado).
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

    // Los 6 INT habilitados (igual que el combinado).
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x55) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });
    unsafe { avr_device::interrupt::enable() };

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    fl.enable();
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    let _ = ufmt::uwriteln!(&mut serial, "# hold - SOLO FL. fl_ma>0 = motor energiza; fl_ticks=0 con fl_ma>0 = problema encoder/INT:");
    let mut t_ms: u32 = 0;
    for _ in 0..(HOLD_MS / PRINT_MS) {
        let mut raw: u16 = 0;
        for _ in 0..4 { raw += acs_fl_pin.analog_read(&mut adc); }
        let fl_ma = acs_fl.read_ma(raw / 4);
        let _ = ufmt::uwriteln!(&mut serial, "{},{},{},{}",
            t_ms, ENCODER_FL.get_counts(), ENCODER_FR.get_counts(), fl_ma);
        arduino_hal::delay_ms(PRINT_MS as u32);
        t_ms += PRINT_MS as u32;
    }

    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
    fl.stop(); fl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# done - final fl,fr:");
    let _ = ufmt::uwriteln!(&mut serial, "{},{},{}", t_ms, ENCODER_FL.get_counts(), ENCODER_FR.get_counts());
    loop { arduino_hal::delay_ms(1000); }
}
