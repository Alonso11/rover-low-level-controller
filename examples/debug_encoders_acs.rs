// Version: v1.0 (HW DEBUG — 6 motores + encoders + ACS712, config REAL bringup 2026-05-31)
//! Igual que `test_motors_encoders_acs` pero con el **cableado e inversiones
//! reales** descubiertos en el bring-up del 2026-05-31:
//!   - CR/CL con LPWM y enables INTERCAMBIADOS (RPWM D5/D6 sin cambio).
//!   - inverted: FR/FL/CL/RL = true, CR/RR = false.
//!   - RL como BTS7960 (módulo sano, NO L298N): RPWM=D8, LPWM=D4, R_EN=D36, L_EN=D37.
//!
//! Valida los 6 encoders (cuentan al girar) y los 6 ACS712 (corriente por rueda).
//!
//! ## Cableado (motores) + encoders + ACS
//! | Motor | RPWM | LPWM | R_EN | L_EN | inv | Enc A | Enc B | ACS |
//! |-------|------|------|------|------|-----|-------|-------|-----|
//! | FR    | D9   | D44  | D23  | D25  | T   | D21   | A13   | A0  |
//! | FL    | D10  | D45  | D22  | D24  | T   | D20   | A14   | A1  |
//! | CR    | D5   | D12  | D30  | D31  | F   | D19   | D46   | A2  |
//! | CL    | D6   | D11  | D28  | D29  | T   | D18   | D47   | A3  |
//! | RR    | D7   | D13  | D34  | D35  | F   | D2    | D48   | A4  |
//! | RL    | D8   | D4   | D36  | D37  | T   | D3    | D49   | A5  |
//!
//! NOTA: las asignaciones de ENCODER son las de main.rs (no se sabe aún si los
//! encoders CR/CL están cruzados como los pines de motor). El test lo revela:
//! al girar CR debe contar el encoder CR; si cuenta el de CL, están cruzados.
//!
//! ## CSV: t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl
//!
//! ## Precauciones
//! - Rover ELEVADO, 6 ruedas en el aire. 12 V con límite de corriente.
//! - NO alimentar sensores desde el Mega que carguen el riel.
//! - `make flash-debug-encoders-acs PORT=/dev/ttyACM0`
//! - Captura: `make monitor PORT=/dev/ttyACM0 | tee logs/2026-05-31_enc_acs.csv`

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
use rover_low_level_controller::sensors::{ACS712, Encoder, QuadratureEncoder};

// ── Encoders estáticos (acceso desde ISR) ───────────────────────────────
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

const RAMP_TARGET:     i16 = 40;
const RAMP_STEP:       i16 = 2;
const STEP_MS:         u16 = 40;
const HOLD_MS:         u16 = 3000;
const BASELINE_MS:     u16 = 500;
const IDLE_MS:         u16 = 3000;
const ADC_AVG_SAMPLES: u8  = 4;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_encoders_acs v1.0 (config real bringup 2026-05-31)");
    let _ = ufmt::uwriteln!(&mut serial,
        "# variant=all-bts7960 acs712=6x20A duty_target={} hold_ms={}", RAMP_TARGET, HOLD_MS);
    let _ = ufmt::uwriteln!(&mut serial,
        "# csv_cols=t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl");

    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // 6× BTS7960 — pinout + inverts REALES (bringup 2026-05-31).
    let mut fr = BTS7960Motor::new(
        pins.d9 .into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), true,
    );
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );
    // CR: LPWM=D12, R_EN=D30, L_EN=D31 (cableado real), inverted=false.
    let mut cr = BTS7960Motor::new(
        pins.d5 .into_output().into_pwm(&mut t3),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), false,
    );
    // CL: LPWM=D11, R_EN=D28, L_EN=D29 (cableado real), inverted=true.
    let mut cl = BTS7960Motor::new(
        pins.d6 .into_output().into_pwm(&mut t4),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), true,
    );
    let mut rr = BTS7960Motor::new(
        pins.d7 .into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );
    // RL como BTS (módulo sano): RPWM=D8, LPWM=D4, R_EN=D36, L_EN=D37, inverted=true.
    let mut rl = BTS7960Motor::new(
        pins.d8 .into_output().into_pwm(&mut t4),
        pins.d4 .into_output().into_pwm(&mut t0),
        pins.d36.into_output(), pins.d37.into_output(), true,
    );

    // Encoders: fase A en INTs, fase B pull-up (asignación de main.rs).
    let _enc_fr_a = pins.d21.into_pull_up_input();
    let _enc_fl_a = pins.d20.into_pull_up_input();
    let _enc_cr_a = pins.d19.into_pull_up_input();
    let _enc_cl_a = pins.d18.into_pull_up_input();
    let _enc_rr_a = pins.d2 .into_pull_up_input();
    let _enc_rl_a = pins.d3 .into_pull_up_input();
    let _enc_fr_b = pins.a13.into_pull_up_input();
    let _enc_fl_b = pins.a14.into_pull_up_input();
    let _enc_cr_b = pins.d46.into_pull_up_input();
    let _enc_cl_b = pins.d47.into_pull_up_input();
    let _enc_rr_b = pins.d48.into_pull_up_input();
    let _enc_rl_b = pins.d49.into_pull_up_input();

    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x55) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });

    // ACS712-20A en A0..A5.
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fr_pin = pins.a0.into_analog_input(&mut adc);
    let acs_fl_pin = pins.a1.into_analog_input(&mut adc);
    let acs_cr_pin = pins.a2.into_analog_input(&mut adc);
    let acs_cl_pin = pins.a3.into_analog_input(&mut adc);
    let acs_rr_pin = pins.a4.into_analog_input(&mut adc);
    let acs_rl_pin = pins.a5.into_analog_input(&mut adc);
    let acs: [ACS712; 6] = [ACS712::new_20a(); 6];

    unsafe { avr_device::interrupt::enable() };

    macro_rules! sample {
        () => {{
            let mut adc_avg = [0u16; 6];
            for _ in 0..ADC_AVG_SAMPLES {
                adc_avg[0] += acs_fr_pin.analog_read(&mut adc);
                adc_avg[1] += acs_fl_pin.analog_read(&mut adc);
                adc_avg[2] += acs_cr_pin.analog_read(&mut adc);
                adc_avg[3] += acs_cl_pin.analog_read(&mut adc);
                adc_avg[4] += acs_rr_pin.analog_read(&mut adc);
                adc_avg[5] += acs_rl_pin.analog_read(&mut adc);
            }
            let ma: [i32; 6] = [
                acs[0].read_ma(adc_avg[0] / ADC_AVG_SAMPLES as u16),
                acs[1].read_ma(adc_avg[1] / ADC_AVG_SAMPLES as u16),
                acs[2].read_ma(adc_avg[2] / ADC_AVG_SAMPLES as u16),
                acs[3].read_ma(adc_avg[3] / ADC_AVG_SAMPLES as u16),
                acs[4].read_ma(adc_avg[4] / ADC_AVG_SAMPLES as u16),
                acs[5].read_ma(adc_avg[5] / ADC_AVG_SAMPLES as u16),
            ];
            let ticks: [i32; 6] = [
                ENCODER_FR.get_counts(), ENCODER_FL.get_counts(),
                ENCODER_CR.get_counts(), ENCODER_CL.get_counts(),
                ENCODER_RR.get_counts(), ENCODER_RL.get_counts(),
            ];
            (ticks, ma)
        }};
    }
    macro_rules! emit_csv {
        ($t_ms:expr, $duty:expr) => {{
            let (ticks, ma) = sample!();
            let _ = ufmt::uwriteln!(&mut serial,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                $t_ms, $duty,
                ticks[0], ticks[1], ticks[2], ticks[3], ticks[4], ticks[5],
                ma[0], ma[1], ma[2], ma[3], ma[4], ma[5]);
        }};
    }

    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "# countdown 2...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "# countdown 1...");
    arduino_hal::delay_ms(1000);

    let _ = ufmt::uwriteln!(&mut serial, "# event=baseline");
    let mut t_ms: u32 = 0;
    for _ in 0..(BASELINE_MS / STEP_MS) {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    fr.enable(); fl.enable(); cr.enable(); cl.enable(); rr.enable(); rl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_enabled");

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET {
        duty += RAMP_STEP;
        if duty > RAMP_TARGET { duty = RAMP_TARGET; }
        fr.set_speed(duty); fl.set_speed(duty); cr.set_speed(duty);
        cl.set_speed(duty); rr.set_speed(duty); rl.set_speed(duty);
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    for _ in 0..(HOLD_MS / STEP_MS) {
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty); cr.set_speed(duty);
        cl.set_speed(duty); rr.set_speed(duty); rl.set_speed(duty);
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    fr.stop(); fl.stop(); cr.stop(); cl.stop(); rr.stop(); rl.stop();
    fr.disable(); fl.disable(); cr.disable(); cl.disable(); rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_disabled");

    let _ = ufmt::uwriteln!(&mut serial, "# event=idle_freewheel");
    for _ in 0..(IDLE_MS / STEP_MS) {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# done — reset Mega para repetir");
    loop {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(1000);
        t_ms += 1000;
    }
}
