use arduino_hal::hal::port::{Pin, PinOps};
use arduino_hal::hal::port::mode::{Output, PwmOutput};
use arduino_hal::hal::simple_pwm::PwmPinOps;

/// Define las acciones que cualquier motor del rover debe poder hacer
pub trait Motor {
    fn set_speed(&mut self, speed: i16);
    fn stop(&mut self);
}

/// Implementación de un motor DC usando un puente H L298N
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

        // Ahora que sabemos que Duty = u8, el cast a u32 funciona
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
