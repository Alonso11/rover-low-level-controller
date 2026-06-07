// Version: v1.0 (HW SANITY TEST — all-bts7960, sin I2C)
//! Test combinado de motores + encoders + ACS712 alineado con el PDF
//! `cableado_rover_olympus_all_bts.pdf` (variante all-bts7960).
//!
//! ## Qué hace
//! 1. Banner + cabecera CSV.
//! 2. Inicializa 6× BTS7960Motor, 6× QuadratureEncoder (ISRs INT0..INT5),
//!    6× ACS712-20A en A0..A5.
//! 3. Cuenta atrás 3..2..1, habilita los R_EN/L_EN.
//! 4. Baseline 500 ms en reposo (offset cero ACS712).
//! 5. Rampa 0→40% (800 ms), hold 3 s, rampa 40%→0 (800 ms).
//! 6. Detiene y deshabilita los drivers.
//! 7. Idle 5 s muestreando para ver inercia + corriente residual.
//!
//! Sin INA3221, sin VL53L0X, sin MPU-6050, sin TF02, sin HC-SR04, sin MSM.
//! Solo lo que el usuario tiene conectado: motores + encoders + ACS712.
//!
//! ## Cableado esperado (Subsistemas 2, 3, 6 del PDF all-bts7960)
//!
//! | Motor | RPWM | LPWM | R_EN | L_EN | Enc A | Enc B | ACS712 |
//! |-------|------|------|------|------|-------|-------|--------|
//! | FR    | D9   | D44  | D23  | D25  | D21   | A13   | A0     |
//! | FL    | D10  | D45  | D22  | D24  | D20   | A14   | A1     |
//! | CR    | D5   | D11  | D28  | D29  | D19   | D46   | A2     |
//! | CL    | D6   | D12  | D30  | D31  | D18   | D47   | A3     |
//! | RR    | D7   | D13  | D34  | D35  | D2    | D48   | A4     |
//! | RL    | D8   | D4   | D36  | D37  | D3    | D49   | A5     |
//!
//! Los ACS712 instalados son de **20 A** (sensibilidad 100 mV/A); el PDF
//! todavía dice 30 A — actualizar el PDF o reemplazar los sensores.
//!
//! ## Formato del log (CSV con líneas de evento como comentarios `#`)
//!
//! ```
//! # test_motors_encoders_acs v1.0
//! # variant=all-bts7960 acs712=6x20A duty_target=40% hold_ms=3000
//! # csv_cols=t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl
//! # event=baseline
//! 0,0,0,0,0,0,0,0,12,-8,3,-1,4,2
//! ...
//! # event=ramp_up
//! 500,2,3,2,4,1,3,2,180,170,210,195,200,205
//! ...
//! # event=hold
//! 1300,40,142,138,189,176,184,191,1850,1920,2100,2050,1980,2010
//! ...
//! # event=ramp_down
//! 4300,38,...
//! # event=idle_freewheel
//! 5100,0,560,548,742,719,738,765,-12,8,2,-1,5,0
//! ```
//!
//! - `t_ms`: ms desde inicio (no incluye countdown).
//! - `duty`: 0..40 (igual para los 6 motores).
//! - `t_xx`: tick_count i32 de cada encoder en ese instante.
//! - `i_xx`: corriente mA (signed; signo según polaridad IP+/IP-).
//!
//! ## Precauciones
//! - Rover ELEVADO con ruedas SIN tocar suelo.
//! - Bench supply 14 V con límite de corriente (fusible 40 A o lab supply
//!   con CC).
//! - `make flash-test-motors-enc-acs PORT=/dev/ttyACM0`.
//! - Captura: `make monitor PORT=/dev/ttyACM0 | tee logs/2026-05-xx_test.csv`
//!   (luego filtra con `grep -v '^#' logs/...csv > clean.csv`).

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

// ── Lectura raw de Port input registers (idéntico a main.rs) ────────────
// PIND=0x29, PINE=0x2C, PINL=0x109, PINK=0x106 — ver datasheet ATmega2560.
const PIND_ADDR: *const u8 = 0x29  as *const u8;
const PINE_ADDR: *const u8 = 0x2C  as *const u8;
const PINL_ADDR: *const u8 = 0x109 as *const u8;
const PINK_ADDR: *const u8 = 0x106 as *const u8;

// ── ISRs INT0..INT5 (any-edge en fase A) ────────────────────────────────
// Replica el patrón de main.rs:108-156 bajo cfg(all-bts7960):
// fase B de FR/FL en PK5/PK6 (A13/A14), CR/CL/RR/RL en Port L.

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

// ── Parámetros del test ─────────────────────────────────────────────────
const RAMP_TARGET:     i16 = 40;     // duty máximo (0..100)
const RAMP_STEP:       i16 = 2;      // incremento por iteración
const STEP_MS:         u16 = 40;     // periodo de paso de rampa Y muestreo
const HOLD_MS:         u16 = 30000;  // duración hold (extendido para medir con multímetro)
const BASELINE_MS:     u16 = 500;    // muestreo en reposo antes de habilitar
const IDLE_MS:         u16 = 5000;   // muestreo tras parar (inercia + freewheel)
const ADC_AVG_SAMPLES: u8  = 4;      // promedio por canal (reduce ruido)

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    // ── Banner + cabecera CSV ───────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# test_motors_encoders_acs v1.0");
    let _ = ufmt::uwriteln!(&mut serial,
        "# variant=all-bts7960 acs712=6x20A duty_target={} hold_ms={}",
        RAMP_TARGET, HOLD_MS);
    let _ = ufmt::uwriteln!(&mut serial,
        "# csv_cols=t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl");

    // ── Timers (Prescale64 → ~490 Hz Phase Correct 8-bit) ───────────────
    let mut t0 = Timer0Pwm::new(dp.TC0, Prescaler::Prescale64);
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

    // ── 6× BTS7960 (cableado Subsistema 2 all-bts) ──────────────────────
    let mut fr = BTS7960Motor::new(
        pins.d9 .into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), false,
    );
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), false,
    );
    let mut cr = BTS7960Motor::new(
        pins.d5 .into_output().into_pwm(&mut t3),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), false,
    );
    let mut cl = BTS7960Motor::new(
        pins.d6 .into_output().into_pwm(&mut t4),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), false,
    );
    let mut rr = BTS7960Motor::new(
        pins.d7 .into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );
    let mut rl = BTS7960Motor::new(
        pins.d8 .into_output().into_pwm(&mut t4),
        pins.d4 .into_output().into_pwm(&mut t0),
        pins.d36.into_output(), pins.d37.into_output(), false,
    );

    // ── Encoders (Subsistema 3 all-bts) ─────────────────────────────────
    // Fase A en INTs, fase B como entrada digital pull-up.
    let _enc_fr_a = pins.d21.into_pull_up_input(); // INT0
    let _enc_fl_a = pins.d20.into_pull_up_input(); // INT1
    let _enc_cr_a = pins.d19.into_pull_up_input(); // INT2
    let _enc_cl_a = pins.d18.into_pull_up_input(); // INT3
    let _enc_rr_a = pins.d2 .into_pull_up_input(); // INT4
    let _enc_rl_a = pins.d3 .into_pull_up_input(); // INT5
    let _enc_fr_b = pins.a13.into_pull_up_input(); // PK5 (movido desde D44)
    let _enc_fl_b = pins.a14.into_pull_up_input(); // PK6 (movido desde D45)
    let _enc_cr_b = pins.d46.into_pull_up_input(); // PL3
    let _enc_cl_b = pins.d47.into_pull_up_input(); // PL2
    let _enc_rr_b = pins.d48.into_pull_up_input(); // PL1
    let _enc_rl_b = pins.d49.into_pull_up_input(); // PL0

    // EICRA = 0x55, EICRB = 0x05, EIMSK = 0x3F → any-edge INT0..INT5
    // (idéntico a main.rs:563-565)
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0x55) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x05) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });

    // ── ACS712-20A en A0..A5 (Subsistema 6) ─────────────────────────────
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fr_pin = pins.a0.into_analog_input(&mut adc);
    let acs_fl_pin = pins.a1.into_analog_input(&mut adc);
    let acs_cr_pin = pins.a2.into_analog_input(&mut adc);
    let acs_cl_pin = pins.a3.into_analog_input(&mut adc);
    let acs_rr_pin = pins.a4.into_analog_input(&mut adc);
    let acs_rl_pin = pins.a5.into_analog_input(&mut adc);
    let acs: [ACS712; 6] = [ACS712::new_20a(); 6];

    unsafe { avr_device::interrupt::enable() };

    // ── Helper: lee los 6 ACS712 (con promedio) y los 6 ticks ───────────
    // Devuelve ticks[6], current_ma[6]. Macro porque el ADC requiere los
    // pines tipados distintos en cada invocación.
    macro_rules! sample {
        () => {{
            let mut adc_avg = [0u16; 6];
            // Lecturas intercaladas A0..A5 (ATmega2560 ADC es secuencial).
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
                ENCODER_FR.get_counts(),
                ENCODER_FL.get_counts(),
                ENCODER_CR.get_counts(),
                ENCODER_CL.get_counts(),
                ENCODER_RR.get_counts(),
                ENCODER_RL.get_counts(),
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
                ma[0],    ma[1],    ma[2],    ma[3],    ma[4],    ma[5],
            );
        }};
    }

    // ── Countdown ───────────────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# countdown 3...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "# countdown 2...");
    arduino_hal::delay_ms(1000);
    let _ = ufmt::uwriteln!(&mut serial, "# countdown 1...");
    arduino_hal::delay_ms(1000);

    // ── Fase 1: baseline (drivers OFF, lee offset cero) ─────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=baseline");
    let mut t_ms: u32 = 0;
    let baseline_steps = BASELINE_MS / STEP_MS;
    for _ in 0..baseline_steps {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Habilitar drivers ───────────────────────────────────────────────
    fr.enable(); fl.enable();
    cr.enable(); cl.enable();
    rr.enable(); rl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_enabled");

    // ── Fase 2: rampa 0→RAMP_TARGET ─────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET {
        duty += RAMP_STEP;
        if duty > RAMP_TARGET { duty = RAMP_TARGET; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty); rl.set_speed(duty);
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Fase 3: hold ────────────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    let hold_steps = HOLD_MS / STEP_MS;
    for _ in 0..hold_steps {
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Fase 4: rampa RAMP_TARGET→0 ─────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty); rl.set_speed(duty);
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Parar y deshabilitar ────────────────────────────────────────────
    fr.stop(); fl.stop();
    cr.stop(); cl.stop();
    rr.stop(); rl.stop();
    fr.disable(); fl.disable();
    cr.disable(); cl.disable();
    rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_disabled");

    // ── Fase 5: idle freewheel (encoders por inercia, ACS712 ~0) ────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=idle_freewheel");
    let idle_steps = IDLE_MS / STEP_MS;
    for _ in 0..idle_steps {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# done — reset Mega para repetir");

    // Quedar imprimiendo ticks finales cada segundo (motores parados).
    loop {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(1000);
        t_ms += 1000;
    }
}
