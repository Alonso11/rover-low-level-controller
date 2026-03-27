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
    /// Se calcula el punto de cruce por cero en unidades ADC (redondeado al
    /// entero más cercano) para evitar error de truncación al pasar dos veces
    /// por división entera (ADC→mV→mA):
    /// ```
    /// zero_adc   = round(zero_mv × 1023 / 5000)
    /// delta_adc  = adc − zero_adc
    /// I_mA       = delta_adc × 5 000 000 / (1023 × 66)
    /// ```
    pub fn read_ma(&self, adc_raw: u16) -> i32 {
        // Redondeo al entero más cercano: +2500 antes de dividir por 5000.
        let zero_adc = (self.zero_mv as u32 * 1023 + 2500) / 5000;
        let delta_adc = adc_raw as i32 - zero_adc as i32;
        // Usa i64 para evitar desbordamiento (max: 512 × 5_000_000 > i32::MAX).
        ((delta_adc as i64 * 5_000_000_i64)
            / (1023_i64 * SENSITIVITY_MV_PER_A as i64)) as i32
    }

    /// Retorna `true` si el valor absoluto de corriente supera `threshold_ma`.
    pub fn is_overcurrent(&self, current_ma: i32, threshold_ma: i32) -> bool {
        current_ma.abs() > threshold_ma
    }
}
