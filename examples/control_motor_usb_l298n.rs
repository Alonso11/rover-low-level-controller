#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::IntoPwmPin;
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::l298n::L298NMotor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    
    let motor_a_pwm = pins.d9.into_output().into_pwm(&mut timer2);
    let motor_a_in1 = pins.d8.into_output();
    let motor_a_in2 = pins.d7.into_output();
    let mut motor_right = L298NMotor::new(motor_a_pwm, motor_a_in1, motor_a_in2, false);

    let motor_b_pwm = pins.d10.into_output().into_pwm(&mut timer2);
    let motor_b_in3 = pins.d6.into_output();
    let motor_b_in4 = pins.d5.into_output();
    let mut motor_left = L298NMotor::new(motor_b_pwm, motor_b_in3, motor_b_in4, false);

    interface.log("Rover Olympus USB listo (F, B, L, R, S)");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => {
                        let _ = motor_right.set_speed(70);
                        let _ = motor_left.set_speed(70);
                        interface.log("Ejecutando: ADELANTE");
                    },
                    b'B' | b'b' => {
                        let _ = motor_right.set_speed(-70);
                        let _ = motor_left.set_speed(-70);
                        interface.log("Ejecutando: ATRAS");
                    },
                    b'L' | b'l' => {
                        let _ = motor_right.set_speed(70);
                        let _ = motor_left.set_speed(-70);
                        interface.log("Ejecutando: GIRO IZQ");
                    },
                    b'R' | b'r' => {
                        let _ = motor_right.set_speed(-70);
                        let _ = motor_left.set_speed(70);
                        interface.log("Ejecutando: GIRO DER");
                    },
                    b'S' | b's' => {
                        let _ = motor_right.stop();
                        let _ = motor_left.stop();
                        interface.log("Ejecutando: STOP");
                    },
                    _ => {
                        interface.log("Error: Comando desconocido");
                    }
                }
            }
        }
    }
}
