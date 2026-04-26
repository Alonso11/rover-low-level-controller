// Version: v1.1
//! Driver para el Puente-H de alta potencia BTS7960 (módulo IBT-2)
//!
//! ## Pinout del módulo IBT-2
//!
//! | Pin   | Función                  | Conexión Arduino        |
//! |-------|--------------------------|-------------------------|
//! | RPWM  | PWM canal adelante       | Pin PWM (ej. D9)        |
//! | LPWM  | PWM canal atrás          | Pin PWM (ej. D10)       |
//! | R_EN  | Habilita canal adelante  | Pin digital output HIGH |
//! | L_EN  | Habilita canal atrás     | Pin digital output HIGH |
//! | R_IS  | Current sense adelante   | No conectado (ACS712 externo) |
//! | L_IS  | Current sense atrás      | No conectado (ACS712 externo) |
//! | VCC   | Lógica 5V                | 5V Arduino              |
//! | GND   | Tierra lógica            | GND Arduino             |
//! | B+/B- | Alimentación motor       | Batería                 |
//! | M+/M- | Salida al motor          | Motor DC                |
//!
//! ## Operación
//!
//! R_EN y L_EN deben estar en HIGH para que el driver conduzca.
//! Este driver los activa en `new()` y los mantiene HIGH siempre.
//!
//! Control de dirección:
//! - Adelante:  RPWM = duty, LPWM = 0
//! - Atrás:     RPWM = 0,    LPWM = duty
//! - Stop:      RPWM = 0,    LPWM = 0  (free-wheel / coast)

use arduino_hal::hal::port::{Pin, PinOps};
use arduino_hal::hal::port::mode::{Output, PwmOutput};
use arduino_hal::hal::simple_pwm::PwmPinOps;
use crate::motor_control::Motor;

/// Controlador para el módulo IBT-2 (BTS7960).
///
/// Requiere dos pines PWM para velocidad/dirección y dos pines digitales
/// para habilitar cada canal del driver.
#[allow(dead_code)]
pub struct BTS7960Motor<TC1, PIN1, TC2, PIN2, REnPin, LEnPin> {
    rpwm:     Pin<PwmOutput<TC1>, PIN1>,
    lpwm:     Pin<PwmOutput<TC2>, PIN2>,
    r_en:     Pin<Output, REnPin>,
    l_en:     Pin<Output, LEnPin>,
    inverted: bool,
}

impl<TC1, PIN1, TC2, PIN2, REnPin, LEnPin> BTS7960Motor<TC1, PIN1, TC2, PIN2, REnPin, LEnPin>
where
    PIN1:   PwmPinOps<TC1, Duty = u8>,
    PIN2:   PwmPinOps<TC2, Duty = u8>,
    REnPin: PinOps,
    LEnPin: PinOps,
{
    /// Crea una nueva instancia y activa ambos canales del IBT-2 (R_EN y L_EN → HIGH).
    #[allow(dead_code)]
    pub fn new(
        mut rpwm: Pin<PwmOutput<TC1>, PIN1>,
        mut lpwm: Pin<PwmOutput<TC2>, PIN2>,
        mut r_en: Pin<Output, REnPin>,
        mut l_en: Pin<Output, LEnPin>,
        inverted: bool,
    ) -> Self {
        rpwm.enable();
        lpwm.enable();
        r_en.set_high(); // habilitar canal adelante
        l_en.set_high(); // habilitar canal atrás
        Self { rpwm, lpwm, r_en, l_en, inverted }
    }
}

impl<TC1, PIN1, TC2, PIN2, REnPin, LEnPin> Motor
    for BTS7960Motor<TC1, PIN1, TC2, PIN2, REnPin, LEnPin>
where
    PIN1:   PwmPinOps<TC1, Duty = u8>,
    PIN2:   PwmPinOps<TC2, Duty = u8>,
    REnPin: PinOps,
    LEnPin: PinOps,
{
    /// Ajusta velocidad y dirección. `speed`: -100 (atrás) a 100 (adelante).
    fn set_speed(&mut self, speed: i16) {
        if speed == 0 {
            self.stop();
            return;
        }

        let is_forward = if self.inverted { speed < 0 } else { speed > 0 };
        let abs_speed  = speed.unsigned_abs() as u32;

        if is_forward {
            let duty = ((abs_speed * self.rpwm.get_max_duty() as u32) / 100) as u8;
            self.lpwm.set_duty(0);
            self.rpwm.set_duty(duty);
        } else {
            let duty = ((abs_speed * self.lpwm.get_max_duty() as u32) / 100) as u8;
            self.rpwm.set_duty(0);
            self.lpwm.set_duty(duty);
        }
    }

    /// Detiene el motor (free-wheel: ambos PWM a 0, EN permanecen HIGH).
    fn stop(&mut self) {
        self.rpwm.set_duty(0);
        self.lpwm.set_duty(0);
    }
}
