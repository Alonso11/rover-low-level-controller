// Version: v1.0
//! Driver para el Puente-H de alta potencia BTS7960 (IBT-2)
//! Este módulo implementa el control de motores utilizando dos señales PWM (RPWM y LPWM).

use arduino_hal::hal::port::Pin;
use arduino_hal::hal::port::mode::PwmOutput;
use arduino_hal::hal::simple_pwm::PwmPinOps;
use crate::motor_control::Motor;

/// Implementación del controlador para el BTS7960.
/// Requiere dos pines PWM: uno para avance (Right PWM) y otro para retroceso (Left PWM).
#[allow(dead_code)]
pub struct BTS7960Motor<TC1, PIN1, TC2, PIN2> {
    rpwm: Pin<PwmOutput<TC1>, PIN1>,
    lpwm: Pin<PwmOutput<TC2>, PIN2>,
    inverted: bool,
}

impl<TC1, PIN1, TC2, PIN2> BTS7960Motor<TC1, PIN1, TC2, PIN2>
where
    PIN1: PwmPinOps<TC1, Duty = u8>,
    PIN2: PwmPinOps<TC2, Duty = u8>,
{
    /// Crea una nueva instancia para el BTS7960.
    #[allow(dead_code)]
    pub fn new(mut rpwm: Pin<PwmOutput<TC1>, PIN1>, mut lpwm: Pin<PwmOutput<TC2>, PIN2>, inverted: bool) -> Self {
        rpwm.enable();
        lpwm.enable();
        Self {
            rpwm,
            lpwm,
            inverted,
        }
    }
}

impl<TC1, PIN1, TC2, PIN2> Motor for BTS7960Motor<TC1, PIN1, TC2, PIN2>
where
    PIN1: PwmPinOps<TC1, Duty = u8>,
    PIN2: PwmPinOps<TC2, Duty = u8>,
{
    fn set_speed(&mut self, speed: i16) {
        let is_forward = if self.inverted { speed < 0 } else { speed >= 0 };
        let abs_speed = speed.abs() as u32;

        let max_duty = self.rpwm.get_max_duty() as u32;
        let duty = ((abs_speed * max_duty) / 100) as u8;

        if abs_speed == 0 {
            self.stop();
        } else if is_forward {
            self.lpwm.set_duty(0);
            self.rpwm.set_duty(duty);
        } else {
            self.rpwm.set_duty(0);
            self.lpwm.set_duty(duty);
        }
    }

    fn stop(&mut self) {
        self.rpwm.set_duty(0);
        self.lpwm.set_duty(0);
    }
}
