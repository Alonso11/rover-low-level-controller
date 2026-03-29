# Sensor ToF VL53L0X (GY-VL53L0XV2)
<!-- Version: v1.0 -->

El **GY-VL53L0XV2** es un módulo basado en el chip ST VL53L0X — sensor
de distancia ToF (Time-of-Flight) de un solo punto con láser VCSEL 940 nm.
Es el sensor de distancia táctica activo del rover, reemplazando al TF-Luna
que no pudo ser adquirido. Ver `docs/decision-log.md` §Semana 4.

---

## Especificaciones Técnicas

| Parámetro | Valor |
|-----------|-------|
| Tecnología | ToF láser VCSEL 940 nm |
| Rango | 3 cm – 200 cm |
| Precisión | ±3% |
| Frecuencia máxima | 50 Hz |
| Voltaje operación | 2.6–3.5V (módulo GY: VIN hasta 5V con regulador) |
| Interfaz | I2C (dirección 0x29, fija) |
| Consumo | ~19 mA en medición continua |

---

## Conexión al Arduino Mega

Conectado via **I2C por software** (bit-bang) en D42/D43 para evitar conflicto
con el bus TWI hardware (D20/D21 = INT0/INT1 = encoders FR/FL).

| Pin Módulo | Pin Arduino | Registro | Función |
|------------|-------------|----------|---------|
| SDA | D42 | PL7 | I2C Data (soft I2C, open-drain) |
| SCL | D43 | PL6 | I2C Clock (soft I2C, open-drain) |
| VCC / VIN | 3.3V o 5V | — | El módulo GY incluye regulador LDO |
| GND | GND | GND | Tierra común |

> Pull-up externo 4.7 kΩ requerido en SDA y SCL (de la línea a 3.3V).
> La dirección I2C 0x29 es fija en hardware — no configurable por software.
> No colisiona con el INA226 (0x40) en el mismo bus soft I2C D42/D43.

---

## Integración en el Firmware

- Driver: `src/sensors/vl53l0x.rs`
- Bus: `src/sensors/soft_i2c.rs` (D42/D43, bit-bang)
- Integrado en `main.rs` v2.6+

### Secuencia de inicialización

El driver implementa la secuencia de inicialización de Pololu:

1. Reset de referencia SPAD
2. Load tuning settings (30 registros de calibración ST)
3. Configurar secuencia single-range continua
4. Iniciar medición continua

### Uso en el loop principal

La distancia se lee en cada ciclo del loop (~20 ms) junto con el HC-SR04.
Umbral de emergencia: **FAULT si distancia < 150 mm**.

```rust
// Fragmento de main.rs
if let Some(dist) = vl53.read_range_mm() {
    sensor_frame.dist_mm = dist;
    if dist < VL53_FAULT_MM {          // 150 mm
        msm.update_distance(dist);      // → FAULT
    }
} else {
    sensor_frame.dist_mm = 0;           // 0 = sin lectura en TLM
}
```

El valor `dist_mm` se incluye en el campo `DIST` del frame TLM.

---

## Capas de Protección de Distancia (resumen)

| Sensor | Umbral | Acción | Latencia |
|--------|--------|--------|----------|
| VL53L0X (D42/D43) | < 150 mm | FAULT inmediato | ~20 ms |
| HC-SR04 (D38/D39) | < 200 mm | FAULT inmediato | ~100 ms |
| HLC (campo DIST del TLM) | < 300 mm | RET proactivo | ~1 s |

---

## Diagnóstico

| Síntoma | Causa probable | Solución |
|---------|---------------|----------|
| `dist_mm = 0` en TLM siempre | Error de inicialización I2C | Verificar pull-ups 4.7kΩ en D42/D43 |
| Lecturas erráticas | Luz solar intensa (IR 940 nm) | El VL53L0X es sensible a IR ambiental fuerte |
| Sin detección > 150 cm | Límite del sensor | Normal — rango máximo ~200 cm |
| `I2C NACK` en arranque | Dirección incorrecta o sin alimentación | Confirmar VIN conectado; dirección fija 0x29 |
