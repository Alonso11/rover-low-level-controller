// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
// Usamos la librería del proyecto
use rover_low_level_controller::motor_control::Servo;
use rover_low_level_controller::motor_control::servo::StandardServo;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let servo_pin = pins.d11.into_output();
    let mut my_servo = StandardServo::new(servo_pin);

    loop {
        for angle in (0..=180).step_by(10) {
            for _ in 0..15 {
                my_servo.set_angle(angle);
                arduino_hal::delay_ms(18);
            }
        }

        for angle in (0..=180).rev().step_by(10) {
            for _ in 0..15 {
                my_servo.set_angle(angle);
                arduino_hal::delay_ms(18);
            }
        }
    }
}
