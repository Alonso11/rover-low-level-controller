// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::simple_pwm::IntoPwmPin;
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::motor_control::Motor;
use rover_low_level_controller::motor_control::bts7960::BTS7960Motor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // --- 1. Inicializar Comunicación con RPi 5 (Serial 3 — USART3) ---
    // TX3=D14 (PJ1), RX3=D15 (PJ0)
    // USART1 (D18/D19) queda libre para encoders INT2/INT3 de los motores centrales.
    let rx = pins.d15.into_pull_up_input();
    let tx = pins.d14.into_output();

    let serial = arduino_hal::Usart::new(
        dp.USART3,
        rx,
        tx,
        arduino_hal::hal::usart::BaudrateExt::into_baudrate(115200),
    );
    let mut interface = CommandInterface::new(serial);

    // --- 2. Inicializar Motor BTS7960 ---
    let mut timer2 = arduino_hal::simple_pwm::Timer2Pwm::new(dp.TC2, arduino_hal::simple_pwm::Prescaler::Prescale64);
    let rpwm = pins.d9.into_output().into_pwm(&mut timer2);
    let lpwm = pins.d10.into_output().into_pwm(&mut timer2);
    let mut motor = BTS7960Motor::new(rpwm, lpwm, false);

    interface.log("Motor listo para comandos RPi5 (F, B, S)");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            match cmd[0] {
                b'F' | b'f' => {
                    motor.set_speed(70);
                    interface.log("Ejecutando: Adelante");
                },
                b'B' | b'b' => {
                    motor.set_speed(-70);
                    interface.log("Ejecutando: Atras");
                },
                b'S' | b's' => {
                    motor.stop();
                    interface.log("Ejecutando: STOP");
                },
                _ => {
                    interface.log("Error: Comando desconocido");
                }
            }
        }
    }
}
