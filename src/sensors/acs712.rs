// Version: v1.0
//! # Driver ACS712-30A — Sensor de corriente por efecto Hall
//!
//! Convierte lecturas crudas de ADC (10-bit, AVCC=5V) a corriente en mA.
//!
//! Este driver es puro Rust sin dependencias de `arduino_hal`: recibe el
//! valor crudo del ADC para que la lógica de conversión sea testeable en x86.
//! El manejo del ADC hardware (pin + `analog_read`) queda en `main.rs`.
//!
//! ## Especificaciones ACS712-30A
//! - Rango:        ±30 A
//! - Sensibilidad: 66 mV/A
//! - V_out @ 0 A:  VCC/2 = 2500 mV  (ajustable con `with_zero_mv`)

/// Sensibilidad del ACS712-30A en mV por amperio.
const SENSITIVITY_MV_PER_A: i32 = 66;

/// Driver para el sensor de corriente ACS712-30A.
pub struct ACS712 {
    /// Voltaje de offset a corriente cero en mV (teórico: 2500).
    /// Ajustar con `with_zero_mv` tras calibración en hardware real.
    zero_mv: i32,
}

impl ACS712 {
    /// Crea una instancia con el offset estándar de 2500 mV (VCC/2).
    pub fn new() -> Self {
        Self { zero_mv: 2500 }
    }

    /// Crea una instancia con offset de calibración personalizado.
    ///
    /// # Calibración
    /// Con 0 A real: leer ADC crudo, calcular `zero_mv = (adc * 5000) / 1023`
    /// y pasar ese valor aquí.
    pub fn with_zero_mv(zero_mv: i32) -> Self {
        Self { zero_mv }
    }

    /// Convierte un valor crudo de ADC (10-bit, AVCC=5V) a corriente en mA.
    ///
    /// Retorna valores negativos si la corriente fluye en sentido contrario.
    ///
    /// # Fórmula
    /// ```
    /// V_mv       = (adc * 5000) / 1023
    /// delta_mv   = V_mv − zero_mv
    /// I_mA       = (delta_mv × 1000) / 66
    /// ```
    pub fn read_ma(&self, adc_raw: u16) -> i32 {
        let v_mv = (adc_raw as u32 * 5000) / 1023;
        (v_mv as i32 - self.zero_mv) * 1000 / SENSITIVITY_MV_PER_A
    }

    /// Retorna `true` si el valor absoluto de corriente supera `threshold_ma`.
    pub fn is_overcurrent(&self, current_ma: i32, threshold_ma: i32) -> bool {
        current_ma.abs() > threshold_ma
    }
}
