// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::IntoPwmPin;
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    
    let rpwm = pins.d9.into_output().into_pwm(&mut timer2);
    let lpwm = pins.d10.into_output().into_pwm(&mut timer2);

    let mut test_motor = BTS7960Motor::new(rpwm, lpwm, false);

    loop {
        test_motor.set_speed(100);
        arduino_hal::delay_ms(2000);

        test_motor.stop();
        arduino_hal::delay_ms(1000);

        test_motor.set_speed(-50);
        arduino_hal::delay_ms(2000);

        test_motor.stop();
        arduino_hal::delay_ms(1000);
    }
}
