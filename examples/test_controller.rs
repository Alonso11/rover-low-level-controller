// Version: v1.1
//! # Ejemplo de Prueba del Controlador Central (AnyPin)
//!
//! Este ejemplo configura los 6 motores L298N usando AnyPin para que todos
//! tengan el mismo tipo y puedan guardarse en un solo array.

#![no_std]
#![no_main]

use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::IntoPwmPin;
use panic_halt as _;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
use rover_low_level_controller::sensors::encoder::HallEncoder;
use rover_low_level_controller::controller::{RoverController, DriveChannel};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    let timer3 = arduino_hal::simple_pwm::Timer3Pwm::new(dp.TC3, arduino_hal::simple_pwm::Prescaler::Prescale64);
    let timer4 = arduino_hal::simple_pwm::Timer4Pwm::new(dp.TC4, arduino_hal::simple_pwm::Prescaler::Prescale64);

    let m0 = L298NMotor::new(pins.d10.into_output().into_pwm(&timer2), pins.d22.into_output().downgrade(), pins.d23.into_output().downgrade(), false);
    let m1 = L298NMotor::new(pins.d9.into_output().into_pwm(&timer2), pins.d24.into_output().downgrade(), pins.d25.into_output().downgrade(), false);
    let m2 = L298NMotor::new(pins.d5.into_output().into_pwm(&timer3), pins.d26.into_output().downgrade(), pins.d27.into_output().downgrade(), false);
    let m3 = L298NMotor::new(pins.d2.into_output().into_pwm(&timer3), pins.d28.into_output().downgrade(), pins.d29.into_output().downgrade(), false);
    let m4 = L298NMotor::new(pins.d6.into_output().into_pwm(&timer4), pins.d30.into_output().downgrade(), pins.d31.into_output().downgrade(), false);
    let m5 = L298NMotor::new(pins.d7.into_output().into_pwm(&timer4), pins.d32.into_output().downgrade(), pins.d33.into_output().downgrade(), false);

    let channels = [
        DriveChannel::new(m0, HallEncoder::new()),
        DriveChannel::new(m1, HallEncoder::new()),
        DriveChannel::new(m2, HallEncoder::new()),
        DriveChannel::new(m3, HallEncoder::new()),
        DriveChannel::new(m4, HallEncoder::new()),
        DriveChannel::new(m5, HallEncoder::new()),
    ];

    let mut rover = RoverController::new(channels);

    ufmt::uwriteln!(&mut serial, "Controlador Rover 6WD Iniciado con AnyPin.\r").void_unwrap();

    loop {
        rover.tank_drive(30, 30);
        for _ in 0..10 {
            rover.update([30, 30, 30, 30, 30, 30]);
            if rover.emergency_stop {
                ufmt::uwriteln!(&mut serial, "¡EMERGENCIA! Atasco detectado.\r").void_unwrap();
            }
            arduino_hal::delay_ms(20);
        }
    }
}
