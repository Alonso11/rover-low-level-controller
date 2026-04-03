// Version: v1.0
//! # Driver LM335 — Sensor de temperatura de precisión
//!
//! Convierte lecturas crudas de ADC (10-bit, AVCC=5V) a temperatura en °C y K.
//!
//! Este driver es puro Rust sin dependencias de `arduino_hal`: recibe el
//! valor crudo del ADC para que la lógica de conversión sea testeable en x86.
//! El manejo del ADC hardware (pin + `analog_read`) queda en `main.rs`.
//!
//! ## Especificaciones LM335
//! - Salida:      10 mV / K
//! - V @ 0 °C:   2731 mV  (273 K × 10 mV/K)
//! - V @ 25 °C:  2981 mV  (298 K × 10 mV/K)
//! - Requiere resistencia de polarización R ≈ 2 kΩ desde 5V al pin OUT.

/// Driver para el sensor de temperatura LM335.
pub struct LM335 {
    /// Offset de calibración en Kelvin (sumar al resultado).
    /// Ajustar si la lectura difiere de un termómetro de referencia.
    offset_k: i32,
}

impl LM335 {
    /// Crea una instancia sin offset de calibración.
    pub fn new() -> Self {
        Self { offset_k: 0 }
    }

    /// Crea una instancia con offset de calibración personalizado.
    ///
    /// # Calibración
    /// Si el sensor marca 28 °C y la referencia es 25 °C → `offset_k = -3`.
    pub fn with_offset(offset_k: i32) -> Self {
        Self { offset_k }
    }

    /// Convierte un valor crudo de ADC (10-bit, AVCC=5V) a temperatura en Kelvin.
    ///
    /// # Fórmula
    /// ```
    /// V_mv     = (adc * 5000) / 1023
    /// T_kelvin = V_mv / 10  + offset_k
    /// ```
    pub fn read_kelvin(&self, adc_raw: u16) -> i32 {
        let v_mv = (adc_raw as u32 * 5000) / 1023;
        (v_mv / 10) as i32 + self.offset_k
    }

    /// Convierte un valor crudo de ADC a temperatura en Celsius.
    pub fn read_celsius(&self, adc_raw: u16) -> i32 {
        self.read_kelvin(adc_raw) - 273
    }
}
