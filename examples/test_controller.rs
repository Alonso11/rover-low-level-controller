// Version: v1.2
//! # RoverController Test Example
//!
//! Configures 6 L298N motors across 3 different hardware timers and stores
//! them in a `[DriveChannel<ErasedMotor, HallEncoder>; 6]` array using
//! type erasure. See `docs/consideration_implementation.md` for rationale.

#![no_std]
#![no_main]

use arduino_hal::simple_pwm::IntoPwmPin;
use panic_halt as _;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
use rover_low_level_controller::motor_control::ErasedMotor;
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

    // Motors are stack-allocated here and live for the entire program (main -> !),
    // satisfying the lifetime invariant of ErasedMotor::new.
    let mut m0 = L298NMotor::new(pins.d10.into_output().into_pwm(&timer2), pins.d22.into_output().downgrade(), pins.d23.into_output().downgrade(), false);
    let mut m1 = L298NMotor::new(pins.d9.into_output().into_pwm(&timer2),  pins.d24.into_output().downgrade(), pins.d25.into_output().downgrade(), false);
    let mut m2 = L298NMotor::new(pins.d5.into_output().into_pwm(&timer3),  pins.d26.into_output().downgrade(), pins.d27.into_output().downgrade(), false);
    let mut m3 = L298NMotor::new(pins.d2.into_output().into_pwm(&timer3),  pins.d28.into_output().downgrade(), pins.d29.into_output().downgrade(), false);
    let mut m4 = L298NMotor::new(pins.d6.into_output().into_pwm(&timer4),  pins.d30.into_output().downgrade(), pins.d31.into_output().downgrade(), false);
    let mut m5 = L298NMotor::new(pins.d7.into_output().into_pwm(&timer4),  pins.d32.into_output().downgrade(), pins.d33.into_output().downgrade(), false);

    // SAFETY: all motors are stack-allocated in main() -> ! and will never
    // go out of scope, satisfying ErasedMotor's lifetime invariant.
    let channels = unsafe { [
        DriveChannel::new(ErasedMotor::new(&mut m0), HallEncoder::new()),
        DriveChannel::new(ErasedMotor::new(&mut m1), HallEncoder::new()),
        DriveChannel::new(ErasedMotor::new(&mut m2), HallEncoder::new()),
        DriveChannel::new(ErasedMotor::new(&mut m3), HallEncoder::new()),
        DriveChannel::new(ErasedMotor::new(&mut m4), HallEncoder::new()),
        DriveChannel::new(ErasedMotor::new(&mut m5), HallEncoder::new()),
    ]};

    let mut rover = RoverController::new(channels);

    ufmt::uwriteln!(&mut serial, "Rover 6WD Controller started.\r").unwrap();

    loop {
        rover.tank_drive(30, 30);
        for _ in 0..10 {
            rover.update([30, 30, 30, 30, 30, 30]);
            if rover.emergency_stop {
                ufmt::uwriteln!(&mut serial, "EMERGENCY: stall detected.\r").unwrap();
            }
            arduino_hal::delay_ms(20);
        }
    }
}
