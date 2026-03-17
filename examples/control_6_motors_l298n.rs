// Version: v1.1 - Timers Optimizados
//! # Ejemplo: Control de Rover de 6 Ruedas (L298N)
//!
//! Usa timers independientes para evitar conflictos de PWM.
//!
//! ## Distribución de Timers y Pines (OPTIMIZADA):
//! * **Timer 1:** Motor Frontal Derecho (Pin D11)
//! * **Timer 2:** Motor Frontal Izquierdo (Pin D10)
//! * **Timer 3:** Motor Central Derecho (Pin D5)
//! * **Timer 4:** Motores Central Izquierdo + Trasero Derecho (Pines D6, D7)
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

    // --- CONFIGURACIÓN DE TIMERS (5 timers para 6 motores) ---
    let mut timer1 = Timer1Pwm::new(dp.TC1, Prescaler::Prescale64);
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    let mut timer5 = Timer5Pwm::new(dp.TC5, Prescaler::Prescale64);
    
    // --- PUENTE H 1 (Motores Frontales) - TIMERS INDEPENDIENTES ---
    // Frontal Derecho: PWM D11 (Timer1), Dir D22/D23
    let fr = L298NMotor::new(
        pins.d11.into_output().into_pwm(&mut timer1), 
        pins.d22.into_output(), 
        pins.d23.into_output(), 
        false
    );
    
    // Frontal Izquierdo: PWM D10 (Timer2), Dir D24/D25
    let fl = L298NMotor::new(
        pins.d10.into_output().into_pwm(&mut timer2), 
        pins.d24.into_output(), 
        pins.d25.into_output(), 
        false
    );

    // --- PUENTE H 2 (Motores Centrales) ---
    // Central Derecho: PWM D5 (Timer3), Dir D26/D27
    let cr = L298NMotor::new(
        pins.d5.into_output().into_pwm(&mut timer3), 
        pins.d26.into_output(), 
        pins.d27.into_output(), 
        false
    );
    
    // Central Izquierdo: PWM D6 (Timer4 Canal A), Dir D28/D29
    let cl = L298NMotor::new(
        pins.d6.into_output().into_pwm(&mut timer4), 
        pins.d28.into_output(), 
        pins.d29.into_output(), 
        false
    );

    // --- PUENTE H 3 (Motores Traseros) ---
    // Trasero Derecho: PWM D7 (Timer4 Canal B - comparte timer con cl), Dir D30/D31
    let rr = L298NMotor::new(
        pins.d7.into_output().into_pwm(&mut timer4), 
        pins.d30.into_output(), 
        pins.d31.into_output(), 
        false
    );
    
    // Trasero Izquierdo: PWM D46 (Timer5), Dir D32/D33
    let rl = L298NMotor::new(
        pins.d46.into_output().into_pwm(&mut timer5), 
        pins.d32.into_output(), 
        pins.d33.into_output(), 
        false
    );

    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    interface.log("Rover Olympus - 6 Motores (Timers Optimizados)");

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
                        interface.log("Comando no reconocido");
                    }
                }
            }
        }
    }
}