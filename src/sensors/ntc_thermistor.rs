// Version: v1.0
//! # Driver NTC Thermistor — Módulo AD36958 (LM393 + NTC 10kΩ)
//!
//! Convierte lecturas crudas de ADC (10-bit, AVCC=5V) a temperatura en °C
//! usando una tabla de lookup con interpolación lineal entera.
//!
//! ## Parámetros del módulo (AD36958)
//! - NTC: 10 kΩ a 25 °C, B = 3950 K (típico módulo genérico)
//! - Resistencia pull-up en placa: 10 kΩ (de VCC al pin AO)
//!
//! ## Circuito del módulo
//! ```text
//! VCC ─── 10kΩ (pull-up) ─── AO ─── NTC ─── GND
//! ```
//! ADC sube cuando la temperatura baja (NTC más resistente → más voltaje).
//!
//! ## Fórmula usada para precalcular la tabla
//! ```text
//! R_ntc = 10000 × ADC / (1023 − ADC)
//! T(K)  = 1 / ( 1/298.15 + ln(R_ntc / 10000) / 3950 )
//! T(°C) = T(K) − 273
//! ```
//!
//! ## Uso en rover — 6 sensores para 3 bancos de baterías 18650
//! ```text
//! A7  / A8  → Banco 1 (sensor A / sensor B)
//! A9  / A10 → Banco 2 (sensor A / sensor B)
//! A11 / A12 → Banco 3 (sensor A / sensor B)
//! ```

// Tabla lookup: (adc_raw, temp_celsius)
// Ordenada por ADC descendente (= temperatura ascendente).
// Rango cubierto: −20 °C … 100 °C.
// Fuera de rango se satura al extremo más cercano.
const NTC_TABLE: &[(u16, i16)] = &[
    (934, -20),
    (874, -10),
    (787,   0),
    (682,  10),
    (568,  20),
    (512,  25),
    (454,  30),
    (352,  40),
    (308,  45),
    (268,  50),
    (202,  60),
    (152,  70),
    (114,  80),
    ( 87,  90),
    ( 66, 100),
];

/// Driver para el módulo termistor NTC AD36958.
#[derive(Clone, Copy)]
pub struct NTCThermistor {
    /// Offset de calibración en °C (puede ser negativo).
    offset_c: i32,
}

impl NTCThermistor {
    /// Crea una instancia con offset de calibración = 0.
    pub fn new() -> Self {
        Self { offset_c: 0 }
    }

    /// Crea una instancia con offset de calibración predefinido.
    pub fn with_offset(offset_c: i32) -> Self {
        Self { offset_c }
    }

    /// Aplica un offset de calibración (builder pattern).
    ///
    /// Uso: `NTCThermistor::new().calibrate(measured_offset)`
    pub fn calibrate(mut self, offset_c: i32) -> Self {
        self.offset_c = offset_c;
        self
    }

    /// Convierte un valor crudo de ADC (10-bit, AVCC=5V) a temperatura en °C.
    ///
    /// Usa interpolación lineal entera entre entradas de la tabla lookup.
    /// Valores fuera de rango se saturan: ADC muy alto → −20 °C, ADC muy bajo → 100 °C.
    pub fn read_celsius(&self, adc_raw: u16) -> i32 {
        let adc = adc_raw.min(1023);
        let last = NTC_TABLE.len() - 1;

        // ADC por encima del primer punto → temperatura mínima de la tabla
        if adc >= NTC_TABLE[0].0 {
            return NTC_TABLE[0].1 as i32 + self.offset_c;
        }
        // ADC por debajo del último punto → temperatura máxima de la tabla
        if adc <= NTC_TABLE[last].0 {
            return NTC_TABLE[last].1 as i32 + self.offset_c;
        }

        // Buscar el segmento [i, i+1] que contiene el ADC
        for i in 0..last {
            let (adc_hi, temp_lo) = NTC_TABLE[i];
            let (adc_lo, temp_hi) = NTC_TABLE[i + 1];

            if adc <= adc_hi && adc >= adc_lo {
                // Interpolación lineal entera
                // t = temp_lo + (temp_hi − temp_lo) × (adc_hi − adc) / (adc_hi − adc_lo)
                let dt = (temp_hi - temp_lo) as i32;
                let da = (adc_hi - adc_lo) as i32;
                let dx = (adc_hi - adc) as i32;
                let t = temp_lo as i32 + dt * dx / da;
                return t + self.offset_c;
            }
        }
        // No debería llegar aquí si la tabla está correcta
        self.offset_c
    }

    /// Retorna `true` si la temperatura supera el umbral (estricto: >).
    pub fn is_overtemp(&self, temp_c: i32, threshold_c: i32) -> bool {
        temp_c > threshold_c
    }
}
