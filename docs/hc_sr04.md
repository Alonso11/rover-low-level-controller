# Driver HC-SR04 — Sensor Ultrasónico
<!-- Version: v2.0 -->

Driver en `src/sensors/hc_sr04.rs` (v1.2) para el sensor ultrasónico HC-SR04.
Conectado a **D38 (Trigger)** y **D39 (Echo)** en el Arduino Mega 2560.

---

## Especificaciones técnicas

| Parámetro | Valor |
|-----------|-------|
| Rango de medición | 2 cm – 400 cm |
| Precisión | ±3 mm |
| Ángulo de apertura | ~15° |
| Voltaje de operación | 5V |
| Interfaz | GPIO: Trigger (output) + Echo (input) |
| Tiempo de ciclo mínimo | ≥ 60 ms entre disparos |

---

## API del driver

### Construcción

```rust
// Default: timeout completo de 30 000 µs (~4 m de rango)
let hcsr04 = HCSR04::new(trigger_pin, echo_pin);

// Con timeout reducido para limitar bloqueo del loop (recomendado)
let hcsr04 = HCSR04::new(trigger_pin, echo_pin).with_timeout(HC_ECHO_TIMEOUT_US);
```

### Lectura

```rust
match hcsr04.measure_mm() {
    Ok(mm)                        => { /* distancia válida */ }
    Err(SensorError::Timeout)     => { /* eco fuera del rango configurado */ }
    Err(SensorError::OutOfRange)  => { /* distancia fuera de 2–4 000 mm */ }
    _                             => {}
}
```

El tipo de retorno es `Result<u16, SensorError>`. En `main.rs` se usa con
`if let Ok(mm)` para ignorar los errores transitórios y solo actuar cuando
hay una lectura válida.

---

## Control de latencia — `with_timeout()`

La espera del eco es bloqueante (busy-wait con `delay_us(1)` por iteración).
Sin límite, un obstáculo a 4 m causa ~23 ms de bloqueo, excediendo el ciclo
de 20 ms del loop principal.

`with_timeout(µs)` limita la espera al rango que interesa:

| Distancia máx | Tiempo eco aprox | Timeout sugerido | Bloqueo máx |
|---------------|-----------------|------------------|-------------|
| 200 mm (emergencia) | ~1 166 µs | 1 750 µs | ~1.75 ms |
| 500 mm | ~2 916 µs | 3 200 µs | ~3.2 ms |
| 4 000 mm (full) | ~23 326 µs | 30 000 µs (defecto) | ~30 ms |

Fórmula: `timeout_us = distance_mm × 10_000 / 1_715`

El firmware configura `HC_ECHO_TIMEOUT_US = 1_750` µs en `src/config.rs`,
reduciendo el bloqueo máximo de ~30 ms a ~1.75 ms (17×).

Objetos más lejanos que el timeout provocan `Err(SensorError::Timeout)`,
que el loop principal ignora (no hay obstáculo relevante en ese rango).

---

## Integración en el loop principal

El HC-SR04 se lee cada `HC_READ_PERIOD = 5` ciclos (~100 ms, no cada 20 ms)
porque la medición es bloqueante. A 0,5 m/s el rover avanza ~5 cm entre
lecturas — margen aceptable para el umbral de emergencia de 200 mm.

```rust
// Fragmento de main.rs
if let Ok(mm) = hcsr04.measure_mm() {
    if mm < HC_EMERGENCY_MM {   // 200 mm
        let resp = msm.process(Command::Fault);
        sync_drive!(rover, msm);
        iface.send_response(format_response(resp, &mut resp_buf));
    }
}
```

---

## Capas de protección de distancia (contexto)

| Sensor | Umbral | Acción | Latencia |
|--------|--------|--------|----------|
| HC-SR04 (D38/D39) | < 200 mm | FAULT inmediato | ~100 ms |
| VL53L0X (D42/D43) | < 150 mm | FAULT inmediato | ~20 ms |
| RPi5 + cámara | < 300 mm | RET proactivo | ~150 ms |

---

## Notas de hardware

- **Ruido eléctrico**: el HC-SR04 es sensible a inestabilidad en 5V. Añadir
  un condensador de desacoplo 100 nF en los terminales de alimentación.
- **Pulso de trigger**: el driver usa 20 µs (en lugar del mínimo de 10 µs)
  para mejorar compatibilidad con variantes de hardware lentas.
- **Pre-pulso bajo**: 10 µs de LOW antes del trigger para limpiar la línea.
- **Ángulo**: el HC-SR04 tiene un haz de ~15°; puede detectar objetos que el
  VL53L0X (haz puntual) no ve.

---

## Diagnóstico

| Síntoma | Causa probable | Solución |
|---------|---------------|----------|
| `Err(Timeout)` continuo | Sensor desconectado o objeto > rango configurado | Verificar cableado D38/D39; aumentar `HC_ECHO_TIMEOUT_US` |
| `Err(OutOfRange)` frecuente | Objeto muy cercano (< 2 cm) o reflejo lateral | Normal en espacios cerrados muy pequeños |
| Lecturas inestables (±50 mm) | Ruido en 5V o superficie reflectora irregular | Condensador desacoplo; promediar N lecturas |
| FAULT espurio al arrancar | Eco residual del ciclo anterior | El driver pone el trigger a LOW 10 µs antes de disparar — verificar timing |
