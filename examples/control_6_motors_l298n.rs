// Version: v1.0
//! # Ejemplo: Control de Rover de 6 Ruedas (L298N)
//!
//! Este programa controla un chasis de 6 ruedas utilizando 3 drivers L298N.
//! Utiliza **múltiples Timers de hardware** para evitar conflictos y asegurar
//! una generación de PWM estable e independiente de la CPU.
//!
//! ## Distribución de Timers y Pines:
//! * **Timer 2 (8-bit):** Motores Frontales (Pines D10, D9)
//! * **Timer 3 (16-bit):** Motores Centrales (Pines D5, D2)
//! * **Timer 4 (16-bit):** Motores Traseros (Pines D6, D7)
//!
//! Comunicación: 115200 baudios vía Serial.

#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Timer2Pwm, Timer3Pwm, Timer4Pwm, Prescaler};
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::l298n::{L298NMotor, SixWheelRover};

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Inicialización del puerto serie para recibir comandos (ej: desde una Raspberry Pi)
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    // --- CONFIGURACIÓN DE TIMERS ---
    // Prescaler::Prescale64 configura la frecuencia de PWM a aprox. 1kHz (f_CPU / (64 * 256))
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    
    // --- PUENTE H 1 (Motores Frontales) ---
    // Lado Derecho: PWM D10, Dir D22/D23
    let fr = L298NMotor::new(pins.d10.into_output().into_pwm(&mut timer2), pins.d22.into_output(), pins.d23.into_output(), false);
    // Lado Izquierdo: PWM D9, Dir D24/D25
    let fl = L298NMotor::new(pins.d9.into_output().into_pwm(&mut timer2), pins.d24.into_output(), pins.d25.into_output(), false);

    // --- PUENTE H 2 (Motores Centrales) ---
    // Lado Derecho: PWM D5, Dir D26/D27
    let cr = L298NMotor::new(pins.d5.into_output().into_pwm(&mut timer3), pins.d26.into_output(), pins.d27.into_output(), false);
    // Lado Izquierdo: PWM D2, Dir D28/D29
    let cl = L298NMotor::new(pins.d2.into_output().into_pwm(&mut timer3), pins.d28.into_output(), pins.d29.into_output(), false);

    // --- PUENTE H 3 (Motores Traseros) ---
    // Lado Derecho: PWM D6, Dir D30/D31
    let rr = L298NMotor::new(pins.d6.into_output().into_pwm(&mut timer4), pins.d30.into_output(), pins.d31.into_output(), false);
    // Lado Izquierdo: PWM D7, Dir D32/D33
    let rl = L298NMotor::new(pins.d7.into_output().into_pwm(&mut timer4), pins.d32.into_output(), pins.d33.into_output(), false);

    // Creación del objeto Rover unificado
    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    interface.log("Sistema Rover Olympus Iniciado - Control de 6 Motores Listo");

    loop {
        // Escucha de comandos entrantes
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => { // Comando: Adelante
                        rover.set_speeds(80, 80);
                        interface.log("Estado: AVANZANDO");
                    },
                    b'B' | b'b' => { // Comando: Atrás
                        rover.set_speeds(-80, -80);
                        interface.log("Estado: RETROCEDIENDO");
                    },
                    b'L' | b'l' => { // Comando: Giro Izquierda (Tanque)
                        rover.set_speeds(-80, 80);
                        interface.log("Estado: GIRANDO IZQ");
                    },
                    b'R' | b'r' => { // Comando: Giro Derecha (Tanque)
                        rover.set_speeds(80, -80);
                        interface.log("Estado: GIRANDO DER");
                    },
                    b'S' | b's' => { // Comando: Parada de Emergencia
                        rover.stop();
                        interface.log("Estado: DETENIDO");
                    },
                    _ => {
                        interface.log("Error: Comando no reconocido");
                    }
                }
            }
        }
    }
}
