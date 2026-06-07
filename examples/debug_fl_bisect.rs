// Version: v1.0 (HW DEBUG — bisección del INT que rompe el conteo de FL)
//! Mueve SOLO el motor FL a 30% de forma CONTINUA y va desenmascarando los
//! interrupts externos UNO A UNO (EICRA/EICRB ya en any-edge para los 6). En
//! cada fase mide el Δticks de FL durante una ventana fija. Un solo flasheo
//! identifica qué INT, al habilitarse, hace caer el conteo de FL (INT1) a ~0.
//!
//! Fases (EIMSK acumulativo):
//!   P0: 0x02  INT1 solo        (baseline; debe contar — como debug_only_fl_enc)
//!   P1: 0x03  +INT0  (FR/D21)
//!   P2: 0x07  +INT2  (CR/D19)
//!   P3: 0x0F  +INT3  (CL/D18)
//!   P4: 0x1F  +INT4  (RR/D2)
//!   P5: 0x3F  +INT5  (RL/D3)
//!
//! Interpretación: la primera fase en la que `fl_delta` cae a ~0 señala el INT
//! culpable. Si cae en P1 (+INT0) → la interacción es FR/INT0 (ambos Port K).
//! Antes de desenmascarar se limpia EIFR para descartar flancos pendientes.
//!
//! Pinout FL: RPWM=D10(T2), LPWM=D45(T5), R_EN=D22, L_EN=D24, inverted=true.
//! Enc A FL=D20/INT1 (PD1), fase B FL=A14/PK6.
//!
//! CSV: t_ms,phase,eimsk,fl_delta,fl_total
//!
//! Flash: `make flash-debug-fl-bisect PORT=/dev/ttyACM0`  (ROVER ELEVADO)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
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
const PHASE_MS: u16     = 2000;   // ventana por fase
const PRINT_MS: u16     = 200;

// EIMSK acumulativo y etiqueta del INT que se añade en cada fase.
const PHASES: [(u8, u8); 6] = [
    (0x02, 1), // INT1 solo
    (0x03, 0), // +INT0 (FR)
    (0x07, 2), // +INT2 (CR)
    (0x0F, 3), // +INT3 (CL)
    (0x1F, 4), // +INT4 (RR)
    (0x3F, 5), // +INT5 (RL)
];

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_fl_bisect v1.0 - FL continuo, desenmascara INT uno a uno");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,phase,eimsk,fl_delta,fl_total");

    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );

    // Todos los pines de encoder como pull-up (igual que el combinado/main).
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

    // EICRA/EICRB: los 6 en any-edge (como el combinado). EIMSK arranca solo INT1.
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x55) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x02) });
    unsafe { avr_device::interrupt::enable() };

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    // Rampa a 30% y mantener girando todo el barrido.
    fl.enable();
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    let _ = ufmt::uwriteln!(&mut serial, "# barrido: la 1a fase con fl_delta~0 senala el INT culpable");
    let mut t_ms: u32 = 0;
    let mut prev_total = ENCODER_FL.get_counts();

    for &(eimsk, added) in PHASES.iter() {
        // Limpiar flancos pendientes y desenmascarar la nueva mascara.
        dp.EXINT.eifr().write(|w| unsafe { w.bits(0x3F) });
        dp.EXINT.eimsk().write(|w| unsafe { w.bits(eimsk) });
        let _ = ufmt::uwriteln!(&mut serial, "# --- fase: +INT{} -> eimsk=0x{:02X}", added, eimsk);

        let phase_start = ENCODER_FL.get_counts();
        for _ in 0..(PHASE_MS / PRINT_MS) {
            let total = ENCODER_FL.get_counts();
            let delta = total - prev_total;
            let _ = ufmt::uwriteln!(&mut serial, "{},{},{},{},{}", t_ms, added, eimsk, delta, total);
            prev_total = total;
            arduino_hal::delay_ms(PRINT_MS as u32);
            t_ms += PRINT_MS as u32;
        }
        let phase_total = ENCODER_FL.get_counts() - phase_start;
        let _ = ufmt::uwriteln!(&mut serial, "# fase +INT{} total_ticks_FL={}", added, phase_total);
    }

    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
    fl.stop(); fl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# done - fl_total_final={}", ENCODER_FL.get_counts());
    loop { arduino_hal::delay_ms(1000); }
}
