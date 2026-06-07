// Version: v1.0 (HW SANITY TEST — all-bts7960 + RL como L298N)
//! Variante de `test_motors_encoders_acs` para la configuración HW post-incidente
//! 2026-05-27: BTS7960 del RL quemado, sustituido por módulo L298N.
//!
//! ## Diferencias respecto a `test_motors_encoders_acs.rs`
//! - RL usa `L298NMotor` en lugar de `BTS7960Motor` (3 pines: D8=ENA/PWM,
//!   D36=IN1, D37=IN2). D4 queda sin usar.
//! - Watchdog de sobrecorriente para RL: si `|i_rl| > OC_RL_LIMIT_MA` en
//!   cualquier muestra, **se corta RL inmediatamente** (`set_speed(0)`)
//!   para no quemar el L298N (spec 2 A continuo, derated a 1.5 A).
//! - Los otros 5 motores siguen siendo BTS7960 sin cambios.
//!
//! ## Cableado RL (subsistema cambiado vs PDF original)
//!
//! | L298N RL | Pin Mega | Era antes (BTS7960) |
//! |----------|----------|--------------------|
//! | ENA      | D8       | RPWM               |
//! | IN1      | D36      | R_EN               |
//! | IN2      | D37      | L_EN               |
//! | OUT1/2   | M+/M-    | (idem)             |
//! | 12V/GND  | (idem)   | B+/B-              |
//! | (NC)     | D4       | LPWM               |
//!
//! Encoder RL y ACS712 RL **sin cambios**: D3/INT5 + D49 (encoder), A5 (ACS).
//!
//! ## Cómo flashear
//! ```
//! make flash-test-motors-enc-acs-rl-l298n PORT=/dev/ttyACM0
//! ```
//!
//! ## CSV idéntico al test anterior (incluye 14 columnas con i_rl)
//! Si la columna `i_rl` excede 1500 mA (o -1500 mA) en algún ciclo, aparece
//! la línea `# event=oc_cut_rl` y `t_rl` se congela porque el motor se apaga.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{
    IntoPwmPin, Prescaler,
    Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm,
};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
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

// ── Parámetros del test ─────────────────────────────────────────────────
const RAMP_TARGET:     i16 = 40;
const RAMP_STEP:       i16 = 2;
const STEP_MS:         u16 = 40;
const HOLD_MS:         u16 = 30000;  // extendido para medir M+ de CR/CL con multímetro
const BASELINE_MS:     u16 = 500;
const IDLE_MS:         u16 = 5000;
const ADC_AVG_SAMPLES: u8  = 4;

/// OC threshold del L298N RL. Subido a 2500 mA tras detectar que el bus PWM
/// induce ~1500 mA de crosstalk en la ACS712 RL cuando los otros 5 BTS7960
/// están activos (sesión 2026-05-27). Ver comentario detallado en
/// `config.rs::OC_FAULT_L298N`. NO sostener corrientes reales sobre 2 A.
const OC_RL_LIMIT_MA:  i32 = 2500;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# test_motors_encoders_acs_rl_l298n v1.0");
    let _ = ufmt::uwriteln!(&mut serial,
        "# variant=all-bts7960+rl-l298n acs712=6x20A duty_target={} hold_ms={} oc_rl_ma={}",
        RAMP_TARGET, HOLD_MS, OC_RL_LIMIT_MA);
    let _ = ufmt::uwriteln!(&mut serial,
        "# csv_cols=t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl");

    // ── Timers (Prescale64 → ~490 Hz) ───────────────────────────────────
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);
    // Nota: Timer0 NO se inicializa porque D4 (era LPWM del BTS7960 RL) queda
    // libre con el L298N. Si en el futuro se quiere usar D4 para otra cosa,
    // inicializar Timer0Pwm aquí.

    // ── 5× BTS7960 + 1× L298N (RL) ──────────────────────────────────────
    let mut fr = BTS7960Motor::new(
        pins.d9 .into_output().into_pwm(&mut t2),
        pins.d44.into_output().into_pwm(&mut t5),
        pins.d23.into_output(), pins.d25.into_output(), false,
    );
    // FL invertido: M+/M- físicamente al revés. Detectado 2026-05-27.
    let mut fl = BTS7960Motor::new(
        pins.d10.into_output().into_pwm(&mut t2),
        pins.d45.into_output().into_pwm(&mut t5),
        pins.d22.into_output(), pins.d24.into_output(), true,
    );
    let mut cr = BTS7960Motor::new(
        pins.d5 .into_output().into_pwm(&mut t3),
        pins.d11.into_output().into_pwm(&mut t1),
        pins.d28.into_output(), pins.d29.into_output(), false,
    );
    // CL invertido: M+/M- físicamente al revés en el módulo BTS7960 CL.
    let mut cl = BTS7960Motor::new(
        pins.d6 .into_output().into_pwm(&mut t4),
        pins.d12.into_output().into_pwm(&mut t1),
        pins.d30.into_output(), pins.d31.into_output(), true,
    );
    let mut rr = BTS7960Motor::new(
        pins.d7 .into_output().into_pwm(&mut t4),
        pins.d13.into_output().into_pwm(&mut t1),
        pins.d34.into_output(), pins.d35.into_output(), false,
    );
    // RL: L298N (sustituto post-incidente). D8=ENA, D36=IN1, D37=IN2.
    let mut rl = L298NMotor::new(
        pins.d8 .into_output().into_pwm(&mut t4),
        pins.d36.into_output(), pins.d37.into_output(), false,
    );

    // ── Encoders ────────────────────────────────────────────────────────
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

    // ── ACS712 ──────────────────────────────────────────────────────────
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fr_pin = pins.a0.into_analog_input(&mut adc);
    let acs_fl_pin = pins.a1.into_analog_input(&mut adc);
    let acs_cr_pin = pins.a2.into_analog_input(&mut adc);
    let acs_cl_pin = pins.a3.into_analog_input(&mut adc);
    let acs_rr_pin = pins.a4.into_analog_input(&mut adc);
    let acs_rl_pin = pins.a5.into_analog_input(&mut adc);
    // Calibración automática del cero de cada ACS712. Las muestras en este
    // punto (drivers aún sin habilitar) representan el offset físico del
    // sensor — incluye variación de fábrica y errores de cableado VCC/GND.
    // Sin esta calibración, los chips con offset grande (>500 mA) disparan
    // OC falsamente y bloquean la prueba antes de que los motores arranquen.
    let _ = ufmt::uwriteln!(&mut serial, "# event=acs_calibration");
    let mut zero_mv = [2500i32; 6]; // fallback al ideal VCC/2 si la cal. falla
    const CAL_SAMPLES: u8 = 32;
    let mut cal_sum = [0u32; 6];
    for _ in 0..CAL_SAMPLES {
        cal_sum[0] += acs_fr_pin.analog_read(&mut adc) as u32;
        cal_sum[1] += acs_fl_pin.analog_read(&mut adc) as u32;
        cal_sum[2] += acs_cr_pin.analog_read(&mut adc) as u32;
        cal_sum[3] += acs_cl_pin.analog_read(&mut adc) as u32;
        cal_sum[4] += acs_rr_pin.analog_read(&mut adc) as u32;
        cal_sum[5] += acs_rl_pin.analog_read(&mut adc) as u32;
        arduino_hal::delay_ms(5);
    }
    for i in 0..6 {
        let adc_avg = cal_sum[i] / CAL_SAMPLES as u32;
        zero_mv[i] = ((adc_avg * 5000) / 1023) as i32;
    }
    let acs: [ACS712; 6] = [
        ACS712::new_20a().calibrate_zero(zero_mv[0]),
        ACS712::new_20a().calibrate_zero(zero_mv[1]),
        ACS712::new_20a().calibrate_zero(zero_mv[2]),
        ACS712::new_20a().calibrate_zero(zero_mv[3]),
        ACS712::new_20a().calibrate_zero(zero_mv[4]),
        ACS712::new_20a().calibrate_zero(zero_mv[5]),
    ];
    let _ = ufmt::uwriteln!(&mut serial,
        "# acs_zero_mv fr={} fl={} cr={} cl={} rr={} rl={}",
        zero_mv[0], zero_mv[1], zero_mv[2], zero_mv[3], zero_mv[4], zero_mv[5]);

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

    // Estado del watchdog OC para RL: una vez cortado no se reanuda hasta reset.
    let mut rl_cut: bool = false;

    macro_rules! emit_csv_and_check_oc {
        ($t_ms:expr, $duty:expr) => {{
            let (ticks, ma) = sample!();
            // OC check RL: si la corriente absoluta supera el límite, corta RL
            // inmediatamente. La corriente se mide con offset de baseline aún
            // sin restar; un offset grande genera falso positivo, así que el
            // chequeo usa el valor "delta vs zero teórico" (acs.read_ma ya hace
            // esto pero el offset físico del chip puede ser grande). Si hay
            // falsos positivos por offset, calibrar con .calibrate_zero() antes.
            if !rl_cut && ma[5].abs() > OC_RL_LIMIT_MA {
                rl.set_speed(0);
                rl_cut = true;
                let _ = ufmt::uwriteln!(&mut serial,
                    "# event=oc_cut_rl ma_rl={} threshold={}", ma[5], OC_RL_LIMIT_MA);
            }
            let _ = ufmt::uwriteln!(&mut serial,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                $t_ms, $duty,
                ticks[0], ticks[1], ticks[2], ticks[3], ticks[4], ticks[5],
                ma[0],    ma[1],    ma[2],    ma[3],    ma[4],    ma[5],
            );
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
        emit_csv_and_check_oc!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    fr.enable(); fl.enable();
    cr.enable(); cl.enable();
    rr.enable();
    // L298N no tiene enable() significativo (no-op del default trait); en este
    // diseño los IN1/IN2 se controlan en cada set_speed.
    rl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_enabled");

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET {
        duty += RAMP_STEP;
        if duty > RAMP_TARGET { duty = RAMP_TARGET; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty);
        if !rl_cut { rl.set_speed(duty); }
        emit_csv_and_check_oc!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    for _ in 0..(HOLD_MS / STEP_MS) {
        emit_csv_and_check_oc!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty);
        if !rl_cut { rl.set_speed(duty); }
        emit_csv_and_check_oc!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    fr.stop(); fl.stop();
    cr.stop(); cl.stop();
    rr.stop(); rl.stop();
    fr.disable(); fl.disable();
    cr.disable(); cl.disable();
    rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial,
        "# drivers_disabled (rl_cut_by_oc={})",
        if rl_cut { 1u8 } else { 0u8 });

    let _ = ufmt::uwriteln!(&mut serial, "# event=idle_freewheel");
    for _ in 0..(IDLE_MS / STEP_MS) {
        emit_csv_and_check_oc!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# done — reset Mega para repetir");

    loop {
        emit_csv_and_check_oc!(t_ms, 0);
        arduino_hal::delay_ms(1000);
        t_ms += 1000;
    }
}
