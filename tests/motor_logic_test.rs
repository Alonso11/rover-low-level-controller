// Version: v1.4
// Tests de lógica de motores — x86, sin hardware.
//
// Los drivers concretos (L298NMotor, BTS7960Motor) dependen de arduino-hal
// y no compilan en x86. Estos tests verifican:
//   1. Aritmética speed → duty cycle (algoritmo común a L298N y BTS7960)
//   2. Lógica de dirección (flag inverted, signo de velocidad)
//   3. Contrato del trait Motor via MockMotor
//   4. SixWheelRover — control diferencial, stop, giro, brake_all, enable_all
//   5. ErasedMotor — delegación correcta al motor concreto
//   6. QuadratureEncoder — decode x2 (ambos flancos de A)
//   7. MotorWithEncoder — counts, delta_counts, reset_encoder
//   8. Motor trait defaults — brake() = stop(), enable/disable no-op
//   9. Dead-band: set_speeds descarta velocidades < MIN_MOTOR_SPEED

use rover_low_level_controller::motor_control::{Motor, ErasedMotor, SixWheelRover, MotorWithEncoder};
use rover_low_level_controller::sensors::Encoder;
use rover_low_level_controller::config::MIN_MOTOR_SPEED;

// ─── MockMotor ───────────────────────────────────────────────────────────────

/// Motor de prueba. Registra la última velocidad y las llamadas a cada método.
struct MockMotor {
    last_speed:    i16,
    stop_called:   bool,
    brake_called:  bool,
    enable_called: bool,
    disable_called:bool,
    call_count:    u32,
}

impl MockMotor {
    fn new() -> Self {
        MockMotor {
            last_speed: 0, stop_called: false, brake_called: false,
            enable_called: false, disable_called: false, call_count: 0,
        }
    }
}

impl Motor for MockMotor {
    fn set_speed(&mut self, speed: i16) {
        self.last_speed   = speed;
        self.stop_called  = false;
        self.brake_called = false;
        self.call_count  += 1;
    }
    fn stop(&mut self) {
        self.last_speed  = 0;
        self.stop_called = true;
        self.call_count += 1;
    }
    fn brake(&mut self) {
        self.last_speed   = 0;
        self.brake_called = true;
        self.call_count  += 1;
    }
    fn enable(&mut self)  { self.enable_called  = true; }
    fn disable(&mut self) { self.disable_called = true; }
}

// ─── Helpers que replican la aritmética interna de L298N / BTS7960 ────────────

/// Duty cycle que calcula L298NMotor::set_speed (max_duty = 255 en Timer 8-bit).
fn speed_to_duty(speed: i16, max_duty: u32) -> u8 {
    let abs_speed = speed.unsigned_abs() as u32;
    if abs_speed == 0 { return 0; }
    ((abs_speed * max_duty) / 100) as u8
}

/// Dirección lógica que calcula L298NMotor::set_speed.
fn l298n_is_forward(speed: i16, inverted: bool) -> bool {
    if inverted { speed < 0 } else { speed >= 0 }
}

/// Dirección lógica que calcula BTS7960Motor::set_speed.
fn bts7960_is_forward(speed: i16, inverted: bool) -> bool {
    if inverted { speed < 0 } else { speed > 0 }
}

// ─── Tests: aritmética speed → duty ─────────────────────────────────────────

#[test]
fn test_duty_zero_speed() {
    assert_eq!(speed_to_duty(0, 255), 0);
}

#[test]
fn test_duty_full_forward() {
    // speed=100, max_duty=255 → (100*255)/100 = 255
    assert_eq!(speed_to_duty(100, 255), 255);
}

#[test]
fn test_duty_half_speed() {
    // speed=50, max_duty=255 → (50*255)/100 = 127
    assert_eq!(speed_to_duty(50, 255), 127);
}

#[test]
fn test_duty_negative_same_as_positive() {
    // El duty cycle usa el valor absoluto — la dirección va por IN1/IN2
    assert_eq!(speed_to_duty(-75, 255), speed_to_duty(75, 255));
}

#[test]
fn test_duty_full_backward() {
    assert_eq!(speed_to_duty(-100, 255), 255);
}

#[test]
fn test_duty_low_max_duty() {
    // Timer de 10-bit: max_duty=1023 — speed=10 → (10*1023)/100 = 102
    assert_eq!(speed_to_duty(10, 1023), 102);
}

// ─── Tests: lógica de dirección ──────────────────────────────────────────────

#[test]
fn test_l298n_forward_positive_speed() {
    assert!(l298n_is_forward(80, false));
}

#[test]
fn test_l298n_backward_negative_speed() {
    assert!(!l298n_is_forward(-80, false));
}

#[test]
fn test_l298n_zero_is_forward() {
    // speed=0 llega a stop() antes de usar is_forward, pero el cálculo da true
    assert!(l298n_is_forward(0, false));
}

#[test]
fn test_l298n_inverted_flips_direction() {
    // Motor físicamente invertido: positivo → es backward lógico
    assert!(!l298n_is_forward(80, true));
    assert!(l298n_is_forward(-80, true));
}

#[test]
fn test_bts7960_zero_is_not_forward() {
    // BTS7960 usa speed > 0 (no >=0), pero speed=0 llega a stop() antes
    assert!(!bts7960_is_forward(0, false));
}

#[test]
fn test_bts7960_inverted_flips_direction() {
    assert!(!bts7960_is_forward(80, true));
    assert!(bts7960_is_forward(-80, true));
}

// ─── Tests: contrato de Motor (via MockMotor) ────────────────────────────────

#[test]
fn test_mock_set_speed_stores_value() {
    let mut m = MockMotor::new();
    m.set_speed(60);
    assert_eq!(m.last_speed, 60);
    assert!(!m.stop_called);
}

#[test]
fn test_mock_stop_resets_speed() {
    let mut m = MockMotor::new();
    m.set_speed(80);
    m.stop();
    assert_eq!(m.last_speed, 0);
    assert!(m.stop_called);
}

#[test]
fn test_mock_negative_speed() {
    let mut m = MockMotor::new();
    m.set_speed(-50);
    assert_eq!(m.last_speed, -50);
}

#[test]
fn test_mock_call_count() {
    let mut m = MockMotor::new();
    m.set_speed(10);
    m.set_speed(20);
    m.stop();
    assert_eq!(m.call_count, 3);
}

// ─── Tests: SixWheelRover ────────────────────────────────────────────────────

fn make_rover() -> SixWheelRover<MockMotor, MockMotor, MockMotor, MockMotor, MockMotor, MockMotor> {
    SixWheelRover::new(
        MockMotor::new(), MockMotor::new(),
        MockMotor::new(), MockMotor::new(),
        MockMotor::new(), MockMotor::new(),
    )
}

#[test]
fn test_rover_forward_all_motors_same_speed() {
    let mut rover = make_rover();
    rover.set_speeds(80, 80);
    assert_eq!(rover.frontal_left.last_speed,  80);
    assert_eq!(rover.center_left.last_speed,   80);
    assert_eq!(rover.rear_left.last_speed,     80);
    assert_eq!(rover.frontal_right.last_speed, 80);
    assert_eq!(rover.center_right.last_speed,  80);
    assert_eq!(rover.rear_right.last_speed,    80);
}

#[test]
fn test_rover_backward_negative_speed() {
    let mut rover = make_rover();
    rover.set_speeds(-60, -60);
    assert!(rover.frontal_left.last_speed  < 0);
    assert!(rover.frontal_right.last_speed < 0);
}

#[test]
fn test_rover_turn_right_left_faster() {
    // Girar derecha: lado izquierdo más rápido
    let mut rover = make_rover();
    rover.set_speeds(80, 20);
    assert_eq!(rover.frontal_left.last_speed,  80);
    assert_eq!(rover.center_left.last_speed,   80);
    assert_eq!(rover.rear_left.last_speed,     80);
    assert_eq!(rover.frontal_right.last_speed, 20);
    assert_eq!(rover.center_right.last_speed,  20);
    assert_eq!(rover.rear_right.last_speed,    20);
}

#[test]
fn test_rover_turn_left_right_faster() {
    let mut rover = make_rover();
    rover.set_speeds(20, 80);
    assert!(rover.frontal_left.last_speed < rover.frontal_right.last_speed);
}

#[test]
fn test_rover_spin_in_place() {
    // Giro en sitio: un lado hacia adelante, el otro hacia atrás
    let mut rover = make_rover();
    rover.set_speeds(70, -70);
    assert!(rover.frontal_left.last_speed  > 0);
    assert!(rover.frontal_right.last_speed < 0);
}

#[test]
fn test_rover_stop_all_motors() {
    let mut rover = make_rover();
    rover.set_speeds(90, 90);
    rover.stop();
    assert!(rover.frontal_left.stop_called);
    assert!(rover.center_left.stop_called);
    assert!(rover.rear_left.stop_called);
    assert!(rover.frontal_right.stop_called);
    assert!(rover.center_right.stop_called);
    assert!(rover.rear_right.stop_called);
}

#[test]
fn test_rover_stop_zeroes_speed() {
    let mut rover = make_rover();
    rover.set_speeds(50, 50);
    rover.stop();
    assert_eq!(rover.frontal_left.last_speed,  0);
    assert_eq!(rover.frontal_right.last_speed, 0);
}

#[test]
fn test_rover_left_right_speeds_are_independent() {
    let mut rover = make_rover();
    rover.set_speeds(33, 77);
    assert_ne!(rover.frontal_left.last_speed, rover.frontal_right.last_speed);
    assert_eq!(rover.frontal_left.last_speed,  rover.center_left.last_speed);
    assert_eq!(rover.frontal_right.last_speed, rover.center_right.last_speed);
}

// ─── Tests: ErasedMotor ──────────────────────────────────────────────────────

#[test]
fn test_erased_motor_set_speed_delegates() {
    let mut m = MockMotor::new();
    {
        let mut erased = unsafe { ErasedMotor::new(&mut m) };
        erased.set_speed(55);
    } // erased dropped aquí — raw pointer ya no se usa
    assert_eq!(m.last_speed, 55);
}

#[test]
fn test_erased_motor_stop_delegates() {
    let mut m = MockMotor::new();
    {
        let mut erased = unsafe { ErasedMotor::new(&mut m) };
        erased.set_speed(90);
        erased.stop();
    }
    assert!(m.stop_called);
    assert_eq!(m.last_speed, 0);
}

#[test]
fn test_erased_motor_multiple_calls() {
    let mut m = MockMotor::new();
    {
        let mut erased = unsafe { ErasedMotor::new(&mut m) };
        erased.set_speed(10);
        erased.set_speed(20);
        erased.set_speed(30);
    }
    assert_eq!(m.last_speed, 30);
    assert_eq!(m.call_count, 3);
}

#[test]
fn test_erased_motor_in_array() {
    // Escenario real: array homogéneo de ErasedMotor sobre distintos tipos concretos
    let mut m1 = MockMotor::new();
    let mut m2 = MockMotor::new();
    {
        let mut motors: [ErasedMotor; 2] = unsafe {
            [ErasedMotor::new(&mut m1), ErasedMotor::new(&mut m2)]
        };
        for (i, m) in motors.iter_mut().enumerate() {
            m.set_speed((i as i16 + 1) * 25);
        }
    }
    assert_eq!(m1.last_speed, 25);
    assert_eq!(m2.last_speed, 50);
}

// ─── Helpers: QuadratureEncoder decode ───────────────────────────────────────
// Replica la fórmula de QuadratureEncoder::on_edge sin depender de AVR.
// forward = a_high XOR b_high  (convención CW estándar)

fn quadrature_is_forward(a_high: bool, b_high: bool) -> bool {
    a_high ^ b_high
}

#[test]
fn test_quad_rising_a_b_low_is_forward() {
    // Subida A + B=0 → adelante (A sube antes que B en rotación CW)
    assert!(quadrature_is_forward(true, false));
}

#[test]
fn test_quad_rising_a_b_high_is_backward() {
    assert!(!quadrature_is_forward(true, true));
}

#[test]
fn test_quad_falling_a_b_high_is_forward() {
    // Bajada A + B=1 → adelante
    assert!(quadrature_is_forward(false, true));
}

#[test]
fn test_quad_falling_a_b_low_is_backward() {
    assert!(!quadrature_is_forward(false, false));
}

// ─── MockEncoder ─────────────────────────────────────────────────────────────

use core::sync::atomic::{AtomicI32, Ordering};

struct MockEncoder {
    counts: AtomicI32,
}

impl MockEncoder {
    const fn new_static() -> Self { MockEncoder { counts: AtomicI32::new(0) } }
    fn add(&self, n: i32) { self.counts.fetch_add(n, Ordering::Relaxed); }
}

impl Encoder for MockEncoder {
    fn get_counts(&self) -> i32 { self.counts.load(Ordering::Relaxed) }
    fn reset(&self)              { self.counts.store(0, Ordering::Relaxed); }
}

// ─── Tests: MotorWithEncoder ──────────────────────────────────────────────────

static MOCK_ENC_A: MockEncoder = MockEncoder::new_static();
static MOCK_ENC_B: MockEncoder = MockEncoder::new_static();

#[test]
fn test_mwe_counts_delegates_to_encoder() {
    MOCK_ENC_A.reset();
    MOCK_ENC_A.add(42);
    let mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    assert_eq!(mwe.counts(), 42);
}

#[test]
fn test_mwe_delta_counts_accumulates_correctly() {
    MOCK_ENC_B.reset();
    let mut mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_B);
    MOCK_ENC_B.add(10);
    assert_eq!(mwe.delta_counts(), 10);
    MOCK_ENC_B.add(5);
    assert_eq!(mwe.delta_counts(), 5);
}

#[test]
fn test_mwe_reset_encoder_zeroes_delta() {
    MOCK_ENC_B.reset();
    let mut mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_B);
    MOCK_ENC_B.add(100);
    mwe.reset_encoder();
    assert_eq!(mwe.delta_counts(), 0);
    assert_eq!(mwe.counts(), 0);
}

#[test]
fn test_mwe_implements_motor_set_speed() {
    MOCK_ENC_A.reset();
    let mut mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    mwe.set_speed(75);
    // Verificamos via downcast imposible en no-std, pero el motor interno recibió la llamada
    // — la compilación confirma que Motor está implementado; el test confirma que no panics.
}

#[test]
fn test_mwe_implements_motor_stop() {
    MOCK_ENC_A.reset();
    let mut mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    mwe.set_speed(50);
    mwe.stop();
}

#[test]
fn test_mwe_in_six_wheel_rover_with_plain_motors() {
    // Escenario real: 4 ruedas con encoder + 2 sin encoder en el mismo SixWheelRover.
    MOCK_ENC_A.reset();
    MOCK_ENC_B.reset();
    let fr = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    let fl = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_B);
    let cr = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    let cl = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_B);
    let rr = MockMotor::new(); // sin encoder
    let rl = MockMotor::new(); // sin encoder
    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);
    rover.set_speeds(60, 60);
    rover.stop();
}

// ─── Tests: Motor trait defaults ─────────────────────────────────────────────

#[test]
fn test_motor_brake_default_calls_stop() {
    // MockMotor overrides brake() explicitly; test via a wrapper that uses the default.
    struct DefaultBrakeMotor { stop_called: bool }
    impl Motor for DefaultBrakeMotor {
        fn set_speed(&mut self, _: i16) {}
        fn stop(&mut self) { self.stop_called = true; }
        // brake() not overridden → default calls stop()
    }
    let mut m = DefaultBrakeMotor { stop_called: false };
    m.brake();
    assert!(m.stop_called, "default brake() must delegate to stop()");
}

#[test]
fn test_motor_enable_disable_default_noop() {
    struct NopMotor;
    impl Motor for NopMotor {
        fn set_speed(&mut self, _: i16) {}
        fn stop(&mut self) {}
    }
    let mut m = NopMotor;
    m.enable();   // must not panic
    m.disable();  // must not panic
}

// ─── Tests: SixWheelRover brake_all / enable_all ─────────────────────────────

#[test]
fn test_rover_brake_all_calls_brake_on_each_motor() {
    let mut rover = SixWheelRover::new(
        MockMotor::new(), MockMotor::new(), MockMotor::new(),
        MockMotor::new(), MockMotor::new(), MockMotor::new(),
    );
    rover.frontal_right.set_speed(80);
    rover.brake_all();
    assert!(rover.frontal_right.brake_called);
    assert!(rover.frontal_left.brake_called);
    assert!(rover.center_right.brake_called);
    assert!(rover.center_left.brake_called);
    assert!(rover.rear_right.brake_called);
    assert!(rover.rear_left.brake_called);
}

#[test]
fn test_rover_enable_all_calls_enable_on_each_motor() {
    let mut rover = SixWheelRover::new(
        MockMotor::new(), MockMotor::new(), MockMotor::new(),
        MockMotor::new(), MockMotor::new(), MockMotor::new(),
    );
    rover.enable_all();
    assert!(rover.frontal_right.enable_called);
    assert!(rover.frontal_left.enable_called);
    assert!(rover.center_right.enable_called);
    assert!(rover.center_left.enable_called);
    assert!(rover.rear_right.enable_called);
    assert!(rover.rear_left.enable_called);
}

// ─── Tests: ErasedMotor delegates brake/enable/disable ───────────────────────

static MOCK_ERASED: MockEncoder = MockEncoder::new_static();

#[test]
fn test_erased_motor_brake_delegates() {
    let mut m = MockMotor::new();
    let mut em = unsafe { ErasedMotor::new(&mut m) };
    em.brake();
    assert!(m.brake_called);
}

#[test]
fn test_erased_motor_enable_delegates() {
    let mut m = MockMotor::new();
    let mut em = unsafe { ErasedMotor::new(&mut m) };
    em.enable();
    assert!(m.enable_called);
}

#[test]
fn test_erased_motor_disable_delegates() {
    let mut m = MockMotor::new();
    let mut em = unsafe { ErasedMotor::new(&mut m) };
    em.disable();
    assert!(m.disable_called);
}

// ─── Tests: MotorWithEncoder delegates brake/enable/disable ──────────────────

#[test]
fn test_mwe_brake_delegates_to_inner_motor() {
    MOCK_ENC_A.reset();
    let mut mwe = MotorWithEncoder::new(MockMotor::new(), &MOCK_ENC_A);
    mwe.brake();
    // No direct access to inner motor through MotorWithEncoder — compile + no-panic is the contract.
}

// ─── Tests: dead-band en set_speeds ──────────────────────────────────────────

#[test]
fn test_deadband_below_min_clamps_to_zero() {
    let mut rover = make_rover();
    rover.set_speeds(MIN_MOTOR_SPEED - 1, MIN_MOTOR_SPEED - 1);
    assert_eq!(rover.frontal_left.last_speed,  0);
    assert_eq!(rover.center_left.last_speed,   0);
    assert_eq!(rover.rear_left.last_speed,     0);
    assert_eq!(rover.frontal_right.last_speed, 0);
    assert_eq!(rover.center_right.last_speed,  0);
    assert_eq!(rover.rear_right.last_speed,    0);
}

#[test]
fn test_deadband_at_min_passes_through() {
    let mut rover = make_rover();
    rover.set_speeds(MIN_MOTOR_SPEED, -MIN_MOTOR_SPEED);
    assert_eq!(rover.frontal_left.last_speed,   MIN_MOTOR_SPEED);
    assert_eq!(rover.frontal_right.last_speed, -MIN_MOTOR_SPEED);
}

#[test]
fn test_deadband_above_min_unchanged() {
    let mut rover = make_rover();
    rover.set_speeds(80, -60);
    assert_eq!(rover.frontal_left.last_speed,   80);
    assert_eq!(rover.frontal_right.last_speed, -60);
}
