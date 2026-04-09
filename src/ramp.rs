// Version: v1.0
//! # Rampa de velocidad — soft-stop / soft-start
//!
//! ## Problema
//! Al detener un motor DC en plena velocidad, la energía cinética almacenada
//! se convierte en una corriente inversa (back-EMF). Con el driver L298N en
//! modo BRAKE (IN1=IN2=LOW ambos transistores inferiores conducen, cortocircuito
//! en bornes del motor) el transitorio de voltaje puede superar brevemente el
//! rail de alimentación. Con el BTS7960 en COAST (RPWM=LPWM=0) el efecto es
//! menor gracias a los diodos flyback internos del chip, pero sigue presente.
//!
//! Este transitorio —acumulado en cada frenada brusca— degrada el esmalte
//! aislante de las bobinas y puede causar fallos prematuros en los drivers.
//!
//! ## Solución
//! En lugar de cambiar el duty cycle de golpe, interpolamos linealmente entre
//! la velocidad actual y el objetivo: reducir la velocidad 10 puntos por ciclo
//! de 20 ms es suficiente para que los inductores del motor absorban la energía
//! sin generar un pico significativo.
//!
//! ## Cuándo NO usar la rampa (hard stop)
//! Hay condiciones donde el stop inmediato es obligatorio por seguridad y no
//! puede retrasarse ni 20 ms:
//!
//! | Evento                    | Causa física                   | Método    |
//! |---------------------------|--------------------------------|-----------|
//! | Obstáculo HC-SR04 <200 mm | Colisión inminente             | hard_stop |
//! | Obstáculo VL53L0X <150 mm | Colisión inminente (láser)     | hard_stop |
//! | Sobrecorriente OC_FAULT   | Motor quemándose ahora mismo   | hard_stop |
//! | Stall encoder             | Motor bloqueado, corriente alta| hard_stop |
//!
//! En todos los demás casos (comando GCS, watchdog de comms, cap de velocidad
//! por safety Limit) se usa la rampa suave.
//!
//! ## Cumplimiento RF-005
//! RF-005 exige Fault Stop en ≤500 ms ante slip >25%.
//! Con RAMP_STEP_SOFT=10 y ciclo de 20 ms, el peor caso (100% → 0%) tarda
//! 10 ticks × 20 ms = **200 ms** — holgura de 300 ms respecto al requisito.
//!
//! | Vel. inicial | Ticks | Tiempo | Margen RF-005 |
//! |-------------|-------|--------|---------------|
//! | 100% (EXP)  | 10    | 200 ms | ✓ (300 ms)    |
//! | 60%  (AVD)  |  6    | 120 ms | ✓ (380 ms)    |
//! | 50%  (RET)  |  5    | 100 ms | ✓ (400 ms)    |
//!
//! ## Nota sobre protección de hardware complementaria
//! La rampa software reduce el di/dt pero no elimina completamente el transitorio.
//! Se recomienda instalar un capacitor de desacoplo (470 µF electrolítico +
//! 100 nF cerámico en paralelo) directamente en los terminales de alimentación
//! de cada driver L298N/BTS7960 para absorber el remanente.

/// Interpolador lineal de velocidad para los dos canales de tracción (izq/der).
///
/// Mantiene las velocidades *reales aplicadas* al rover, distintas de las
/// velocidades *objetivo* que define el MSM. Cada tick del loop principal
/// se llama a `step()` para avanzar un paso hacia el objetivo.
pub struct DriveRamp {
    /// Velocidad real actualmente aplicada al lado izquierdo (−100…100 %).
    pub actual_l: i16,
    /// Velocidad real actualmente aplicada al lado derecho (−100…100 %).
    pub actual_r: i16,
}

impl DriveRamp {
    /// Construye la rampa con ambos canales en reposo.
    pub const fn new() -> Self {
        Self { actual_l: 0, actual_r: 0 }
    }

    /// Avanza un paso de tamaño máximo `step` hacia `(target_l, target_r)`.
    ///
    /// Llamar exactamente una vez por ciclo del loop principal (20 ms).
    /// Retorna las velocidades reales a aplicar en este tick.
    ///
    /// # Parámetros
    /// - `target_l / target_r`: velocidad objetivo del MSM para cada lado.
    /// - `step`: incremento máximo absoluto por tick (positivo).
    pub fn step(&mut self, target_l: i16, target_r: i16, step: i16) -> (i16, i16) {
        self.actual_l = step_toward(self.actual_l, target_l, step);
        self.actual_r = step_toward(self.actual_r, target_r, step);
        (self.actual_l, self.actual_r)
    }

    /// Detiene ambos canales **inmediatamente**, sin pasar por la rampa.
    ///
    /// Usar exclusivamente en condiciones de seguridad: obstáculo detectado,
    /// sobrecorriente, o stall de encoder. Ver tabla de eventos en el módulo.
    pub fn hard_stop(&mut self) {
        self.actual_l = 0;
        self.actual_r = 0;
    }

    /// Retorna `true` si ambos canales ya alcanzaron el objetivo.
    ///
    /// Útil para saber cuándo la rampa de bajada completó el stop.
    pub fn at_target(&self, target_l: i16, target_r: i16) -> bool {
        self.actual_l == target_l && self.actual_r == target_r
    }
}

/// Mueve `current` hacia `target` en pasos de `step` como máximo.
fn step_toward(current: i16, target: i16, step: i16) -> i16 {
    let diff = target - current;
    if diff == 0          { return current; }
    if diff.abs() <= step { return target;  }
    if diff > 0 { current + step } else { current - step }
}
