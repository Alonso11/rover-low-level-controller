// Version: v1.3
// Pruebas unitarias de los drivers analógicos ACS712, LM335 y NTCThermistor,
// y de la función de plausibilidad check_temp_c.
// Se ejecutan en PC sin necesidad del Arduino.
// Comando: ./test_native.sh  (o ver test_native.sh para el comando completo)

use rover_low_level_controller::sensors::{ACS712, LM335, NTCThermistor, check_temp_c};

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

// ─── ACS712 — casos adicionales ──────────────────────────────────────────────

#[test]
fn test_acs712_05a_negative_current() {
    // ACS712-05A también soporta corriente negativa
    // −2 A → V = 2500 − 2×185 = 2130 mV → ADC = (2130×1023)/5000 = 436
    let acs = ACS712::new_05a();
    let ma = acs.read_ma(436);
    assert!((ma + 2000).abs() < 30, "esperado ~−2000 mA, got {}", ma);
}

#[test]
fn test_acs712_with_zero_mv_usa_sensibilidad_30a() {
    // with_zero_mv siempre crea instancia 30A (66 mV/A, ~74 mA/count)
    // 05A sería 185 mV/A (~27 mA/count) → menor valor por count
    let acs_custom = ACS712::with_zero_mv(2500);
    let acs_05     = ACS712::new_05a();
    let step_custom = acs_custom.read_ma(513).abs(); // 1 count sobre zero
    let step_05     = acs_05.read_ma(513).abs();
    assert!(step_custom > step_05,
        "with_zero_mv debe usar sensibilidad 30A (más mA/count): custom={} 05a={}", step_custom, step_05);
}

#[test]
fn test_acs712_calibrate_zero_preserva_sensibilidad_05a() {
    // calibrate_zero() en una instancia 05A no cambia la sensibilidad
    let acs_cal = ACS712::new_05a().calibrate_zero(2480);
    let acs_30  = ACS712::with_zero_mv(2480); // 30A, mismo zero
    let adc = 525u16;
    // 05A da menos mA/count que 30A (más sensible al voltaje, menor corriente)
    assert!(acs_cal.read_ma(adc).abs() < acs_30.read_ma(adc).abs(),
        "05A calibrado debe dar menos mA/count que 30A");
}

#[test]
fn test_acs712_overcurrent_umbral_cero() {
    // Cualquier corriente no nula supera un umbral de 0 mA
    let acs = ACS712::new();
    assert!(acs.is_overcurrent(1, 0));
    assert!(acs.is_overcurrent(-1, 0));
    assert!(!acs.is_overcurrent(0, 0)); // exactamente 0 no supera > 0
}

#[test]
fn test_acs712_formula_rounding_en_zero_adc() {
    // zero_adc = (2500*1023 + 2500) / 5000 = 2560000/5000 = 512
    // ADC=512 con zero_mv=2500 debe dar exactamente 0 mA
    let acs = ACS712::new();
    assert_eq!(acs.read_ma(512), 0);
    // Verificar también con 05A
    let acs_05 = ACS712::new_05a();
    assert_eq!(acs_05.read_ma(512), 0);
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

// ─── LM335 — casos adicionales ───────────────────────────────────────────────

#[test]
fn test_lm335_negative_celsius() {
    // −10 °C = 263 K → V = 2630 mV → ADC = (2630×1023)/5000 = 538
    // V_recalc = (538×5000)/1023 = 2629 mV → T_K = 262 → T_C = −11
    // La truncación entera introduce 1 K de error — toleramos ±2 °C
    let lm = LM335::new();
    let t = lm.read_celsius(538);
    assert!((t - (-10)).abs() <= 2, "esperado ~−10 °C, got {}", t);
}

#[test]
fn test_lm335_adc_zero_extremo() {
    // ADC=0 → V=0 → T=0 K → T=−273 °C
    let lm = LM335::new();
    assert_eq!(lm.read_celsius(0), -273);
}

#[test]
fn test_lm335_adc_max_extremo() {
    // ADC=1023 → V=5000 mV → T=500 K → T=227 °C
    let lm = LM335::new();
    assert_eq!(lm.read_celsius(1023), 227);
}

#[test]
fn test_lm335_monotone() {
    // Temperatura aumenta estrictamente con ADC
    let lm = LM335::new();
    let t1 = lm.read_celsius(400);
    let t2 = lm.read_celsius(600);
    let t3 = lm.read_celsius(800);
    assert!(t1 < t2, "ADC 400 debe dar menos que ADC 600: {} < {}", t1, t2);
    assert!(t2 < t3, "ADC 600 debe dar menos que ADC 800: {} < {}", t2, t3);
}

#[test]
fn test_lm335_offset_en_temperatura_negativa() {
    // Offset también se aplica en temperaturas negativas
    let base = LM335::new().read_celsius(538);
    let con_offset = LM335::with_offset(5).read_celsius(538);
    assert_eq!(con_offset, base + 5);
}

// ─── NTCThermistor — AD36958 (B=3950, R25=10kΩ, Rpull=10kΩ) ─────────────────

#[test]
fn test_ntc_at_25c() {
    // ADC 512 → punto exacto de la tabla → 25 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(512);
    assert!((t - 25).abs() <= 1, "esperado ~25 °C, got {}", t);
}

#[test]
fn test_ntc_at_0c() {
    // ADC 787 → punto exacto de la tabla → 0 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(787);
    assert!((t - 0).abs() <= 1, "esperado ~0 °C, got {}", t);
}

#[test]
fn test_ntc_at_50c() {
    // ADC 268 → punto exacto de la tabla → 50 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(268);
    assert!((t - 50).abs() <= 1, "esperado ~50 °C, got {}", t);
}

#[test]
fn test_ntc_at_100c() {
    // ADC 66 → punto exacto de la tabla → 100 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(66);
    assert!((t - 100).abs() <= 1, "esperado ~100 °C, got {}", t);
}

#[test]
fn test_ntc_interpolation() {
    // ADC 480 está entre 512 (25 °C) y 454 (30 °C)
    // t = 25 + (30-25)*(512-480)/(512-454) = 25 + 5*32/58 = 25+2 = 27 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(480);
    assert!((t - 27).abs() <= 1, "esperado ~27 °C interpolado, got {}", t);
}

#[test]
fn test_ntc_saturation_cold() {
    // ADC muy alto (encima del primer punto) → temperatura mínima: −20 °C
    let ntc = NTCThermistor::new();
    assert_eq!(ntc.read_celsius(1023), -20);
    assert_eq!(ntc.read_celsius(934),  -20); // justo en el primer punto
}

#[test]
fn test_ntc_saturation_hot() {
    // ADC muy bajo (debajo del último punto) → temperatura máxima: 100 °C
    let ntc = NTCThermistor::new();
    assert_eq!(ntc.read_celsius(0),  100);
    assert_eq!(ntc.read_celsius(66), 100); // justo en el último punto
}

#[test]
fn test_ntc_with_offset_positive() {
    // ADC 512 → 25 °C sin offset; con offset +3 → 28 °C
    let ntc = NTCThermistor::with_offset(3);
    let t = ntc.read_celsius(512);
    assert!((t - 28).abs() <= 1, "esperado ~28 °C con offset +3, got {}", t);
}

#[test]
fn test_ntc_with_offset_negative() {
    // Con offset −3 → 22 °C
    let ntc = NTCThermistor::with_offset(-3);
    let t = ntc.read_celsius(512);
    assert!((t - 22).abs() <= 1, "esperado ~22 °C con offset -3, got {}", t);
}

#[test]
fn test_ntc_calibrate_builder() {
    // calibrate() debe producir el mismo resultado que with_offset()
    let a = NTCThermistor::new().calibrate(5);
    let b = NTCThermistor::with_offset(5);
    assert_eq!(a.read_celsius(512), b.read_celsius(512));
}

#[test]
fn test_ntc_is_overtemp_true() {
    let ntc = NTCThermistor::new();
    assert!(ntc.is_overtemp(46, 45));
}

#[test]
fn test_ntc_is_overtemp_false() {
    let ntc = NTCThermistor::new();
    assert!(!ntc.is_overtemp(44, 45));
}

#[test]
fn test_ntc_is_overtemp_boundary() {
    // Exactamente en el umbral no es sobretemperatura (> no >=)
    let ntc = NTCThermistor::new();
    assert!(!ntc.is_overtemp(45, 45));
}

#[test]
fn test_ntc_temp_increases_as_adc_decreases() {
    // Propiedad fundamental: ADC menor → temperatura mayor
    let ntc = NTCThermistor::new();
    assert!(ntc.read_celsius(300) > ntc.read_celsius(500));
}

#[test]
fn test_ntc_at_minus_10c() {
    // ADC 874 → punto exacto de la tabla → −10 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(874);
    assert!((t - (-10)).abs() <= 1, "esperado ~−10 °C, got {}", t);
}

#[test]
fn test_ntc_at_minus_20c() {
    // ADC 934 → punto exacto de la tabla → −20 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(934);
    assert!((t - (-20)).abs() <= 1, "esperado ~−20 °C, got {}", t);
}

#[test]
fn test_ntc_at_10c() {
    // ADC 682 → punto exacto de la tabla → 10 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(682);
    assert!((t - 10).abs() <= 1, "esperado ~10 °C, got {}", t);
}

#[test]
fn test_ntc_at_20c() {
    // ADC 568 → punto exacto de la tabla → 20 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(568);
    assert!((t - 20).abs() <= 1, "esperado ~20 °C, got {}", t);
}

#[test]
fn test_ntc_interpolation_extremo_frio() {
    // ADC 900 entre 934 (−20 °C) y 874 (−10 °C):
    // t = −20 + (−10−(−20)) × (934−900) / (934−874) = −20 + 10×34/60 = −20+5 = −15 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(900);
    assert!((t - (-15)).abs() <= 1, "esperado ~−15 °C interpolado, got {}", t);
}

#[test]
fn test_ntc_interpolation_extremo_caliente() {
    // ADC 75 entre 87 (90 °C) y 66 (100 °C):
    // t = 90 + (100−90) × (87−75) / (87−66) = 90 + 10×12/21 = 90+5 = 95 °C
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(75);
    assert!((t - 95).abs() <= 1, "esperado ~95 °C interpolado, got {}", t);
}

#[test]
fn test_ntc_adc_clamping_por_encima_de_1023() {
    // ADC > 1023 se clampea a 1023 → por encima del primer punto de tabla → −20 °C
    let ntc = NTCThermistor::new();
    assert_eq!(ntc.read_celsius(2000), ntc.read_celsius(1023));
    assert_eq!(ntc.read_celsius(1023), -20);
}

#[test]
fn test_ntc_with_offset_zero_igual_que_new() {
    let a = NTCThermistor::new();
    let b = NTCThermistor::with_offset(0);
    assert_eq!(a.read_celsius(512), b.read_celsius(512));
    assert_eq!(a.read_celsius(268), b.read_celsius(268)); // 50 °C
}

#[test]
fn test_ntc_offset_en_temperatura_alta() {
    // El offset se aplica también en temperaturas altas (no solo a 25 °C)
    let base       = NTCThermistor::new().read_celsius(66);   // 100 °C
    let con_offset = NTCThermistor::with_offset(3).read_celsius(66);
    assert_eq!(con_offset, base + 3);
}

#[test]
fn test_ntc_batt_thresholds_order() {
    // Verificar que los umbrales de batería tienen sentido (Warn < Limit < Fault)
    // Los ADC correspondientes: Warn=45°C→308, Limit=55°C→~233, Fault=65°C→~175
    let ntc = NTCThermistor::new();
    let warn_t  = ntc.read_celsius(308); // ~45 °C
    let limit_t = ntc.read_celsius(233); // ~55 °C
    let fault_t = ntc.read_celsius(175); // ~65 °C
    assert!(warn_t < limit_t, "Warn debe ser < Limit");
    assert!(limit_t < fault_t, "Limit debe ser < Fault");
}

// ─── check_temp_c — validación de plausibilidad ──────────────────────────────

#[test]
fn test_check_temp_c_valor_normal() {
    // Temperatura dentro del rango → Some(t)
    assert_eq!(check_temp_c(25, -40, 80), Some(25));
    assert_eq!(check_temp_c(0,  -40, 80), Some(0));
    assert_eq!(check_temp_c(-40, -40, 80), Some(-40)); // límite inferior inclusivo
    assert_eq!(check_temp_c(80,  -40, 80), Some(80));  // límite superior inclusivo
}

#[test]
fn test_check_temp_c_lm335_desconectado() {
    // LM335 con pin ADC=0 → read_celsius(0) = -273 °C → fuera de rango → None
    let lm = LM335::new();
    let t = lm.read_celsius(0);
    assert_eq!(t, -273);
    assert_eq!(check_temp_c(t, -40, 80), None, "LM335 desconectado debe dar None");
}

#[test]
fn test_check_temp_c_lm335_adc_max() {
    // LM335 con ADC=1023 → read_celsius(1023) = 227 °C → fuera de rango → None
    let lm = LM335::new();
    let t = lm.read_celsius(1023);
    assert_eq!(t, 227);
    assert_eq!(check_temp_c(t, -40, 80), None, "LM335 saturado debe dar None");
}

#[test]
fn test_check_temp_c_ntc_desconectado_frio() {
    // NTC con pin ADC flotante alto (>934) → read_celsius satura a -20 °C
    // Con BATT_TEMP_MIN_C = -20 el límite es inclusivo → Some(-20)
    // (diseño explícito: -20 está en tabla de NTC, es legítima)
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(1023);
    assert_eq!(t, -20);
    assert_eq!(check_temp_c(t, -20, 100), Some(-20));
}

#[test]
fn test_check_temp_c_ntc_desconectado_caliente() {
    // NTC con pin ADC=0 → read_celsius(0) = 100 °C → límite superior inclusivo → Some(100)
    let ntc = NTCThermistor::new();
    let t = ntc.read_celsius(0);
    assert_eq!(t, 100);
    assert_eq!(check_temp_c(t, -20, 100), Some(100));
}

#[test]
fn test_check_temp_c_fuera_por_arriba() {
    assert_eq!(check_temp_c(101, -20, 100), None);
    assert_eq!(check_temp_c(200, -20, 100), None);
}

#[test]
fn test_check_temp_c_fuera_por_abajo() {
    assert_eq!(check_temp_c(-21, -20, 100), None);
    assert_eq!(check_temp_c(-273, -20, 100), None);
}
