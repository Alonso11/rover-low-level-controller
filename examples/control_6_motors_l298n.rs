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
    
    // Configuración de la interfaz serie para recibir comandos de la RPi 5
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    // Inicialización de Timers para PWM
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
    
    // --- PUENTE H 1 (Motores Frontales) ---
    let fr = L298NMotor::new(pins.d10.into_output().into_pwm(&mut timer2), pins.d22.into_output(), pins.d23.into_output(), false);
    let fl = L298NMotor::new(pins.d9.into_output().into_pwm(&mut timer2), pins.d24.into_output(), pins.d25.into_output(), false);

    // --- PUENTE H 2 (Motores Centrales) ---
    let cr = L298NMotor::new(pins.d5.into_output().into_pwm(&mut timer3), pins.d26.into_output(), pins.d27.into_output(), false);
    let cl = L298NMotor::new(pins.d2.into_output().into_pwm(&mut timer3), pins.d28.into_output(), pins.d29.into_output(), false);

    // --- PUENTE H 3 (Motores Traseros) ---
    let rr = L298NMotor::new(pins.d6.into_output().into_pwm(&mut timer4), pins.d30.into_output(), pins.d31.into_output(), false);
    let rl = L298NMotor::new(pins.d7.into_output().into_pwm(&mut timer4), pins.d32.into_output(), pins.d33.into_output(), false);

    // Chasis unificado de 6 ruedas
    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    interface.log("Control de Rover de 6 Ruedas Listo (Protocolo L298N)");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => { // ADELANTE
                        rover.set_speeds(80, 80);
                        interface.log("Rover: ADELANTE");
                    },
                    b'B' | b'b' => { // ATRAS
                        rover.set_speeds(-80, -80);
                        interface.log("Rover: ATRAS");
                    },
                    b'L' | b'l' => { // GIRO IZQUIERDA
                        rover.set_speeds(-80, 80);
                        interface.log("Rover: GIRO IZQ");
                    },
                    b'R' | b'r' => { // GIRO DERECHA
                        rover.set_speeds(80, -80);
                        interface.log("Rover: GIRO DER");
                    },
                    b'S' | b's' => { // STOP
                        rover.stop();
                        interface.log("Rover: STOP");
                    },
                    _ => {
                        interface.log("Error: Comando desconocido");
                    }
                }
            }
        }
    }
}
