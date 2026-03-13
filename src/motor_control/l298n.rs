//! Driver para el Puente-H L298N
//! Este módulo implementa el control de motores DC utilizando el driver L298N.

use arduino_hal::hal::port::{Pin, PinOps};
use arduino_hal::hal::port::mode::{Output, PwmOutput};
use arduino_hal::hal::simple_pwm::PwmPinOps;
use crate::motor_control::Motor;

/// Implementación del controlador para el driver Puente-H L298N.
#[allow(dead_code)]
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
    /// Crea una nueva instancia para el L298N.
    #[allow(dead_code)]
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

/// Estructura para controlar un chasis de 6 ruedas con 3 puentes-H L298N.
#[allow(dead_code)]
pub struct SixWheelRover<M1, M2, M3, M4, M5, M6> {
    pub frontal_right: M1,
    pub frontal_left: M2,
    pub center_right: M3,
    pub center_left: M4,
    pub rear_right: M5,
    pub rear_left: M6,
}

impl<M1, M2, M3, M4, M5, M6> SixWheelRover<M1, M2, M3, M4, M5, M6>
where
    M1: Motor, M2: Motor, M3: Motor, M4: Motor, M5: Motor, M6: Motor,
{
    pub fn new(fr: M1, fl: M2, cr: M3, cl: M4, rr: M5, rl: M6) -> Self {
        Self {
            frontal_right: fr,
            frontal_left: fl,
            center_right: cr,
            center_left: cl,
            rear_right: rr,
            rear_left: rl,
        }
    }

    pub fn set_speeds(&mut self, left_speed: i16, right_speed: i16) {
        self.frontal_left.set_speed(left_speed);
        self.center_left.set_speed(left_speed);
        self.rear_left.set_speed(left_speed);
        
        self.frontal_right.set_speed(right_speed);
        self.center_right.set_speed(right_speed);
        self.rear_right.set_speed(right_speed);
    }

    pub fn stop(&mut self) {
        self.frontal_left.stop();
        self.center_left.stop();
        self.rear_left.stop();
        self.frontal_right.stop();
        self.center_right.stop();
        self.rear_right.stop();
    }
}
