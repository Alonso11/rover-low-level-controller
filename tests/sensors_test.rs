// Version: v1.0
// Pruebas unitarias de los drivers analógicos ACS712 y LM335.
// Se ejecutan en PC sin necesidad del Arduino.
// Comando: ./test_native.sh  (o ver test_native.sh para el comando completo)

use rover_low_level_controller::sensors::{ACS712, LM335};

// ─── ACS712-30A ───────────────────────────────────────────────────────────────

#[test]
fn test_acs712_zero_current() {
    // ADC 512 → V = (512 * 5000) / 1023 = 2500 mV → I = 0 mA
    let acs = ACS712::new();
    assert_eq!(acs.read_ma(512), 0);
}

#[test]
fn test_acs712_positive_10a() {
    // 10 A → V = 2500 + 10×66 = 3160 mV → ADC = (3160×1023)/5000 = 647
    // Verificamos que la lectura está dentro de ±15 mA de 10000
    let acs = ACS712::new();
    let ma = acs.read_ma(647);
    assert!((ma - 10000).abs() < 15, "esperado ~10000 mA, got {}", ma);
}

#[test]
fn test_acs712_negative_current() {
    // ADC por debajo de 512 → corriente negativa
    let acs = ACS712::new();
    assert!(acs.read_ma(400) < 0);
}

#[test]
fn test_acs712_max_adc_positive() {
    // ADC 1023 → V = 5000 mV → delta = 2500 → I = 37878 mA (~37.9 A)
    let acs = ACS712::new();
    assert!(acs.read_ma(1023) > 0);
}

#[test]
fn test_acs712_min_adc_negative() {
    // ADC 0 → V = 0 mV → delta = -2500 → I = -37878 mA
    let acs = ACS712::new();
    assert!(acs.read_ma(0) < 0);
}

#[test]
fn test_acs712_not_overcurrent_below_threshold() {
    let acs = ACS712::new();
    assert!(!acs.is_overcurrent(2499, 2500));
    assert!(!acs.is_overcurrent(-2499, 2500));
    assert!(!acs.is_overcurrent(0, 2500));
}

#[test]
fn test_acs712_overcurrent_above_threshold() {
    let acs = ACS712::new();
    assert!(acs.is_overcurrent(2501, 2500));
    assert!(acs.is_overcurrent(-2501, 2500));
}

#[test]
fn test_acs712_overcurrent_at_boundary() {
    // Exactamente en el límite no es sobrecorriente (> no >=)
    let acs = ACS712::new();
    assert!(!acs.is_overcurrent(2500, 2500));
}

#[test]
fn test_acs712_custom_zero_mv() {
    // Offset calibrado: si zero_mv = 2480, ADC que da 2480 mV debe ser ~0 mA
    // ADC para 2480 mV: (2480 * 1023) / 5000 = 507
    let acs = ACS712::with_zero_mv(2480);
    let ma = acs.read_ma(507);
    assert!(ma.abs() < 20, "offset calibrado debe dar ~0 mA, got {}", ma);
}

#[test]
fn test_acs712_symmetry() {
    // El sensor es simétrico: la misma desviación en ambos lados da igual magnitud
    let acs = ACS712::new();
    let pos = acs.read_ma(647); //  10 A
    let neg = acs.read_ma(377); // ~-10 A (512 - (647-512) = 377)
    assert!((pos + neg).abs() < 30, "debe ser simétrico: {} + {} ≈ 0", pos, neg);
}

// ─── ACS712-05A ───────────────────────────────────────────────────────────────

#[test]
fn test_acs712_05a_zero_current() {
    // ADC 512 → V = 2500 mV → I = 0 mA (igual que 30A, mismo zero)
    let acs = ACS712::new_05a();
    assert_eq!(acs.read_ma(512), 0);
}

#[test]
fn test_acs712_05a_positive_2a() {
    // 2 A → V = 2500 + 2×185 = 2870 mV → ADC = round(2870×1023/5000) = 587
    let acs = ACS712::new_05a();
    let ma = acs.read_ma(587);
    assert!((ma - 2000).abs() < 30, "esperado ~2000 mA, got {}", ma);
}

#[test]
fn test_acs712_05a_mayor_resolucion_que_30a() {
    // Mejor resolución = 1 count de ADC representa menos mA.
    // 05A: (1×5_000_000)/(1023×185) ≈ 26 mA/count
    // 30A: (1×5_000_000)/(1023×66)  ≈ 74 mA/count
    let acs_05 = ACS712::new_05a();
    let acs_30 = ACS712::new_30a();
    let step_05 = acs_05.read_ma(513).abs(); // 1 count sobre zero (ADC 512)
    let step_30 = acs_30.read_ma(513).abs();
    assert!(step_05 < step_30,
        "05A debe tener menor mA/count (mejor resolución): 05A={} 30A={}", step_05, step_30);
}

#[test]
fn test_acs712_calibrate_zero_builder() {
    // new_05a().calibrate_zero(zero_mv) → lectura en zero_adc debe ser ~0 mA
    // V(580) = (580×5000)/1023 = 2835 mV
    let acs = ACS712::new_05a().calibrate_zero(2835);
    let ma = acs.read_ma(580);
    assert!(ma.abs() < 15, "calibrado debe dar ~0 mA, got {}", ma);
}

#[test]
fn test_acs712_new_es_alias_30a() {
    // new() y new_30a() deben dar el mismo resultado
    let a = ACS712::new();
    let b = ACS712::new_30a();
    assert_eq!(a.read_ma(647), b.read_ma(647));
}

// ─── LM335 ────────────────────────────────────────────────────────────────────

#[test]
fn test_lm335_25_celsius() {
    // 25 °C = 298 K → V = 2981 mV → ADC = (2981×1023)/5000 = 610
    let lm = LM335::new();
    let t = lm.read_celsius(610);
    assert!((t - 25).abs() <= 1, "esperado ~25 °C, got {}", t);
}

#[test]
fn test_lm335_0_celsius() {
    // 0 °C = 273 K → V = 2731 mV → ADC = (2731×1023)/5000 = 559
    let lm = LM335::new();
    let t = lm.read_celsius(559);
    assert!((t - 0).abs() <= 1, "esperado ~0 °C, got {}", t);
}

#[test]
fn test_lm335_100_celsius() {
    // 100 °C = 373 K → V = 3731 mV → ADC = (3731×1023)/5000 = 764
    let lm = LM335::new();
    let t = lm.read_celsius(764);
    assert!((t - 100).abs() <= 1, "esperado ~100 °C, got {}", t);
}

#[test]
fn test_lm335_kelvin_25c() {
    // 25 °C = 298 K
    let lm = LM335::new();
    let k = lm.read_kelvin(610);
    assert!((k - 298).abs() <= 1, "esperado ~298 K, got {}", k);
}

#[test]
fn test_lm335_celsius_equals_kelvin_minus_273() {
    let lm = LM335::new();
    let adc = 610u16;
    assert_eq!(lm.read_celsius(adc), lm.read_kelvin(adc) - 273);
}

#[test]
fn test_lm335_with_positive_offset() {
    // Sin offset: ADC 610 → 25 °C
    // Con offset +3: debe dar 28 °C
    let lm = LM335::with_offset(3);
    let t = lm.read_celsius(610);
    assert!((t - 28).abs() <= 1, "esperado ~28 °C con offset +3, got {}", t);
}

#[test]
fn test_lm335_with_negative_offset() {
    // Con offset -3: debe dar 22 °C
    let lm = LM335::with_offset(-3);
    let t = lm.read_celsius(610);
    assert!((t - 22).abs() <= 1, "esperado ~22 °C con offset -3, got {}", t);
}

#[test]
fn test_lm335_zero_offset_unchanged() {
    let lm_default = LM335::new();
    let lm_zero    = LM335::with_offset(0);
    assert_eq!(lm_default.read_celsius(610), lm_zero.read_celsius(610));
}
