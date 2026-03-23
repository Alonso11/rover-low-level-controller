// Version: v2.0 — Debug individual de motores (compatible con Python bridge)
//! # Debug: Control Individual de Motores L298N
//!
//! Cambia `MOTOR_TO_TEST` (1-6), recompila y flashea para probar cada motor.
//! Los comandos F/B/S del bridge controlan SOLO ese motor.
//!
//! ## Mapa de motores:
//! * 1 — D9  / OC2B / Timer2 — IN: D22, D23 — Frontal Derecho
//! * 2 — D10 / OC2A / Timer2 — IN: D24, D25 — Frontal Izquierdo
//! * 3 — D5  / OC3A / Timer3 — IN: D28, D29 — Central Derecho
//! * 4 — D6  / OC4A / Timer4 — IN: D30, D31 — Central Izquierdo
//! * 5 — D7  / OC4B / Timer4 — IN: D34, D35 — Trasero Derecho
//! * 6 — D8  / OC4C / Timer4 — IN: D36, D37 — Trasero Izquierdo

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Timer2Pwm, Timer3Pwm, Timer4Pwm, Prescaler};
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
use rover_low_level_controller::motor_control::Motor;

/// *** CAMBIA ESTE NÚMERO PARA PROBAR ***
/// 0 = todos los motores, 1-6 = motor individual
const MOTOR_TO_TEST: u8 = 0;

/// *** VELOCIDAD DE PRUEBA (0-100) ***
const SPEED: i16 = 50;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // M1: Frontal Derecho — D9/ENB/OC2B/Timer2 — Canal B → IN3=D23, IN4=D25
    let mut m1 = L298NMotor::new(
        pins.d9.into_output().into_pwm(&mut timer2),
        pins.d23.into_output(),  // IN3 (canal B)
        pins.d25.into_output(),  // IN4 (canal B)
        false,
    );

    // M2: Frontal Izquierdo — D10/ENA/OC2A/Timer2 — Canal A → IN1=D22, IN2=D24
    let mut m2 = L298NMotor::new(
        pins.d10.into_output().into_pwm(&mut timer2),
        pins.d22.into_output(),  // IN1 (canal A)
        pins.d24.into_output(),  // IN2 (canal A)
        false,
    );

    // M3: Central Derecho — D5/OC3A/Timer3
    let mut m3 = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut timer4),
        pins.d28.into_output(),
        pins.d30.into_output(),
        false,
    );

    // M4: Central Izquierdo — D6/OC4A/Timer4
    let mut m4 = L298NMotor::new(
        pins.d7.into_output().into_pwm(&mut timer4),
        pins.d29.into_output(),
        pins.d31.into_output(),
        false,
    );

    // M5: Trasero Derecho — D7/OC4B/Timer4
    let mut m5 = L298NMotor::new(
        pins.d6.into_output().into_pwm(&mut timer4),
        pins.d34.into_output(),
        pins.d35.into_output(),
        false,
    );

    // M6: Trasero Izquierdo — D8/OC4C/Timer4
    let mut m6 = L298NMotor::new(
        pins.d5.into_output().into_pwm(&mut timer3),
        pins.d36.into_output(),
        pins.d37.into_output(),
        false,
    );

    match MOTOR_TO_TEST {
        0 => interface.log("TEST TODOS los motores"),
        1 => interface.log("TEST M1: D9/OC2B/Timer2 (Frontal Der)"),
        2 => interface.log("TEST M2: D10/OC2A/Timer2 (Frontal Izq)"),
        3 => interface.log("TEST M3: D5/OC3A/Timer3 (Central Der)"),
        4 => interface.log("TEST M4: D6/OC4A/Timer4 (Central Izq)"),
        5 => interface.log("TEST M5: D7/OC4B/Timer4 (Trasero Der)"),
        6 => interface.log("TEST M6: D8/OC4C/Timer4 (Trasero Izq)"),
        _ => interface.log("ERR: MOTOR_TO_TEST invalido (0-6)"),
    }
    interface.log("Comandos: F=adelante B=atras S=stop");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            if cmd.is_empty() { continue; }

            let speed: i16 = match cmd[0] {
                b'F' | b'f' => SPEED,
                b'B' | b'b' => -SPEED,
                b'S' | b's' => 0,
                _ => { interface.log("ERR: usa F B S"); continue; }
            };

            match MOTOR_TO_TEST {
                0 => {
                    m1.set_speed(speed);
                    m2.set_speed(speed);
                    m3.set_speed(speed);
                    m4.set_speed(speed);
                    m5.set_speed(speed);
                    m6.set_speed(speed);
                    interface.log("TODOS OK");
                }
                1 => { m1.set_speed(speed); interface.log("M1 OK"); }
                2 => { m2.set_speed(speed); interface.log("M2 OK"); }
                3 => { m3.set_speed(speed); interface.log("M3 OK"); }
                4 => { m4.set_speed(speed); interface.log("M4 OK"); }
                5 => { m5.set_speed(speed); interface.log("M5 OK"); }
                6 => { m6.set_speed(speed); interface.log("M6 OK"); }
                _ => { interface.log("ERR: MOTOR_TO_TEST invalido"); }
            }
        }
    }
}
