#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::IntoPwmPin;
use ufmt::uwriteln;

mod motor_control;
mod command_interface;
mod sensors;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // --- Serial (Comunicacion con Pi 5) ---
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = uwriteln!(serial, "Rover Low-Level Controller [Rust v0.3]");

    // --- Motores ---
    let timer3 = arduino_hal::simple_pwm::Timer3Pwm::new(dp.TC3, arduino_hal::simple_pwm::Prescaler::Prescale64);
    
    let mut left_pwm = pins.d2.into_output().into_pwm(&timer3);
    let mut left_in1 = pins.d22.into_output();
    let mut left_in2 = pins.d23.into_output();

    let mut right_pwm = pins.d3.into_output().into_pwm(&timer3);
    let mut right_in1 = pins.d24.into_output();
    let mut right_in2 = pins.d25.into_output();

    left_pwm.enable();
    right_pwm.enable();

    // --- Sensores (Ejemplo HC-SR04) ---
    let mut trig = pins.d4.into_output();
    let echo = pins.d5.into_pull_up_input();

    // --- LED de Estado ---
    let mut led = pins.d13.into_output();

    loop {
        // Interfaz de comandos simple
        if let Ok(byte) = serial.read() {
            match byte {
                b'M' => {
                    left_in1.set_high(); left_in2.set_low(); left_pwm.set_duty(150);
                    right_in1.set_high(); right_in2.set_low(); right_pwm.set_duty(150);
                    let _ = uwriteln!(serial, "ACK: Moving");
                }
                b'S' => {
                    left_pwm.set_duty(0); right_pwm.set_duty(0);
                    let _ = uwriteln!(serial, "ACK: Stopped");
                }
                b'D' => {
                    // Simular lectura de distancia
                    trig.set_low();
                    arduino_hal::delay_us(2);
                    trig.set_high();
                    arduino_hal::delay_us(10);
                    trig.set_low();
                    
                    // En un sistema real aqui mediriamos el pulso de echo
                    let _ = uwriteln!(serial, "DIST: 42cm (Mock)");
                }
                _ => { let _ = uwriteln!(serial, "ERR: Unknown"); }
            }
            led.toggle();
        }
    }
}

#[no_mangle]
pub extern "C" fn _exit() -> ! { loop {} }
#[no_mangle]
pub extern "C" fn exit() -> ! { loop {} }
