#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::IntoPwmPin;
mod motor_control;
use motor_control::Motor;
use motor_control::bts7960::BTS7960Motor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // --- CONFIGURACIÓN DE TIMERS ---
    // En el Mega, D9 y D10 comparten el Timer 2
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    
    // --- CONFIGURACIÓN BTS7960 (Test Motor Único) ---
    // RPWM (Giro horario) -> D9
    // LPWM (Giro antihorario) -> D10
    // R_EN / L_EN -> Deben conectarse a 5V externos o del Arduino
    let rpwm = pins.d9.into_output().into_pwm(&mut timer2);
    let lpwm = pins.d10.into_output().into_pwm(&mut timer2);

    let mut test_motor = BTS7960Motor::new(rpwm, lpwm, false);

    loop {
        // Adelante 100% (Solo activará RPWM)
        test_motor.set_speed(100);
        arduino_hal::delay_ms(2000);

        // Frenado
        test_motor.stop();
        arduino_hal::delay_ms(1000);

        // Atrás 50% (Solo activará LPWM)
        test_motor.set_speed(-50);
        arduino_hal::delay_ms(2000);

        // Frenado
        test_motor.stop();
        arduino_hal::delay_ms(1000);
    }
}
