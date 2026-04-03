// Version: v1.0
//! # Configuración del firmware — parámetros ajustables en tiempo de compilación
//!
//! Centraliza todas las constantes que afectan el comportamiento del rover.
//! Para cambiar un parámetro basta modificar este archivo y recompilar;
//! no se requieren cambios en `main.rs`.
//!
//! ## Variantes de hardware
//! Los umbrales de sobrecorriente se seleccionan en tiempo de compilación
//! mediante features de Cargo (ver `Cargo.toml`):
//!
//! | Feature         | Motores          | Umbral OC      |
//! |-----------------|------------------|----------------|
//! | (default)       | L298N × 6        | OC_*_L298N × 6 |
//! | `mixed-drivers` | FR/FL=L298N, resto=BTS7960 | mixto |
//! | `all-bts7960`   | BTS7960 × 6      | OC_*_BTS × 6  |

// ─── Loop principal ───────────────────────────────────────────────────────────

/// Duración de cada ciclo del loop principal en ms.
pub const LOOP_MS: u32 = 20;

/// Ciclos entre emisiones de telemetría periódica (~1 s a 20 ms/ciclo).
pub const TLM_PERIOD: u8 = 50;

/// Tamaño del buffer de respuesta ASCII en bytes.
/// TLM extendido: `TLM:NORMAL:000000:99999ms:36000mV:±32768mA:±30000×6:100C:100×6C:8189mm:±2147483647:±2147483647\n` ≈ 185 bytes.
pub const RESP_BUF: usize = 200;

// ─── Sensores de proximidad ──────────────────────────────────────────────────

/// Cada cuántos ciclos leer el HC-SR04 (~100 ms a 20 ms/ciclo).
/// El driver es bloqueante; ver `docs/consideration_implementation.md §5`.
pub const HC_READ_PERIOD: u8 = 5;

/// Distancia de emergencia HC-SR04 en mm. Por debajo → FAULT inmediato.
pub const HC_EMERGENCY_MM: u16 = 200;

/// Timeout de eco HC-SR04 en µs.
///
/// Limita el bloqueo del loop principal. Sin límite, un obstáculo a ~4 m provoca
/// ~23 ms de busy-wait, excediendo el ciclo de 20 ms. Con este valor solo se
/// espera el eco de objetos hasta ~300 mm (1.5× `HC_EMERGENCY_MM`), reduciendo
/// el bloqueo máximo de ~30 ms a ~1.75 ms.
///
/// Fórmula: `distance_mm × 10_000 / 1_715`. 300 mm → 1 749 µs.
pub const HC_ECHO_TIMEOUT_US: u32 = 1_750;

/// Distancia de emergencia VL53L0X en mm. Por debajo → FAULT inmediato.
/// Umbral más ajustado que HC-SR04 gracias a la mayor precisión del ToF láser.
pub const TOF_EMERGENCY_MM: u16 = 150;

// ─── Detección de stall ──────────────────────────────────────────────────────

/// Ciclos sin movimiento de encoder para declarar stall (~1 s a 20 ms/ciclo).
pub const STALL_THRESHOLD: u16 = 50;

/// Velocidad mínima absoluta (%) para activar la detección de stall.
/// Por debajo se asume que el motor está intencionalmente parado.
pub const STALL_SPEED_MIN: i16 = 20;

// ─── Muestreo ADC ────────────────────────────────────────────────────────────

/// Cada cuántos ciclos leer sensores analógicos completos (~500 ms).
/// Lectura precisa para niveles Warn/Limit y para el frame TLM.
pub const SEN_READ_PERIOD: u8 = 25;

/// Cada cuántos ciclos ejecutar el chequeo rápido de sobrecorriente (~60 ms).
/// Solo detecta nivel Fault; usa menos muestras para reducir latencia.
pub const SEN_FAST_PERIOD: u8 = 3;

/// Muestras ADC por canal en la lectura lenta (8 × ~104 µs ≈ 832 µs/canal).
pub const SEN_SAMPLES: u8 = 8;

/// Muestras ADC por canal en el chequeo rápido (2 × ~104 µs ≈ 208 µs/canal).
pub const SEN_FAST_SAMPLES: u8 = 2;

// ─── Hardware de monitorización ──────────────────────────────────────────────

/// Resistencia de shunt del INA226 en mΩ.
/// Ajustar según el componente físico instalado (10 mΩ = 0.01 Ω = R010).
/// Calibrar antes de confiar en las lecturas de corriente.
pub const INA226_SHUNT_MOHM: u16 = 10;

// ─── Protección de corriente ─────────────────────────────────────────────────

/// Velocidad máxima (%) aplicada a todos los motores cuando `safety == Limit`.
pub const LIMIT_SPEED_CAP: i16 = 60;

/// Umbrales para driver L298N (2 A continuo / 3 A pico).
pub const OC_WARN_L298N:  i32 = 1_200; // 60 % de 2 A
pub const OC_LIMIT_L298N: i32 = 1_600; // 80 % de 2 A
pub const OC_FAULT_L298N: i32 = 2_000; // 100 % de 2 A

/// Umbrales para driver BTS7960 (43 A pico — indicativos, calibrar con motor real).
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
pub const OC_WARN_BTS:  i32 = 8_000;  // ~20 % del pico
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
pub const OC_LIMIT_BTS: i32 = 12_000; // ~28 % del pico
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
pub const OC_FAULT_BTS: i32 = 15_000; // ~35 % del pico

/// Umbrales de sobrecorriente por motor `[FR, FL, CR, CL, RR, RL]`.
/// Seleccionados en tiempo de compilación según el feature activo.
#[cfg(feature = "mixed-drivers")]
pub const OC_WARN:  [i32; 6] = [OC_WARN_L298N,  OC_WARN_L298N,  OC_WARN_BTS,  OC_WARN_BTS,  OC_WARN_BTS,  OC_WARN_BTS];
#[cfg(feature = "mixed-drivers")]
pub const OC_LIMIT: [i32; 6] = [OC_LIMIT_L298N, OC_LIMIT_L298N, OC_LIMIT_BTS, OC_LIMIT_BTS, OC_LIMIT_BTS, OC_LIMIT_BTS];
#[cfg(feature = "mixed-drivers")]
pub const OC_FAULT: [i32; 6] = [OC_FAULT_L298N, OC_FAULT_L298N, OC_FAULT_BTS, OC_FAULT_BTS, OC_FAULT_BTS, OC_FAULT_BTS];

#[cfg(feature = "all-bts7960")]
pub const OC_WARN:  [i32; 6] = [OC_WARN_BTS;  6];
#[cfg(feature = "all-bts7960")]
pub const OC_LIMIT: [i32; 6] = [OC_LIMIT_BTS; 6];
#[cfg(feature = "all-bts7960")]
pub const OC_FAULT: [i32; 6] = [OC_FAULT_BTS; 6];

#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
pub const OC_WARN:  [i32; 6] = [OC_WARN_L298N;  6];
#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
pub const OC_LIMIT: [i32; 6] = [OC_LIMIT_L298N; 6];
#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
pub const OC_FAULT: [i32; 6] = [OC_FAULT_L298N; 6];

// ─── Protección térmica de batería ────────────────────────────────────────────

/// Umbrales de temperatura de celdas 18650 en °C.
/// El thermal runaway inicia ~80–90 °C; estos márgenes son conservadores.
pub const BATT_WARN_C:  i32 = 45; // operación prolongada a alta carga
pub const BATT_LIMIT_C: i32 = 55; // reducir velocidad
pub const BATT_FAULT_C: i32 = 65; // detener rover — peligro inmediato

// ─── Plausibilidad de sensores de temperatura ─────────────────────────────────
//
// Si una lectura cae fuera de estos rangos, el sensor probablemente está
// desconectado o falla (pin ADC flotante). Ignorar la lectura y emitir
// "WARN:LM335_OOR" / "WARN:NTC_OOR" en lugar de usar el valor basura.
//
// LM335 desconectado (ADC=0): read_celsius(0) = -273 °C → silencioso, jamás alarma.
// NTC desconectado  (ADC flotante alto ~1023): read_celsius(1023) = -20 °C → ídem.

/// Rango plausible de temperatura ambiente para LM335.
pub const AMBIENT_TEMP_MIN_C: i32 = -40;
pub const AMBIENT_TEMP_MAX_C: i32 =  80;

/// Rango plausible de temperatura para sensores NTC de baterías.
pub const BATT_TEMP_MIN_C: i32 = -20;
pub const BATT_TEMP_MAX_C: i32 = 100;

// ─── Odometría (calibrar con hardware real) ───────────────────────────────────
//
// Estos valores son TBD — se calibrarán en campo con el rover real.
// Para calibrar TICKS_PER_REV: elevar el rover, girar una rueda una vuelta
// completa y contar pulsos con `test_encoders`. Repetir 5 veces y promediar.
// Para WHEEL_RADIUS_MM: medir el diámetro de la rueda con calibrador y dividir.
// Para WHEEL_BASE_MM: medir la separación entre centros de ruedas izq/der.

/// Pulsos de encoder por vuelta completa del eje de salida (Phase A solamente).
/// CALIBRAR en campo — NFP-5840-31ZY-EN, ratio de reductora desconocido.
pub const TICKS_PER_REV: u32 = 20; // TBD

/// Radio de la rueda en mm.
/// CALIBRAR en campo — medir con calibrador sobre rueda PLA MAX.
pub const WHEEL_RADIUS_MM: u32 = 50; // TBD

/// Distancia entre centros de contacto de ruedas izquierda y derecha (track width) en mm.
/// CALIBRAR en campo — medir separación física entre ruedas.
pub const WHEEL_BASE_MM: u32 = 280; // TBD
