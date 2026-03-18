// Version: v1.3 - Timers Optimizados
//! # Ejemplo: Control de Rover de 6 Ruedas (L298N)
//!
//! Este programa controla un chasis de 6 ruedas utilizando 3 drivers L298N.
//! Utiliza **múltiples Timers de hardware** para evitar conflictos y asegurar
//! una generación de PWM estable e independiente de la CPU.
//!
//! ## Distribución de Timers y Pines:
//! * **Timer 1:** Motor Frontal Derecho (Pin D11)
//! * **Timer 2:** Motor Frontal Izquierdo (Pin D10)
//! * **Timer 3:** Motores Centrales (Pines D5, D2) - Canales A y B
//! * **Timer 4:** Motor Trasero Derecho (Pin D6)
//! * **Timer 5:** Motor Trasero Izquierdo (Pin D46)
//!
//! Comunicación: 115200 baudios vía Serial.

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Timer1Pwm, Timer2Pwm, Timer3Pwm, Timer4Pwm, Timer5Pwm, Prescaler};
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::l298n::{L298NMotor, SixWheelRover};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    // --- CONFIGURACIÓN DE TIMERS ---
    let mut timer1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut timer5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);
    
    // --- MOTOR 1: Frontal Derecho (Timer1) ---
    let fr = L298NMotor::new(
        pins.d11.into_output().into_pwm(&mut timer1),
        pins.d22.into_output(), 
        pins.d23.into_output(), 
        false
    );
    
    // --- MOTOR 2: Frontal Izquierdo (Timer2) ---
    let fl = L298NMotor::new(
        pins.d10.into_output().into_pwm(&mut timer2),
        pins.d24.into_output(), 
        pins.d25.into_output(), 
        false
    );

    // --- MOTOR 3: Central Derecho (Timer3 Canal A) ---
    let cr = L298NMotor::new(
        pins.d5.into_output().into_pwm(&mut timer3),
        pins.d26.into_output(), 
        pins.d27.into_output(), 
        false
    );
    
    // --- MOTOR 4: Central Izquierdo (Timer3 Canal B) ---
    let cl = L298NMotor::new(
        pins.d2.into_output().into_pwm(&mut timer3),
        pins.d28.into_output(), 
        pins.d29.into_output(), 
        false
    );

    // --- MOTOR 5: Trasero Derecho (Timer4) ---
    let rr = L298NMotor::new(
        pins.d6.into_output().into_pwm(&mut timer4),
        pins.d30.into_output(), 
        pins.d31.into_output(), 
        false
    );
    
    // --- MOTOR 6: Trasero Izquierdo (Timer5) ---
    let rl = L298NMotor::new(
        pins.d46.into_output().into_pwm(&mut timer5),
        pins.d32.into_output(), 
        pins.d33.into_output(), 
        false
    );

    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    interface.log("Rover 6 Motores - Timers Balanceados v1.3");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => {
                        rover.set_speeds(80, 80);
                        interface.log("AVANZANDO");
                    },
                    b'B' | b'b' => {
                        rover.set_speeds(-80, -80);
                        interface.log("RETROCEDIENDO");
                    },
                    b'L' | b'l' => {
                        rover.set_speeds(-80, 80);
                        interface.log("GIRANDO IZQ");
                    },
                    b'R' | b'r' => {
                        rover.set_speeds(80, -80);
                        interface.log("GIRANDO DER");
                    },
                    b'S' | b's' => {
                        rover.stop();
                        interface.log("DETENIDO");
                    },
                    _ => {
                        interface.log("Comando desconocido");
                    }
                }
            }
        }
    }
}
