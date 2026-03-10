//! Driver para el Puente-H L298N
//! Este módulo implementa el control de motores DC utilizando el driver L298N.

use arduino_hal::hal::port::{Pin, PinOps};
use arduino_hal::hal::port::mode::{Output, PwmOutput};
use arduino_hal::hal::simple_pwm::PwmPinOps;
use crate::motor_control::Motor; // Importamos el trait desde mod.rs

/// Implementación del controlador para el driver Puente-H L298N.
/// Requiere un pin PWM (ENA o ENB) y dos pines de salida digital (IN1/IN2 o IN3/IN4).
pub struct L298NMotor<TC, PwmPin, In1Pin, In2Pin> {
    pwm: Pin<PwmOutput<TC>, PwmPin>,
    in1: Pin<Output, In1Pin>,
    in2: Pin<Output, In2Pin>,
    inverted: bool,
}

impl<TC, PwmPin, In1Pin, In2Pin> L298NMotor<TC, PwmPin, In1Pin, In2Pin> 
where 
    PwmPin: PwmPinOps<TC, Duty = u8>,
    In1Pin: PinOps,
    In2Pin: PinOps,
{
    pub fn new(mut pwm: Pin<PwmOutput<TC>, PwmPin>, in1: Pin<Output, In1Pin>, in2: Pin<Output, In2Pin>, inverted: bool) -> Self {
        pwm.enable();
        Self {
            pwm,
            in1,
            in2,
            inverted,
        }
    }
}

impl<TC, PwmPin, In1Pin, In2Pin> Motor for L298NMotor<TC, PwmPin, In1Pin, In2Pin>
where 
    PwmPin: PwmPinOps<TC, Duty = u8>,
    In1Pin: PinOps,
    In2Pin: PinOps,
{
    fn set_speed(&mut self, speed: i16) {
        let is_forward = if self.inverted { speed < 0 } else { speed >= 0 };
        let abs_speed = speed.abs() as u32;

        if abs_speed == 0 {
            self.stop();
            return;
        }

        if is_forward {
            self.in1.set_high();
            self.in2.set_low();
        } else {
            self.in1.set_low();
            self.in2.set_high();
        }

        let max_duty = self.pwm.get_max_duty() as u32;
        let duty = ((abs_speed * max_duty) / 100) as u8;
        
        self.pwm.set_duty(duty);
    }

    fn stop(&mut self) {
        self.in1.set_low();
        self.in2.set_low();
        self.pwm.set_duty(0);
    }
}
