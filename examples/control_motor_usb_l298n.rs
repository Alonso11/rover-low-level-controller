// Version: v1.0
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

    // Configuración de Timers para el Puente H Frontal (Timer 2)
    // Usamos Timer2 porque controla los motores frontales en el diseño del Rover de 6 ruedas.
    // Esto asegura compatibilidad con el hardware real.
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(
        dp.TC2,
        arduino_hal::simple_pwm::Prescaler::Prescale64,
    );

    // --- Motor Derecho (Frontal) ---
    // PWM: D10 (OC2A)
    // Dirección: D22, D23
    let motor_right_pwm = pins.d10.into_output().into_pwm(&mut timer2);
    let motor_right_in1 = pins.d22.into_output();
    let motor_right_in2 = pins.d23.into_output();
    let mut motor_right = L298NMotor::new(motor_right_pwm, motor_right_in1, motor_right_in2, false);

    // --- Motor Izquierdo (Frontal) ---
    // PWM: D9 (OC2B)
    // Dirección: D24, D25
    let motor_left_pwm = pins.d9.into_output().into_pwm(&mut timer2);
    let motor_left_in3 = pins.d24.into_output();
    let motor_left_in4 = pins.d25.into_output();
    let mut motor_left = L298NMotor::new(motor_left_pwm, motor_left_in3, motor_left_in4, false);

    interface.log("Rover Olympus USB listo (Motores Frontales)");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();

            if cmd.len() > 0 {
                match cmd[0] {
                    b'F' | b'f' => {
                        let _ = motor_right.set_speed(70);
                        let _ = motor_left.set_speed(70);
                        interface.log("Ejecutando: ADELANTE");
                    }
                    b'B' | b'b' => {
                        let _ = motor_right.set_speed(-70);
                        let _ = motor_left.set_speed(-70);
                        interface.log("Ejecutando: ATRAS");
                    }
                    b'L' | b'l' => {
                        let _ = motor_right.set_speed(70);
                        let _ = motor_left.set_speed(-70);
                        interface.log("Ejecutando: GIRO IZQ");
                    }
                    b'R' | b'r' => {
                        let _ = motor_right.set_speed(-70);
                        let _ = motor_left.set_speed(70);
                        interface.log("Ejecutando: GIRO DER");
                    }
                    b'S' | b's' => {
                        let _ = motor_right.stop();
                        let _ = motor_left.stop();
                        interface.log("Ejecutando: STOP");
                    }
                    _ => {
                        interface.log("Error: Comando desconocido");
                    }
                }
            }
        }
    }
}
