// Version: v1.0 (HW DEBUG — FR vs FL, comparación directa de conteo de encoder)
//! Mueve FR y FL al MISMO duty y cuenta ambos encoders con SOLO INT0+INT1
//! habilitados. Sirve para confirmar la sobre-cuenta de FR (~2× vs FL) vista en
//! la integración: si a igual velocidad visible FR acumula ~2× ticks que FL,
//! son flancos espurios en la fase A de FR (D21/INT0); si cuentan parejo, FR
//! giraba más rápido (mecánico).
//!
//! Cada 200 ms imprime ticks acumulados y Δticks de ambos, más el ratio fr/fl
//! (×100). Esperado si están sanos: ratio ≈ 100. Si FR sobre-cuenta: ≈ 200.
//!
//! ## Pinout
//! | Rueda | RPWM | LPWM | R_EN | L_EN | Enc A      | Enc B    | inv |
//! |-------|------|------|------|------|------------|----------|-----|
//! | FR    | D9   | D44  | D23  | D25  | D21/INT0   | A13/PK5  | true|
//! | FL    | D10  | D45  | D22  | D24  | D20/INT1   | A14/PK6  | true|
//!
//! CSV: t_ms,fr_ticks,fl_ticks,fr_d,fl_d,ratio_x100
//!
//! Flash: `make flash-debug-fr-enc PORT=/dev/ttyACM1`  (ROVER ELEVADO)

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

const PIND_ADDR: *const u8 = 0x29  as *const u8; // D21=PD0, D20=PD1
const PINK_ADDR: *const u8 = 0x106 as *const u8; // A13=PK5, A14=PK6

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

const SPEED: i16        = 30;
const RAMP_STEP: i16    = 2;
const RAMP_STEP_MS: u16 = 40;
const HOLD_MS: u16      = 6000;
const PRINT_MS: u16     = 200;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_only_fr_enc v1.0 - FR vs FL mismo duty, solo INT0+INT1");
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,fr_ticks,fl_ticks,fr_d,fl_d,ratio_x100");

    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // FR — RPWM=D9(T2), LPWM=D44(T5), R_EN=D23, L_EN=D25, inverted=true.
    let mut fr = BTS7960Motor::new(
        pins.d9 .into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), true,
    );
    // FL — RPWM=D10(T2), LPWM=D45(T5), R_EN=D22, L_EN=D24, inverted=true.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );

    // Encoders: fase A FR=D21/INT0, FL=D20/INT1; fase B FR=A13/PK5, FL=A14/PK6.
    let _a_fr = pins.d21.into_pull_up_input();
    let _a_fl = pins.d20.into_pull_up_input();
    let _b_fr = pins.a13.into_pull_up_input();
    let _b_fl = pins.a14.into_pull_up_input();

    // Solo INT0+INT1 any-edge: EICRA ISC01:00=01, ISC11:10=01 → 0x05. EIMSK=0x03.
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x03) });
    unsafe { avr_device::interrupt::enable() };

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...2...1");
    arduino_hal::delay_ms(3000);

    fr.enable(); fl.enable();
    let mut duty: i16 = 0;
    while duty < SPEED {
        duty += RAMP_STEP; if duty > SPEED { duty = SPEED; }
        fr.set_speed(duty); fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }

    let _ = ufmt::uwriteln!(&mut serial, "# hold - FR y FL al mismo duty. ratio~100=ok, ~200=FR sobre-cuenta:");
    let mut t_ms: u32 = 0;
    let mut prev_fr: i32 = ENCODER_FR.get_counts();
    let mut prev_fl: i32 = ENCODER_FL.get_counts();
    for _ in 0..(HOLD_MS / PRINT_MS) {
        let fr_t = ENCODER_FR.get_counts();
        let fl_t = ENCODER_FL.get_counts();
        let fr_d = fr_t - prev_fr;
        let fl_d = fl_t - prev_fl;
        // ratio |fr_d|/|fl_d| ×100, evitando div/0.
        let fld_abs = if fl_d < 0 { -fl_d } else { fl_d };
        let frd_abs = if fr_d < 0 { -fr_d } else { fr_d };
        let ratio = if fld_abs > 0 { (frd_abs * 100) / fld_abs } else { -1 };
        let _ = ufmt::uwriteln!(&mut serial, "{},{},{},{},{},{}",
            t_ms, fr_t, fl_t, fr_d, fl_d, ratio);
        prev_fr = fr_t; prev_fl = fl_t;
        arduino_hal::delay_ms(PRINT_MS as u32);
        t_ms += PRINT_MS as u32;
    }

    while duty > 0 {
        duty -= RAMP_STEP; if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty);
        arduino_hal::delay_ms(RAMP_STEP_MS as u32);
    }
    fr.stop(); fl.stop(); fr.disable(); fl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# done - final fr,fl:");
    let _ = ufmt::uwriteln!(&mut serial, "{},{},{}", t_ms, ENCODER_FR.get_counts(), ENCODER_FL.get_counts());
    loop { arduino_hal::delay_ms(1000); }
}
