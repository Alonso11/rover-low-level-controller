// Version: v2.0
//! # Firmware Principal — Rover Olympus / Arduino Mega 2560
//!
//! ## Loop principal (20 ms / ciclo):
//!   1. `msm.tick()`           — watchdog: sin PING en 100 ciclos (~2 s) → FAULT
//!   2. `iface.poll_command()` — lee trama ASCII desde USART3 (RPi5)
//!   3. `msm.process(cmd)`     — transición de estado + calcula DriveOutput
//!   4. `sync_drive!()`        — aplica DriveOutput a los 6 motores
//!   5. `iface.send_response()` — respuesta ASCII a RPi5
//!   6. Telemetría (~1 s)      — TLM:<SAFETY>:<MASK>
//!
//! ## Pines:
//!   - USART3 D14(TX3)/D15(RX3) → RPi5 @ 115200
//!   - Timer2 D9(FR) D10(FL) | Timer4 D8(CR) D7(CL) D6(RR) | Timer3 D5(RL)
//!   - Dirección motores: D22–D37

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer3Pwm, Timer4Pwm};
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::l298n::{L298NMotor, SixWheelRover};
use arduino_hal::prelude::*;
use rover_low_level_controller::state_machine::{
    format_response, parse_command, MasterStateMachine, Response, RoverState,
};

const TLM_PERIOD: u8 = 50;  // ciclos entre telemetría (~1 s)
const LOOP_MS: u32  = 20;
const RESP_BUF: usize = 24;

/// Aplica msm.drive al rover; en FAULT/STANDBY para motores.
macro_rules! sync_drive {
    ($rover:expr, $msm:expr) => {
        match $msm.state {
            RoverState::Fault | RoverState::Standby => $rover.stop(),
            _ => $rover.set_speeds($msm.drive.left, $msm.drive.right),
        }
    };
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // USART3 → RPi5
    let serial_rpi = arduino_hal::Usart::new(
        dp.USART3,
        pins.d15,
        pins.d14.into_output(),
        115200_u32.into_baudrate(),
    );
    let mut iface = CommandInterface::new(serial_rpi);

    // Timers PWM
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // 6 motores — layout verificado en control_6_motors_l298n.rs v3.0
    let fr = L298NMotor::new(pins.d9.into_output().into_pwm(&mut timer2),  pins.d23.into_output(), pins.d25.into_output(), false);
    let fl = L298NMotor::new(pins.d10.into_output().into_pwm(&mut timer2), pins.d22.into_output(), pins.d24.into_output(), false);
    let cr = L298NMotor::new(pins.d8.into_output().into_pwm(&mut timer4),  pins.d28.into_output(), pins.d30.into_output(), false);
    let cl = L298NMotor::new(pins.d7.into_output().into_pwm(&mut timer4),  pins.d29.into_output(), pins.d31.into_output(), false);
    let rr = L298NMotor::new(pins.d6.into_output().into_pwm(&mut timer4),  pins.d34.into_output(), pins.d35.into_output(), false);
    let rl = L298NMotor::new(pins.d5.into_output().into_pwm(&mut timer3),  pins.d36.into_output(), pins.d37.into_output(), false);
    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    let mut msm         = MasterStateMachine::new();
    let mut resp_buf    = [0u8; RESP_BUF];
    let mut tlm_counter: u8 = 0;

    iface.log("=== ROVER OLYMPUS v2.0 — MSM READY ===");

    loop {
        // 1. Watchdog
        if let Some(wdog_resp) = msm.tick() {
            sync_drive!(rover, msm);
            iface.send_response(format_response(wdog_resp, &mut resp_buf));
        }

        // 2. Comando entrante desde RPi5
        if iface.poll_command() {
            let response = match parse_command(iface.get_command()) {
                Some(cmd) => msm.process(cmd),
                None      => Response::ErrUnknown,
            };
            sync_drive!(rover, msm);
            iface.send_response(format_response(response, &mut resp_buf));
        }

        // 3. Telemetría periódica (~1 s)
        tlm_counter = tlm_counter.wrapping_add(1);
        if tlm_counter >= TLM_PERIOD {
            tlm_counter = 0;
            iface.send_response(format_response(msm.telemetry(0), &mut resp_buf));
        }

        arduino_hal::delay_ms(LOOP_MS);
    }
}
