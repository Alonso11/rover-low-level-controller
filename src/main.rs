#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::IntoPwmPin;

mod motor_control;
use motor_control::{Motor, HBridge, DriveTrain};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // --- Configuracion de Timers para PWM ---
    // En el Mega 2560, el Timer 3 maneja los pines PWM 2, 3 y 5.
    let timer3 = arduino_hal::simple_pwm::Timer3Pwm::new(dp.TC3, arduino_hal::simple_pwm::Prescaler::Prescale64);

    // --- Configuracion de Motores ---
    // Motor Izquierdo: PWM pin 2, IN1 pin 22, IN2 pin 23
    let left_motor = HBridge::new(
        pins.d2.into_output().into_pwm(&timer3),
        pins.d22.into_output(),
        pins.d23.into_output(),
    );

    // Motor Derecho: PWM pin 3, IN1 pin 24, IN2 pin 25
    let right_motor = HBridge::new(
        pins.d3.into_output().into_pwm(&timer3),
        pins.d24.into_output(),
        pins.d25.into_output(),
    );

    // --- Crear el Sistema de Traccion ---
    let mut rover = DriveTrain::new(left_motor, right_motor);

    // Pin de LED para estado
    let mut led = pins.d13.into_output();

    loop {
        led.toggle();

        // 1. Mover adelante a media velocidad
        rover.drive(150, 150);
        arduino_hal::delay_ms(2000);

        // 2. Girar a la derecha (sobre su eje)
        rover.drive(150, -150);
        arduino_hal::delay_ms(1000);

        // 3. Detenerse
        rover.stop();
        arduino_hal::delay_ms(2000);
    }
}

#[no_mangle]
pub extern "C" fn _exit() -> ! { loop {} }
#[no_mangle]
pub extern "C" fn exit() -> ! { loop {} }
