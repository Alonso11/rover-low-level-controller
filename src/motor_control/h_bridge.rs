use super::Motor;
use embedded_hal::digital::OutputPin;
use embedded_hal::pwm::SetDutyCycle;

/// Implementacion para un puente H estandar (PWM + 2 pines de direccion).
pub struct HBridge<PWM, IN1, IN2> {
    pwm_pin: PWM,
    in1_pin: IN1,
    in2_pin: IN2,
}

impl<PWM, IN1, IN2> HBridge<PWM, IN1, IN2>
where
    PWM: SetDutyCycle,
    IN1: OutputPin,
    IN2: OutputPin,
{
    pub fn new(pwm_pin: PWM, in1_pin: IN1, in2_pin: IN2) -> Self {
        let mut driver = Self { pwm_pin, in1_pin, in2_pin };
        driver.stop();
        driver
    }
}

impl<PWM, IN1, IN2> Motor for HBridge<PWM, IN1, IN2>
where
    PWM: SetDutyCycle,
    IN1: OutputPin,
    IN2: OutputPin,
{
    fn set_speed(&mut self, speed: i16) {
        if speed == 0 {
            self.stop();
        } else if speed > 0 {
            // Adelante
            let _ = self.in1_pin.set_high();
            let _ = self.in2_pin.set_low();
            // PWM 1.0 usa set_duty_cycle(valor) o set_duty_cycle_fraction(num, den)
            // Para simplificar, asumimos que 255 es el maximo segun el Timer configurado
            let _ = self.pwm_pin.set_duty_cycle(speed.min(255) as u16);
        } else {
            // Atras
            let _ = self.in1_pin.set_low();
            let _ = self.in2_pin.set_high();
            let _ = self.pwm_pin.set_duty_cycle(speed.abs().min(255) as u16);
        }
    }

    fn stop(&mut self) {
        let _ = self.in1_pin.set_low();
        let _ = self.in2_pin.set_low();
        let _ = self.pwm_pin.set_duty_cycle(0);
    }
}
