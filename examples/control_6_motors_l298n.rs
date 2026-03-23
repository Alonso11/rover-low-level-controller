// Version: v3.0 - Layout Físico Verificado
//! # Ejemplo: Control de Rover de 6 Ruedas (L298N)
//!
//! Este programa controla un chasis de 6 ruedas utilizando 3 drivers L298N.
//! Pinout verificado físicamente con `debug_motors_l298n.rs`.
//!
//! ## Distribución de Timers y Pines (verificado):
//! * **Timer 2:** Motores Frontales  — D9  (OC2B, Front Right), D10 (OC2A, Front Left)
//! * **Timer 4:** Centro + Tras. Der.— D8  (OC4C, Center Right), D7 (OC4B, Center Left), D6 (OC4A, Rear Right)
//! * **Timer 3:** Trasero Izquierdo  — D5  (OC3A, Rear Left)
//! * **Timer 1:** Reservado Servo    — D11 (OC1A) — libre, no usar para motores
//!
//! ## Pines de Dirección (verificados):
//! * Front:  D9→ENB: IN3=D23, IN4=D25 | D10→ENA: IN1=D22, IN2=D24
//! * [D26, D27 libres — separador físico Driver1/Driver2]
//! * Center: D8→ENA: IN1=D28, IN2=D30 | D7→ENB: IN3=D29, IN4=D31
//! * [D32, D33 libres — separador físico Driver2/Driver3]
//! * Rear:   D6→ENA: IN1=D34, IN2=D35 | D5→OC3A: IN1=D36, IN2=D37
//!
//! ## Compatibilidad con encoders y sensores (sin conflictos):
//! * D2  (INT4, PE4) → Encoder Rear Right
//! * D3  (INT5, PE5) → Encoder Rear Left
//! * D18 (INT3, PD3) → Encoder Center Left  (RPi en USART3, no USART1)
//! * D19 (INT2, PD2) → Encoder Center Right (RPi en USART3, no USART1)
//! * D20 (INT1, PD1) → Encoder Front Left
//! * D21 (INT0, PD0) → Encoder Front Right
//! * D14 (TX3, PJ1)  → RPi TX (USART3)
//! * D15 (RX3, PJ0)  → RPi RX (USART3)
//! * D16 (TX2, PH1)  → TF-Luna RX
//! * D17 (RX2, PH0)  → TF-Luna TX
//! * D38 (PD7)       → HC-SR04 Trigger
//! * D39 (PG2)       → HC-SR04 Echo
//!
//! Comunicación: 115200 baudios vía Serial (USB, USART0).

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

    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    // --- CONFIGURACIÓN DE TIMERS ---
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64); // Motores frontales
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64); // Trasero izquierdo
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64); // Centro + trasero der

    // --- MOTOR 1: Frontal Derecho (Timer2/OC2B — D9/ENB) ---
    let fr = L298NMotor::new(
        pins.d9.into_output().into_pwm(&mut timer2),
        pins.d23.into_output(), // IN3 (canal B)
        pins.d25.into_output(), // IN4 (canal B)
        false
    );

    // --- MOTOR 2: Frontal Izquierdo (Timer2/OC2A — D10/ENA) ---
    let fl = L298NMotor::new(
        pins.d10.into_output().into_pwm(&mut timer2),
        pins.d22.into_output(), // IN1 (canal A)
        pins.d24.into_output(), // IN2 (canal A)
        false
    );

    // D26, D27 libres — separador físico Driver1/Driver2

    // --- MOTOR 3: Central Derecho (Timer4/OC4C — D8/ENA) ---
    let cr = L298NMotor::new(
        pins.d8.into_output().into_pwm(&mut timer4),
        pins.d28.into_output(), // IN1 (canal A)
        pins.d30.into_output(), // IN2 (canal A)
        false
    );

    // --- MOTOR 4: Central Izquierdo (Timer4/OC4B — D7/ENB) ---
    let cl = L298NMotor::new(
        pins.d7.into_output().into_pwm(&mut timer4),
        pins.d29.into_output(), // IN3 (canal B)
        pins.d31.into_output(), // IN4 (canal B)
        false
    );

    // D32, D33 libres — separador físico Driver2/Driver3

    // --- MOTOR 5: Trasero Derecho (Timer4/OC4A — D6) ---
    let rr = L298NMotor::new(
        pins.d6.into_output().into_pwm(&mut timer4),
        pins.d34.into_output(),
        pins.d35.into_output(),
        false
    );

    // --- MOTOR 6: Trasero Izquierdo (Timer3/OC3A — D5) ---
    let rl = L298NMotor::new(
        pins.d5.into_output().into_pwm(&mut timer3),
        pins.d36.into_output(),
        pins.d37.into_output(),
        false
    );

    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    interface.log("Rover 6 Motores - Layout Verificado v3.0");

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
