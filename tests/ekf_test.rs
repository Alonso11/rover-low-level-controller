use rover_low_level_controller::ekf::*;

fn init_test_ekf() -> EkfState {
    EkfState::new(0.01, 0.09) // Incertidumbre inicial: 10cm en XY, ~17deg en Theta
}

#[test]
fn test_straight_line_movement() {
    let mut s = init_test_ekf();
    
    // Simular avance de 100 pulsos por lado (sin aceleración lateral)
    // Con TICKS_PER_REV=20 y WHEEL_RADIUS=50mm:
    // Distancia = (100 / (3*20)) * 2 * PI * 0.05 ≈ 0.523 metros
    predict(&mut s, 100, 100, 0.0, 9.81); 
    
    assert!(s.x > 0.5, "X debería ser > 0.5m, actual: {}", s.x);
    assert!(s.y.abs() < 1e-5, "Y debería ser ~0 en línea recta");
    assert!(s.theta.abs() < 1e-5, "Theta debería ser ~0 en línea recta");
}

#[test]
fn test_rotation_logic() {
    let mut s = init_test_ekf();
    
    // Giro a la izquierda: rueda derecha avanza, izquierda quieta
    predict(&mut s, 0, 50, 0.0, 9.81);
    
    assert!(s.theta > 0.0, "Theta debería ser positivo (giro antihorario)");
    assert!(s.x > 0.0, "Debería haber avanzado algo en X");
    assert!(s.y > 0.0, "Debería haber avanzado algo en Y");
}

#[test]
fn test_gyro_fusion_correction() {
    let mut s = init_test_ekf();
    
    // 1. Predicción: El encoder dice que giramos (derrape simulado)
    predict(&mut s, 0, 20, 0.0, 9.81);
    let theta_encoder = s.theta;
    
    // 2. Actualización: El giroscopio dice que NO giramos (omega = 0)
    update_gyro(&mut s, 0.0);
    
    // El valor final de theta debe estar entre el inicial (0) y el del encoder,
    // pero MUCHO más cerca de 0 porque el gyro es mucho más preciso.
    assert!(s.theta < theta_encoder, "El gyro debería haber reducido el error del encoder");
    assert!(s.theta.abs() < 0.1 * theta_encoder, "La corrección del gyro debería ser dominante");
}

#[test]
fn test_pitch_inflates_covariance() {
    let mut s = init_test_ekf();
    let p_before = s.p.p22;
    
    // Simular inclinación fuerte (Pitch > 0.14 rad) usando acelerómetro
    // ax = g * sin(pitch) -> 9.81 * sin(0.3) ≈ 1.4
    predict(&mut s, 10, 10, 1.5, 9.5); 
    
    let p_after = s.p.p22;
    assert!(p_after > p_before, "La incertidumbre en Theta debería crecer en pendientes");
}
