// tests/ramp_test.rs — Tests unitarios para DriveRamp
//
// Ejecutar con:
//   cargo test --test ramp_test --no-default-features --target x86_64-unknown-linux-gnu
//
// Estos tests verifican que la rampa de velocidad cumple:
//   - RF-005: soft-stop desde 100% alcanza 0 en ≤ 500 ms (10 ticks × 20 ms = 200 ms)
//   - hard_stop() resetea inmediatamente (0 ticks)
//   - La interpolación es simétrica para aceleración y desaceleración
//   - step_toward no supera el objetivo (no overshoot)

use rover_low_level_controller::ramp::DriveRamp;

const STEP: i16 = 10; // Mismo valor que RAMP_STEP_SOFT en config.rs

// ─── Soft-stop ────────────────────────────────────────────────────────────────

#[test]
fn soft_stop_desde_cien_en_diez_ticks() {
    let mut ramp = DriveRamp::new();
    // Simula rover a 100% velocidad
    ramp.actual_l = 100;
    ramp.actual_r = 100;

    let mut ticks = 0u32;
    loop {
        let (l, r) = ramp.step(0, 0, STEP);
        ticks += 1;
        if l == 0 && r == 0 { break; }
        assert!(ticks <= 10, "soft-stop excedió 10 ticks (>200 ms)");
    }
    assert_eq!(ticks, 10, "100% → 0 debe tardar exactamente 10 ticks");
}

#[test]
fn soft_stop_desde_sesenta_en_seis_ticks() {
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 60;
    ramp.actual_r = 60;

    let mut ticks = 0u32;
    loop {
        let (l, r) = ramp.step(0, 0, STEP);
        ticks += 1;
        if l == 0 && r == 0 { break; }
        assert!(ticks <= 6);
    }
    assert_eq!(ticks, 6);
}

#[test]
fn soft_stop_desde_cincuenta_en_cinco_ticks() {
    // RET speed = 50 %
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 50;
    ramp.actual_r = 50;

    for i in 1..=5u32 {
        let (l, r) = ramp.step(0, 0, STEP);
        if i < 5 {
            assert!(l > 0 || r > 0, "tick {} no debería ser 0 aún", i);
        } else {
            assert_eq!((l, r), (0, 0), "tick 5 debe alcanzar 0");
        }
    }
}

// ─── Hard stop ────────────────────────────────────────────────────────────────

#[test]
fn hard_stop_es_inmediato() {
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 100;
    ramp.actual_r = -80;

    ramp.hard_stop();

    assert_eq!(ramp.actual_l, 0);
    assert_eq!(ramp.actual_r, 0);
    // Siguiente step con target ≠ 0 debe ignorar actual si el paso es suficiente
    // (hard_stop resetea el estado; la rampa sube desde 0 si hay nuevo target)
    let (l, r) = ramp.step(0, 0, STEP);
    assert_eq!((l, r), (0, 0));
}

#[test]
fn hard_stop_en_velocidad_negativa() {
    let mut ramp = DriveRamp::new();
    ramp.actual_l = -50;
    ramp.actual_r = -50;

    ramp.hard_stop();
    assert_eq!(ramp.actual_l, 0);
    assert_eq!(ramp.actual_r, 0);
}

// ─── Aceleración (soft-start) ─────────────────────────────────────────────────

#[test]
fn soft_start_de_cero_a_cien() {
    let mut ramp = DriveRamp::new(); // actual = 0

    let mut ticks = 0u32;
    loop {
        let (l, r) = ramp.step(100, 100, STEP);
        ticks += 1;
        if l == 100 && r == 100 { break; }
        assert!(ticks <= 10);
    }
    assert_eq!(ticks, 10);
}

#[test]
fn soft_start_simetrico_con_soft_stop() {
    // Aceleración y desaceleración deben tomar el mismo número de ticks
    let mut ramp_up   = DriveRamp::new();
    let mut ramp_down = DriveRamp::new();
    ramp_down.actual_l = 100;
    ramp_down.actual_r = 100;

    let mut ticks_up   = 0u32;
    let mut ticks_down = 0u32;

    loop {
        let (l, _) = ramp_up.step(100, 100, STEP);
        ticks_up += 1;
        if l == 100 { break; }
    }
    loop {
        let (l, _) = ramp_down.step(0, 0, STEP);
        ticks_down += 1;
        if l == 0 { break; }
    }
    assert_eq!(ticks_up, ticks_down, "aceleración y desaceleración deben ser simétricas");
}

// ─── Sin overshoot ────────────────────────────────────────────────────────────

#[test]
fn no_overshoot_con_step_grande() {
    // Si el target está a menos de step, debe aterrizar exactamente en target
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 5; // a 5 del target 0, step = 10 → debe llegar a 0 en 1 tick
    ramp.actual_r = 3;

    let (l, r) = ramp.step(0, 0, STEP);
    assert_eq!(l, 0, "no debe overshoot en L");
    assert_eq!(r, 0, "no debe overshoot en R");
}

#[test]
fn no_overshoot_en_positivo() {
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 95; // a 5 del target 100
    ramp.actual_r = 98;

    let (l, r) = ramp.step(100, 100, STEP);
    assert_eq!(l, 100);
    assert_eq!(r, 100);
}

// ─── Canales independientes ───────────────────────────────────────────────────

#[test]
fn canales_izq_der_son_independientes() {
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 60;
    ramp.actual_r = 0;

    // Izquierda bajando a 0, derecha subiendo a 60
    let (l, r) = ramp.step(0, 60, STEP);
    assert_eq!(l, 50, "L debe bajar de 60 a 50");
    assert_eq!(r, 10, "R debe subir de 0 a 10");
}

// ─── at_target ────────────────────────────────────────────────────────────────

#[test]
fn at_target_cuando_llega() {
    let mut ramp = DriveRamp::new();
    assert!(ramp.at_target(0, 0), "nueva rampa debe estar en target 0,0");

    ramp.actual_l = 10;
    assert!(!ramp.at_target(0, 0));

    ramp.step(0, 0, STEP);
    assert!(ramp.at_target(0, 0));
}

// ─── Cambio de dirección ─────────────────────────────────────────────────────

#[test]
fn cambio_de_avance_a_retroceso() {
    // AVD → RET: izquierda pasa de +60 a -50 suavemente
    let mut ramp = DriveRamp::new();
    ramp.actual_l = 60;
    ramp.actual_r = -60;

    // Dirección: target = (-50, -50)
    // L: 60 → 50 → ... → -50 (110 pasos, 110 ticks)
    let (l, r) = ramp.step(-50, -50, STEP);
    assert_eq!(l, 50,  "L debe bajar de 60 a 50 en primer tick");
    assert_eq!(r, -60 + STEP, "R debe subir de -60 a -50 en primer tick");
}
