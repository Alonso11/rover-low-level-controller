// Version: v1.0 — test de integración HW SIN INA3221 (sin MSM/UART/HLC)
//! Variante de `test_integration_full` que excluye el INA3221 (monitor de
//! voltaje de batería). Útil para correr el test cuando el INA todavía no
//! está cableado o cuando se diagnostica algún problema del soft I2C
//! sin que afecte al resto de sensores.
//!
//! El resto de subsistemas es idéntico a `test_integration_full`.
//!
//! ## Subsistemas incluidos
//! - 5× BTS7960 (FR/FL/CR/CL/RR) + 1× L298N (RL) — variante post-fallo BTS7960
//! - 6× QuadratureEncoder (ISRs INT0..INT5)
//! - 6× ACS712-20A en A0..A5 con auto-calibración del cero
//! - HC-SR04 frontal en D38(Trig)/D39(Echo) — proximidad ultrasónica
//! - TF02 LiDAR en USART2 D17(RX2) @ 115200 baud
//! - 4× NTC en A7..A10 — temperatura de los 2 packs (2 sensores por pack)
//!
//! ## NO incluido
//! - INA3221 (voltaje batería). Para incluirlo usar `test_integration_full`.
//!
//! ## NO incluidos (a propósito)
//! - VL53L0X, MPU-6050, LM335, NTC (no eran necesarios para esta evaluación).
//! - MSM, protocolo UART CSP, telemetría TLM (eso ya es firmware principal).
//! - OC fault handler completo (solo watchdog simple en RL).
//!
//! ## Cadencia de lecturas
//! Cada muestra @ 40 ms (STEP_MS):
//! - ACS712 × 6, encoders × 6, TF02 byte parser → cada paso
//! - HC-SR04 y INA3221 → cada 5 pasos (200 ms = 5 Hz)
//!
//! ## CSV (líneas de evento marcadas con `#`)
//! ```
//! t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,
//!   i_fr,i_fl,i_cr,i_cl,i_rr,i_rl,
//!   hc_mm,tf_mm,tf_sig,
//!   t_b1a,t_b1b,t_b2a,t_b2b
//! ```
//! - `hc_mm`/`tf_mm`: distancia en mm (0 = sin dato o fuera de rango).
//! - `tf_sig`: SIG byte del TF02 (>=7 = lectura fiable).
//! - `t_b1a/b1b`: temperatura °C del pack 1 (Principal), sensores A y B (A7/A8).
//! - `t_b2a/b2b`: temperatura °C del pack 2 (Emergencia), sensores A y B (A9/A10).

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
use rover_low_level_controller::sensors::{
    ACS712, Encoder, QuadratureEncoder, TF02, NTCThermistor,
};
use rover_low_level_controller::sensors::hc_sr04::HCSR04;
use rover_low_level_controller::command_interface::RxRingBuffer;

// ── Encoders estáticos ──────────────────────────────────────────────────
static ENCODER_FR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_FL: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_CR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_CL: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_RR: QuadratureEncoder = QuadratureEncoder::new();
static ENCODER_RL: QuadratureEncoder = QuadratureEncoder::new();

// ── Ring buffer USART2 (TF02) ───────────────────────────────────────────
static TF02_RX_BUF: RxRingBuffer = RxRingBuffer::new();

// ── Direcciones port input ──────────────────────────────────────────────
const PIND_ADDR: *const u8 = 0x29  as *const u8;
const PINE_ADDR: *const u8 = 0x2C  as *const u8;
const PINL_ADDR: *const u8 = 0x109 as *const u8;
const PINK_ADDR: *const u8 = 0x106 as *const u8;

// ── ISRs encoders ───────────────────────────────────────────────────────
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

// ── ISR USART2 RX — alimenta ring buffer del TF02 ───────────────────────
#[avr_device::interrupt(atmega2560)]
fn USART2_RX() {
    let byte = unsafe {
        (*avr_device::atmega2560::USART2::ptr()).udr2().read().bits()
    };
    unsafe { TF02_RX_BUF.push(byte); }
}

// ── Parámetros del test ─────────────────────────────────────────────────
const RAMP_TARGET:        i16 = 40;
const RAMP_STEP:          i16 = 2;
const STEP_MS:            u16 = 40;
const HOLD_MS:            u16 = 3000;
const BASELINE_MS:        u16 = 600;
const IDLE_MS:            u16 = 5000;
const ADC_AVG_SAMPLES:    u8  = 4;
const HC_INA_PERIOD:      u8  = 5;    // cada 5 pasos (200 ms = 5 Hz)
const OC_RL_LIMIT_MA:     i32 = 2500;
const HC_ECHO_TIMEOUT_US: u32 = 1_750; // ~300 mm máx

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# test_integration_no_ina v1.0");
    let _ = ufmt::uwriteln!(&mut serial,
        "# variant=all-bts7960+rl-l298n duty_target={} hold_ms={} oc_rl={}",
        RAMP_TARGET, HOLD_MS, OC_RL_LIMIT_MA);
    let _ = ufmt::uwriteln!(&mut serial,
        "# csv_cols=t_ms,duty,t_fr,t_fl,t_cr,t_cl,t_rr,t_rl,i_fr,i_fl,i_cr,i_cl,i_rr,i_rl,hc_mm,tf_mm,tf_sig,t_b1a,t_b1b,t_b2a,t_b2b");

    // ── Timers ──────────────────────────────────────────────────────────
    let mut t1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut t2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut t3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut t5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);

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
    // Detectado en test de integración 2026-05-27 (giraba opuesto al resto).
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

    // ── HC-SR04 ─────────────────────────────────────────────────────────
    let mut hcsr04 = HCSR04::new(
        pins.d38.into_output(),
        pins.d39.into_floating_input().forget_imode(),
    ).with_timeout(HC_ECHO_TIMEOUT_US);

    // ── USART2 / TF02 — D17 (RX2) @ 115200 baud, 8N1 ────────────────────
    unsafe {
        let p2 = &(*avr_device::atmega2560::USART2::ptr());
        p2.ubrr2().write(|w| w.bits(16));               // 115200 baud con U2X2=1
        p2.ucsr2a().write(|w| w.u2x2().set_bit());
        p2.ucsr2b().write(|w| w.rxen2().set_bit().rxcie2().set_bit());
        p2.ucsr2c().write(|w| w.ucsz2().chr8().usbs2().stop1());
        while p2.ucsr2a().read().rxc2().bit_is_set() {
            let _ = p2.udr2().read().bits();
        }
    }
    let mut tf02 = TF02::new();
    let mut tf_sig_logged = false;

    // ── ACS712 con auto-calibración ─────────────────────────────────────
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fr_pin = pins.a0.into_analog_input(&mut adc);
    let acs_fl_pin = pins.a1.into_analog_input(&mut adc);
    let acs_cr_pin = pins.a2.into_analog_input(&mut adc);
    let acs_cl_pin = pins.a3.into_analog_input(&mut adc);
    let acs_rr_pin = pins.a4.into_analog_input(&mut adc);
    let acs_rl_pin = pins.a5.into_analog_input(&mut adc);
    // 4× NTC para 2 packs (PDF labels banco 1 = Principal, banco 2 = Emergencia)
    let ntc_b1a_pin = pins.a7 .into_analog_input(&mut adc);
    let ntc_b1b_pin = pins.a8 .into_analog_input(&mut adc);
    let ntc_b2a_pin = pins.a9 .into_analog_input(&mut adc);
    let ntc_b2b_pin = pins.a10.into_analog_input(&mut adc);
    let ntc: [NTCThermistor; 4] = [NTCThermistor::new(); 4];

    let _ = ufmt::uwriteln!(&mut serial, "# event=acs_calibration");
    let mut zero_mv = [2500i32; 6];
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

    // ── Cached sensor values (HC y NTC se actualizan cada HC_INA_PERIOD) ─
    let mut hc_mm: u16 = 0;
    let mut ntc_c: [i32; 4] = [0, 0, 0, 0];
    let mut sample_counter: u8 = 0;
    let mut rl_cut: bool = false;

    macro_rules! emit_csv {
        ($t_ms:expr, $duty:expr) => {{
            // ACS promedio
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
            if !rl_cut && ma[5].abs() > OC_RL_LIMIT_MA {
                rl.set_speed(0);
                rl_cut = true;
                let _ = ufmt::uwriteln!(&mut serial,
                    "# event=oc_cut_rl ma_rl={} threshold={}", ma[5], OC_RL_LIMIT_MA);
            }
            // TF02: drenar bytes pendientes del ring buffer
            while let Some(byte) = TF02_RX_BUF.pop() {
                if tf02.feed(byte) && !tf_sig_logged {
                    let _ = ufmt::uwriteln!(&mut serial,
                        "# tf02_first_frame dist_mm={} sig={}",
                        tf02.last_dist_mm, tf02.last_sig);
                    tf_sig_logged = true;
                }
            }
            // HC + NTC solo cada HC_INA_PERIOD pasos
            if sample_counter == 0 {
                hc_mm = hcsr04.measure_mm().unwrap_or(0);
                ntc_c[0] = ntc[0].read_celsius(ntc_b1a_pin.analog_read(&mut adc));
                ntc_c[1] = ntc[1].read_celsius(ntc_b1b_pin.analog_read(&mut adc));
                ntc_c[2] = ntc[2].read_celsius(ntc_b2a_pin.analog_read(&mut adc));
                ntc_c[3] = ntc[3].read_celsius(ntc_b2b_pin.analog_read(&mut adc));
            }
            sample_counter = (sample_counter + 1) % HC_INA_PERIOD;

            let ticks: [i32; 6] = [
                ENCODER_FR.get_counts(),
                ENCODER_FL.get_counts(),
                ENCODER_CR.get_counts(),
                ENCODER_CL.get_counts(),
                ENCODER_RR.get_counts(),
                ENCODER_RL.get_counts(),
            ];
            let _ = ufmt::uwriteln!(&mut serial,
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
                $t_ms, $duty,
                ticks[0], ticks[1], ticks[2], ticks[3], ticks[4], ticks[5],
                ma[0],    ma[1],    ma[2],    ma[3],    ma[4],    ma[5],
                hc_mm, tf02.last_dist_mm, tf02.last_sig,
                ntc_c[0], ntc_c[1], ntc_c[2], ntc_c[3],
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

    // ── Fase 1: baseline (drivers OFF) ──────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=baseline");
    let mut t_ms: u32 = 0;
    for _ in 0..(BASELINE_MS / STEP_MS) {
        emit_csv!(t_ms, 0);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Enable drivers ──────────────────────────────────────────────────
    fr.enable(); fl.enable();
    cr.enable(); cl.enable();
    rr.enable(); rl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# drivers_enabled");

    // ── Fase 2: ramp_up ─────────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET {
        duty += RAMP_STEP;
        if duty > RAMP_TARGET { duty = RAMP_TARGET; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty);
        if !rl_cut { rl.set_speed(duty); }
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Fase 3: hold ────────────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    for _ in 0..(HOLD_MS / STEP_MS) {
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Fase 4: ramp_down ───────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        fr.set_speed(duty); fl.set_speed(duty);
        cr.set_speed(duty); cl.set_speed(duty);
        rr.set_speed(duty);
        if !rl_cut { rl.set_speed(duty); }
        emit_csv!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    // ── Stop + disable ──────────────────────────────────────────────────
    fr.stop(); fl.stop();
    cr.stop(); cl.stop();
    rr.stop(); rl.stop();
    fr.disable(); fl.disable();
    cr.disable(); cl.disable();
    rr.disable(); rl.disable();
    let _ = ufmt::uwriteln!(&mut serial,
        "# drivers_disabled (rl_cut_by_oc={})",
        if rl_cut { 1u8 } else { 0u8 });

    // ── Fase 5: idle ────────────────────────────────────────────────────
    let _ = ufmt::uwriteln!(&mut serial, "# event=idle");
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
