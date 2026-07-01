// Version: v1.0 — diagnóstico aislado RL con L298N
//! Test mínimo del motor RL únicamente, para descartar si el motor está dañado
//! o si las lecturas raras de la sesión 2026-05-27 eran crosstalk PWM de los
//! otros 5 motores.
//!
//! **Cero ruido PWM:** los otros 5 motores NO se inicializan en este ejemplo
//! (sus drivers BTS7960 quedan deshabilitados por hardware — R_EN/L_EN sin
//! configurar = LOW por defecto del Mega).
//!
//! ## Qué se observa
//! - `i_rl` real (sin contaminación PWM del bus) durante baseline, rampa, hold.
//! - `t_rl` (encoder) → si avanza, el motor gira; si no, está atascado o el
//!   encoder está mal.
//! - OC watchdog dispara solo si la corriente real excede 1500 mA.
//!
//! ## Cableado
//! Idéntico al del ejemplo `test_motors_encoders_acs_rl_l298n.rs` pero solo
//! para el motor RL:
//! - L298N: ENA=D8 (PWM Timer4), IN1=D36, IN2=D37
//! - Encoder RL: A=D3 (INT5), B=D49 (PL0), VCC=5V, GND
//! - ACS712 RL: VIOUT=A5, VCC=5V, GND, paso de corriente por IP+/IP-
//!
//! Los demás motores y sensores **no se inicializan**.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer4Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
use rover_low_level_controller::sensors::{ACS712, Encoder, QuadratureEncoder};

static ENCODER_RL: QuadratureEncoder = QuadratureEncoder::new();

const PINE_ADDR: *const u8 = 0x2C  as *const u8;
const PINL_ADDR: *const u8 = 0x109 as *const u8;

#[avr_device::interrupt(atmega2560)]
fn INT5() {
    let pine = unsafe { core::ptr::read_volatile(PINE_ADDR) };
    let pinl = unsafe { core::ptr::read_volatile(PINL_ADDR) };
    ENCODER_RL.on_edge((pine & (1 << 5)) != 0, (pinl & (1 << 0)) != 0);
}

// Duty muy conservador. Si el motor está OK debería moverse con 25 %. Si está
// dañado o atascado, la corriente subirá por encima del OC antes de llegar
// a este valor.
const RAMP_TARGET:     i16 = 25;
const RAMP_STEP:       i16 = 1;
const STEP_MS:         u16 = 60;
const HOLD_MS:         u16 = 5000;
const BASELINE_MS:     u16 = 600;
const IDLE_MS:         u16 = 3000;
const ADC_AVG_SAMPLES: u8  = 8;   // más muestras → menos ruido (no hay PWM bus)

const OC_RL_LIMIT_MA:  i32 = 1500;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# test_only_rl_l298n v1.0");
    let _ = ufmt::uwriteln!(&mut serial,
        "# variant=rl-only-l298n duty_target={} hold_ms={} oc_rl_ma={}",
        RAMP_TARGET, HOLD_MS, OC_RL_LIMIT_MA);
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_ms,duty,t_rl,i_rl,adc_rl_raw");

    // Solo Timer4 (necesario para D8 ENA). Sin Timer0/1/2/3/5 → menos ruido.
    let mut t4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    let mut rl = L298NMotor::new(
        pins.d8 .into_output().into_pwm(&mut t4),
        pins.d36.into_output(), pins.d37.into_output(), false,
    );

    // Encoder RL: fase A en D3/INT5, fase B en D49/PL0.
    let _enc_rl_a = pins.d3 .into_pull_up_input();
    let _enc_rl_b = pins.d49.into_pull_up_input();

    // EIMSK bit 5 = INT5. EICRB ISC51=0 ISC50=1 → any-edge (bits 2:3 → 01).
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x04) }); // 0b00000100 — any-edge INT5
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x20) }); // 0b00100000 — INT5 enable

    // ACS712 RL en A5 con auto-calibración.
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_rl_pin = pins.a5.into_analog_input(&mut adc);

    let _ = ufmt::uwriteln!(&mut serial, "# event=acs_calibration");
    const CAL_SAMPLES: u8 = 64;
    let mut cal_sum: u32 = 0;
    for _ in 0..CAL_SAMPLES {
        cal_sum += acs_rl_pin.analog_read(&mut adc) as u32;
        arduino_hal::delay_ms(5);
    }
    let adc_avg = cal_sum / CAL_SAMPLES as u32;
    let zero_mv = ((adc_avg * 5000) / 1023) as i32;
    let acs_rl = ACS712::new_20a().calibrate_zero(zero_mv);
    let _ = ufmt::uwriteln!(&mut serial,
        "# acs_rl zero_mv={} adc_avg={}", zero_mv, adc_avg);

    unsafe { avr_device::interrupt::enable() };

    let mut rl_cut: bool = false;

    macro_rules! emit_csv_and_check_oc {
        ($t_ms:expr, $duty:expr) => {{
            let mut sum: u32 = 0;
            for _ in 0..ADC_AVG_SAMPLES {
                sum += acs_rl_pin.analog_read(&mut adc) as u32;
            }
            let adc_raw = (sum / ADC_AVG_SAMPLES as u32) as u16;
            let ma = acs_rl.read_ma(adc_raw);
            if !rl_cut && ma.abs() > OC_RL_LIMIT_MA {
                rl.set_speed(0);
                rl_cut = true;
                let _ = ufmt::uwriteln!(&mut serial,
                    "# event=oc_cut_rl ma_rl={} threshold={}", ma, OC_RL_LIMIT_MA);
            }
            let ticks = ENCODER_RL.get_counts();
            let _ = ufmt::uwriteln!(&mut serial,
                "{},{},{},{},{}", $t_ms, $duty, ticks, ma, adc_raw);
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

    rl.enable();
    let _ = ufmt::uwriteln!(&mut serial, "# rl_enabled");

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_up");
    let mut duty: i16 = 0;
    while duty < RAMP_TARGET && !rl_cut {
        duty += RAMP_STEP;
        rl.set_speed(duty);
        emit_csv_and_check_oc!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=hold");
    for _ in 0..(HOLD_MS / STEP_MS) {
        emit_csv_and_check_oc!(t_ms, if rl_cut { 0 } else { duty });
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    let _ = ufmt::uwriteln!(&mut serial, "# event=ramp_down");
    while duty > 0 {
        duty -= RAMP_STEP;
        if duty < 0 { duty = 0; }
        if !rl_cut { rl.set_speed(duty); }
        emit_csv_and_check_oc!(t_ms, duty);
        arduino_hal::delay_ms(STEP_MS as u32);
        t_ms += STEP_MS as u32;
    }

    rl.stop();
    rl.disable();
    let _ = ufmt::uwriteln!(&mut serial,
        "# rl_disabled (rl_cut_by_oc={})", if rl_cut { 1u8 } else { 0u8 });

    let _ = ufmt::uwriteln!(&mut serial, "# event=idle");
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
