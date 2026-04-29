// Version: v1.1
//! Wrapper que combina cualquier `Motor` con cualquier `Encoder`.
//!
//! Permite que `SixWheelRover` mezcle motores con y sin encoder sin cambiar
//! su interfaz: `MotorWithEncoder` implementa el trait `Motor` igual que
//! `L298NMotor` o `BTS7960Motor`.
//!
//! ## Uso típico (rover 6 ruedas mixto)
//!
//! ```ignore
//! // 4 ruedas NFP-5840-31ZY-EN con encoder
//! let fr = MotorWithEncoder::new(bts_fr, &ENCODER_FR);
//! let fl = MotorWithEncoder::new(bts_fl, &ENCODER_FL);
//! let cr = MotorWithEncoder::new(bts_cr, &ENCODER_CR);
//! let cl = MotorWithEncoder::new(bts_cl, &ENCODER_CL);
//! // 2 ruedas traseras sin encoder
//! let rr = L298NMotor::new(...);
//! let rl = L298NMotor::new(...);
//!
//! let rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);
//! ```

use crate::motor_control::Motor;
use crate::sensors::Encoder;

/// Motor DC con encoder asociado.
///
/// `E` debe ser `'static` porque el encoder vive en una variable global
/// accesible desde ISRs de interrupción.
pub struct MotorWithEncoder<M, E: 'static> {
    motor:       M,
    encoder:     &'static E,
    last_counts: i32,
}

impl<M: Motor, E: Encoder> MotorWithEncoder<M, E> {
    pub fn new(motor: M, encoder: &'static E) -> Self {
        Self { motor, encoder, last_counts: 0 }
    }

    /// Pulsos acumulados desde el inicio o desde el último `reset_encoder()`.
    pub fn counts(&self) -> i32 {
        self.encoder.get_counts()
    }

    /// Variación de pulsos desde la última llamada a este método.
    /// Útil para calcular velocidad en cada ciclo de control.
    pub fn delta_counts(&mut self) -> i32 {
        let now   = self.encoder.get_counts();
        let delta = now.wrapping_sub(self.last_counts);
        self.last_counts = now;
        delta
    }

    /// Reinicia el encoder y el contador interno de delta.
    pub fn reset_encoder(&mut self) {
        self.encoder.reset();
        self.last_counts = 0;
    }
}

impl<M: Motor, E: Encoder> Motor for MotorWithEncoder<M, E> {
    fn set_speed(&mut self, speed: i16) { self.motor.set_speed(speed); }
    fn stop(&mut self)                  { self.motor.stop(); }
    fn brake(&mut self)                 { self.motor.brake(); }
    fn enable(&mut self)                { self.motor.enable(); }
    fn disable(&mut self)               { self.motor.disable(); }
}
