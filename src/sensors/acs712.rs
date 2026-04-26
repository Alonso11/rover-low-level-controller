// Version: v1.2
//! # Driver ACS712 — Sensor de corriente por efecto Hall
//!
//! Convierte lecturas crudas de ADC (10-bit, AVCC=5V) a corriente en mA.
//! Disponible en variantes de ±5 A, ±20 A y ±30 A con distinta sensibilidad.
//!
//! ## Constructores recomendados
//! | Variante       | Rango | Sensibilidad | Resolución   | Uso en rover              |
//! |----------------|-------|--------------|--------------|---------------------------|
//! | `new_05a()`    | ±5 A  | 185 mV/A     | ~26 mA/count | Motores con L298N (2A)    |
//! | `new_20a()`    | ±20 A | 100 mV/A     | ~49 mA/count | Trade-off único sensor    |
//! | `new_30a()`    | ±30 A | 66 mV/A      | ~74 mA/count | Motores con BTS7960 (43A) |
//!
//! ## Trade-off: un solo tipo para los 6 motores
//! Si se usan 6× ACS712-20A (un tipo por compra):
//!   - L298N (fault a 2 A): 2000 mA / 49 mA/count ≈ 41 counts → detección OK
//!   - BTS7960 (fault a 15 A): 15000 mA / 49 mA/count ≈ 306 counts → OK
//!   - Pico BTS7960 (43 A): satura a 20 A → stall extremo no se detecta,
//!     pero el firmware ya dispara FAULT antes (OC_FAULT_BTS = 15 A < 20 A).
//!
//! ## Selección automática según feature de Cargo
//! - (default / `all-l298n`): `new_05a()` para los 6 motores
//! - `mixed-drivers`:  `new_05a()` para FR/FL, `new_30a()` para CR/CL/RR/RL
//! - `all-bts7960`:   `new_30a()` para los 6 motores
//! - `all-20a`:       `new_20a()` para los 6 motores (un tipo, trade-off)
//!
//! El driver es puro Rust sin dependencias de `arduino_hal`: recibe el
//! valor crudo del ADC para que la lógica de conversión sea testeable en x86.

/// Driver para el sensor de corriente ACS712.
#[derive(Clone, Copy)]
pub struct ACS712 {
    /// Voltaje de offset a corriente cero en mV (teórico: 2500 = VCC/2).
    zero_mv: i32,
    /// Sensibilidad del sensor en mV por amperio.
    /// ACS712-05A: 185 mV/A  |  ACS712-30A: 66 mV/A
    sensitivity_mv_a: i32,
}

impl ACS712 {
    /// ACS712-05A: ±5 A, 185 mV/A.
    /// Recomendado para motores con driver L298N (2 A continuo).
    /// Resolución: ~27 mA/count — 3× mejor que el 30A en este rango.
    pub fn new_05a() -> Self {
        Self { zero_mv: 2500, sensitivity_mv_a: 185 }
    }

    /// ACS712-20A: ±20 A, 100 mV/A.
    /// Trade-off cuando se compra un solo tipo para los 6 motores.
    /// Resolución: ~49 mA/count.
    /// - L298N (fault 2 A): 41 counts de margen → detección adecuada.
    /// - BTS7960 (fault 15 A): 306 counts → OK. Satura a 20 A (pico 43 A
    ///   no detectable), pero OC_FAULT_BTS dispara antes de saturar.
    pub fn new_20a() -> Self {
        Self { zero_mv: 2500, sensitivity_mv_a: 100 }
    }

    /// ACS712-30A: ±30 A, 66 mV/A.
    /// Recomendado para motores con driver BTS7960 (43 A pico).
    /// Resolución: ~74 mA/count.
    pub fn new_30a() -> Self {
        Self { zero_mv: 2500, sensitivity_mv_a: 66 }
    }

    /// Alias de `new_30a()` para compatibilidad con código existente.
    pub fn new() -> Self {
        Self::new_30a()
    }

    /// Crea una instancia 30A con offset de calibración personalizado.
    ///
    /// # Calibración
    /// Con 0 A real: leer ADC crudo, calcular `zero_mv = (adc * 5000) / 1023`
    /// y pasar ese valor aquí.
    pub fn with_zero_mv(zero_mv: i32) -> Self {
        Self { zero_mv, sensitivity_mv_a: 66 }
    }

    /// Aplica un offset de calibración a una instancia existente (builder).
    ///
    /// Uso: `ACS712::new_05a().calibrate_zero(measured_mv)`
    pub fn calibrate_zero(mut self, zero_mv: i32) -> Self {
        self.zero_mv = zero_mv;
        self
    }

    /// Convierte un valor crudo de ADC (10-bit, AVCC=5V) a corriente en mA.
    ///
    /// Retorna valores negativos si la corriente fluye en sentido contrario.
    ///
    /// # Fórmula
    /// ```
    /// zero_adc  = round(zero_mv × 1023 / 5000)
    /// delta_adc = adc − zero_adc
    /// I_mA      = delta_adc × 5_000_000 / (1023 × sensitivity_mv_a)
    /// ```
    pub fn read_ma(&self, adc_raw: u16) -> i32 {
        let zero_adc  = (self.zero_mv as u32 * 1023 + 2500) / 5000;
        let delta_adc = adc_raw as i32 - zero_adc as i32;
        ((delta_adc as i64 * 5_000_000_i64)
            / (1023_i64 * self.sensitivity_mv_a as i64)) as i32
    }

    /// Retorna `true` si el valor absoluto de corriente supera `threshold_ma`.
    pub fn is_overcurrent(&self, current_ma: i32, threshold_ma: i32) -> bool {
        current_ma.abs() > threshold_ma
    }
}
