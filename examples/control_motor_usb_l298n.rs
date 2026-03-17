// Version: v1.1 - Timers Independientes
#![no_std]
#![no_main]

use arduino_hal::simple_pwm::IntoPwmPin;
use panic_halt as _;
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::l298n::L298NMotor;
use rover_low_level_controller::motor_control::Motor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    // --- TIMERS INDEPENDIENTES ---
    // Timer2 para Motor Derecho
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(
        dp.TC2,
        arduino_hal::simple_pwm::Prescaler::Prescale64,
    );
    
    // Timer3 para Motor Izquierdo (evita conflictos)
    let mut timer3 = arduino_hal::simple_pwm::Timer3Pwm::new(
        dp.TC3,
        arduino_hal::simple_pwm::Prescaler::Prescale64,
    );

    // --- Motor Derecho ---
    // PWM: D10 (Timer2 - OC2A)
    // Dirección: D22, D23
    let motor_right_pwm = pins.d10.into_output().into_pwm(&mut timer2);
    let motor_right_in1 = pins.d22.into_output();
    let motor_right_in2 = pins.d23.into_output();
    let mut motor_right = L298NMotor::new(
        motor_right_pwm, 
        motor_right_in1, 
        motor_right_in2, 
        false
    );

    // --- Motor Izquierdo ---
    // PWM: D5 (Timer3 - OC3A) <- CAMBIO CLAVE: de D9 a D5
    // Dirección: D24, D25
    let motor_left_pwm = pins.d5.into_output().into_pwm(&mut timer3);
    let motor_left_in3 = pins.d24.into_output();
    let motor_left_in4 = pins.d25.into_output();
    let mut motor_left = L298NMotor::new(
        motor_left_pwm, 
        motor_left_in3, 
        motor_left_in4, 
        false
    );

    interface.log("Rover Olympus - 2 Motores con Timers Independientes");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => {
                        motor_right.set_speed(70);
                        motor_left.set_speed(70);
                        interface.log("ADELANTE");
                    }
                    b'B' | b'b' => {
                        motor_right.set_speed(-70);
                        motor_left.set_speed(-70);
                        interface.log("ATRAS");
                    }
                    b'L' | b'l' => {
                        motor_right.set_speed(70);
                        motor_left.set_speed(-70);
                        interface.log("GIRO IZQ");
                    }
                    b'R' | b'r' => {
                        motor_right.set_speed(-70);
                        motor_left.set_speed(70);
                        interface.log("GIRO DER");
                    }
                    b'S' | b's' => {
                        motor_right.stop();
                        motor_left.stop();
                        interface.log("STOP");
                    }
                    _ => {
                        interface.log("Comando desconocido");
                    }
                }
            }
        }
    }
}