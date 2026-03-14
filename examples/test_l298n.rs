// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::IntoPwmPin;
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    
    let motor_a_pwm = pins.d9.into_output().into_pwm(&mut timer2);
    let motor_a_in1 = pins.d8.into_output();
    let motor_a_in2 = pins.d7.into_output();
    let mut motor_right = L298NMotor::new(motor_a_pwm, motor_a_in1, motor_a_in2, false);

    let motor_b_pwm = pins.d10.into_output().into_pwm(&mut timer2);
    let motor_b_in3 = pins.d6.into_output();
    let motor_b_in4 = pins.d5.into_output();
    let mut motor_left = L298NMotor::new(motor_b_pwm, motor_b_in3, motor_b_in4, false);

    loop {
        let _ = motor_right.set_speed(70);
        let _ = motor_left.set_speed(70);
        arduino_hal::delay_ms(2000);

        let _ = motor_right.stop();
        let _ = motor_left.stop();
        arduino_hal::delay_ms(1000);
    }
}
