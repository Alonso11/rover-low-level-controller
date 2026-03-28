// Version: v1.1
//! Test básico del driver BTS7960 (IBT-2).
//!
//! ## Conexiones
//!
//! | IBT-2 | Arduino Mega |
//! |-------|--------------|
//! | RPWM  | D9  (OC2B)   |
//! | LPWM  | D10 (OC2A)   |
//! | R_EN  | D22          |
//! | L_EN  | D23          |
//! | VCC   | 5V           |
//! | GND   | GND          |
//! | B+/B- | Batería      |
//! | M+/M- | Motor        |

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);

    let rpwm = pins.d9.into_output().into_pwm(&mut timer2);
    let lpwm = pins.d10.into_output().into_pwm(&mut timer2);
    let r_en = pins.d22.into_output();
    let l_en = pins.d23.into_output();

    let mut motor = BTS7960Motor::new(rpwm, lpwm, r_en, l_en, false);

    loop {
        motor.set_speed(100);
        arduino_hal::delay_ms(2000);

        motor.stop();
        arduino_hal::delay_ms(1000);

        motor.set_speed(-50);
        arduino_hal::delay_ms(2000);

        motor.stop();
        arduino_hal::delay_ms(1000);
    }
}
