// Version: v1.4 (SAFE TEST)
//! Test seguro del driver BTS7960 (IBT-2) con rampa de velocidad.
//!
//! ## Conexiones (Mismo pinout que el relay-bank en main.rs para facilitar pruebas)
//!
//! | IBT-2 | Arduino Mega | Función             |
//! |-------|--------------|---------------------|
//! | RPWM  | D9  (OC2B)   | PWM Adelante        |
//! | LPWM  | D10 (OC2A)   | PWM Atrás           |
//! | R_EN  | D40          | Enable Canal R      |
//! | L_EN  | D41          | Enable Canal L      |
//! | VCC   | 5V           | Lógica              |
//! | GND   | GND          | Tierra              |
//! | B+/B- | Batería      | Potencia (12V-24V)  |
//! | M+/M- | Motor        | Salida Motor        |

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm};
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;
use rover_low_level_controller::ramp::DriveRamp;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Timer2 controla D9 y D10 en el Mega
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);

    let rpwm = pins.d9.into_output().into_pwm(&mut timer2);
    let lpwm = pins.d10.into_output().into_pwm(&mut timer2);
    
    // Pines de habilitación. En el IBT-2, ambos deben estar en HIGH para que funcione.
    let r_en = pins.d40.into_output();
    let l_en = pins.d41.into_output();

    let mut motor = BTS7960Motor::new(rpwm, lpwm, r_en, l_en, false);
    let mut ramp = DriveRamp::new();

    // Habilitamos el driver explícitamente después de la configuración.
    motor.enable();
    
    // Paso de rampa: 5 unidades cada 20ms (tarda 400ms en llegar de 0 a 100%)
    // Esto es muy seguro para los MOSFETs del BTS7960.
    const STEP: i16 = 5; 

    loop {
        // --- ADELANTE SUAVE (0 a 100%) ---
        // Se llama a ramp.step cada 20ms hasta llegar al target.
        while !ramp.at_target(100, 0) {
            let (target, _) = ramp.step(100, 0, STEP);
            motor.set_speed(target);
            arduino_hal::delay_ms(20);
        }

        // Mantener velocidad máxima 2 segundos
        arduino_hal::delay_ms(2000);

        // --- FRENADO SUAVE (100% a 0%) ---
        while !ramp.at_target(0, 0) {
            let (target, _) = ramp.step(0, 0, STEP);
            motor.set_speed(target);
            arduino_hal::delay_ms(20);
        }

        // Pausa en cero
        arduino_hal::delay_ms(1000);

        // --- ATRÁS SUAVE (0 a -100%) ---
        while !ramp.at_target(-100, 0) {
            let (target, _) = ramp.step(-100, 0, STEP);
            motor.set_speed(target);
            arduino_hal::delay_ms(20);
        }

        // Mantener reversa 2 segundos
        arduino_hal::delay_ms(2000);

        // --- FRENADO SUAVE (-100% a 0%) ---
        while !ramp.at_target(0, 0) {
            let (target, _) = ramp.step(0, 0, STEP);
            motor.set_speed(target);
            arduino_hal::delay_ms(20);
        }
        
        // Pausa larga antes de reiniciar ciclo
        arduino_hal::delay_ms(3000);
    }
}
