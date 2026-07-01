// Version: v1.0 (HW DEBUG — SOLO FL motor + su encoder)
//! Aísla la rueda FL: mueve SOLO el motor FL y lee SOLO su encoder. Sirve para
//! distinguir si el problema es el motor (no gira) o el encoder (gira pero no cuenta).
//!
//! Secuencia: countdown → rampa 0→30% → hold 8 s (imprime ticks FL cada 200 ms)
//! → rampa→0 → stop. Si el motor gira, los ticks deben subir; si suben 0, el
//! encoder FL es el problema (D20/INT1 fase A, A14/PK6 fase B).
//!
//! ## Pinout FL
//! | Señal | Pin  | Detalle      |
//! |-------|------|--------------|
//! | RPWM  | D10  | Timer2 OC2A  |
//! | LPWM  | D45  | Timer5 OC5B  |
//! | R_EN  | D22  | enable       |
//! | L_EN  | D24  | enable       |
//! | Enc A | D20  | INT1 (PD1)   |
//! | Enc B | A14  | PK6          |
//! inverted=true (gira adelante).
//!
//! CSV: t_ms,ticks_fl
//!
//! Flash: `make flash-debug-only-fl-enc PORT=/dev/ttyACM0`  (ROVER ELEVADO)

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer5Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::sensors::{Encoder, QuadratureEncoder};

static ENCODER_FL: QuadratureEncoder = QuadratureEncoder::new();

const PIND_ADDR: *const u8 = 0x29  as *const u8; // D20 = PD1
const PINK_ADDR: *const u8 = 0x106 as *const u8; // A14 = PK6

#[avr_device::interrupt(atmega2560)]
fn INT1() {
    let pind = unsafe { core::ptr::read_volatile(PIND_ADDR) };
    let b = (unsafe { core::ptr::read_volatile(PINK_ADDR) } & (1 << 6)) != 0;
    ENCODER_FL.on_edge((pind & (1 << 1)) != 0, b);
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

    let _ = ufmt::uwriteln!(&mut serial, "# debug_only_fl_enc v1.0 - SOLO FL motor + encoder");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,ticks_fl");

    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(),
        pins.d24.into_output(),
        true,
    );

    // Encoder FL: fase A en INT1/D20, fase B en A14/PK6.
    let _a = pins.d20.into_pull_up_input();
    let _b = pins.a14.into_pull_up_input();
    // Solo INT1 any-edge: EICRA bits ISC11:ISC10 = 01 (any edge) → 0x04. EIMSK bit1.
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x04) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x02) });
    unsafe { avr_device::interrupt::enable() };

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    fl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# ramp_up");
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    let _ = ufmt::uwriteln!(&mut serial, "# hold - ticks_fl (debe subir si el motor gira):");
    let mut t_ms: u32 = 0;
    for _ in 0..(HOLD_MS / PRINT_MS) {
        let _ = ufmt::uwriteln!(&mut serial, "{},{}", t_ms, ENCODER_FL.get_counts());
        arduino_hal::delay_ms(PRINT_MS as u32);
        t_ms += PRINT_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
    fl.stop(); fl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# done - ticks final:");
    let _ = ufmt::uwriteln!(&mut serial, "{},{}", t_ms, ENCODER_FL.get_counts());
    loop { arduino_hal::delay_ms(1000); }
}
