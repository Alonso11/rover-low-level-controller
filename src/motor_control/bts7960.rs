// Version: v1.4
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
//! - Adelante:  RPWM = duty, LPWM = off
//! - Atrás:     RPWM = off,  LPWM = duty
//! - Stop/Freno: ambos canales DESCONECTADOS (`stop()` / `brake()`)
//!
//! ## ⚠️ PWM a duty 0 NO es 0 V — usar `disable()` para apagar de verdad
//!
//! avr-hal configura estos timers en **Fast PWM** (WGM=0b0101, non-inverting
//! `com().match_clear()`). En Fast PWM, `set_duty(0)` con `OCRnx = BOTTOM` NO
//! deja el pin en LOW: el datasheet del ATmega2560 dice que la salida es
//! *"a narrow spike for each TOP+1 timer clock cycle"* — un pulso de ~1 ciclo
//! de timer por periodo (≈0.4 % de duty residual a Prescale64).
//!
//! Ese 0.4 % es suficiente para que la rueda de **menor fricción (FR)** —al aire
//! en banco— SIGA GIRANDO aunque el firmware "pare" el motor. Empeora porque el
//! FR mezcla dos timers (RPWM=Timer2, LPWM=Timer5): los spikes no quedan en fase
//! y aparece un voltaje neto asimétrico (bringup 2026-06-02, FR gira en FAULT).
//!
//! Solución: para PARAR de verdad hay que **`disable()`** el canal (escribe
//! `com().disconnected()` → el pin pasa a ser gobernado por el registro PORT,
//! que `into_output()` dejó en LOW = 0 V real). `set_speed()` vuelve a
//! `enable()` el canal activo antes de aplicar duty. Con ambos canales
//! desconectados (IN bajos, R_EN/L_EN altos) el BTS7960 pone ambas salidas al
//! low-side → motor en corto a GND = freno pasivo, sin spike, sin giro.

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
    /// Crea una nueva instancia. Por seguridad, el motor arranca DESHABILITADO 
    /// (R_EN y L_EN en LOW). Llame a `enable()` para activar el driver.
    #[allow(dead_code)]
    pub fn new(
        mut rpwm: Pin<PwmOutput<TC1>, PIN1>,
        mut lpwm: Pin<PwmOutput<TC2>, PIN2>,
        mut r_en: Pin<Output, REnPin>,
        mut l_en: Pin<Output, LEnPin>,
        inverted: bool,
    ) -> Self {
        // Estado inicial = estado de `stop()`: ambos canales DESCONECTADOS (no
        // `enable()` con duty 0, que dejaría el spike de Fast PWM). Así, cuando
        // `enable()` suba R_EN/L_EN tras calibrar los ACS712, no hay arranque
        // espurio. `set_speed()` reconecta el canal activo cuando haga falta.
        rpwm.set_duty(0);
        lpwm.set_duty(0);
        rpwm.disable();
        lpwm.disable();
        r_en.set_low();
        l_en.set_low();
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
            // Canal inverso APAGADO de verdad (desconectado, no duty 0 = spike).
            self.lpwm.set_duty(0);
            self.lpwm.disable();
            // Canal activo: fijar duty y reconectar el comparador.
            self.rpwm.set_duty(duty);
            self.rpwm.enable();
        } else {
            let duty = ((abs_speed * self.lpwm.get_max_duty() as u32) / 100) as u8;
            self.rpwm.set_duty(0);
            self.rpwm.disable();
            self.lpwm.set_duty(duty);
            self.lpwm.enable();
        }
    }

    /// Detiene el motor: DESCONECTA ambos canales PWM (no solo duty 0).
    ///
    /// `set_duty(0)` en Fast PWM deja un spike residual (~0.4 %) que hace girar
    /// al FR (ver nota del módulo). `disable()` escribe `com().disconnected()`
    /// → el pin pasa al registro PORT (LOW por `into_output()`) = 0 V real.
    /// Con ambas IN en LOW y R_EN/L_EN en HIGH, el BTS7960 conmuta ambas salidas
    /// al low-side: motor en corto a GND = freno pasivo, sin giro residual.
    fn stop(&mut self) {
        self.rpwm.set_duty(0);
        self.lpwm.set_duty(0);
        self.rpwm.disable();
        self.lpwm.disable();
    }

    /// Freno seguro: idéntico a `stop()` — ambos canales desconectados (0 V real,
    /// motor en corto a GND vía low-side del BTS7960).
    ///
    /// HISTORIA: el freno "high-side" original (ambos PWM al MÁXIMO) hacía GIRAR
    /// al FR en FAULT (timers RPWM=Timer2/LPWM=Timer5 desfasados → voltaje neto).
    /// Se cambió a both-0 (2026-06-01), pero `set_duty(0)` ≠ 0 V en Fast PWM
    /// (spike de 0.4 %) → el FR SEGUÍA girando (bringup 2026-06-02). El fix
    /// definitivo es DESCONECTAR el comparador (`disable()`), no bajar el duty.
    fn brake(&mut self) {
        self.stop();
    }

    /// Habilita el driver (R_EN y L_EN → HIGH).
    fn enable(&mut self) {
        self.r_en.set_high();
        self.l_en.set_high();
    }

    /// Deshabilita el driver: para primero y baja R_EN y L_EN (Hi-Z).
    fn disable(&mut self) {
        self.stop();
        self.r_en.set_low();
        self.l_en.set_low();
    }
}
