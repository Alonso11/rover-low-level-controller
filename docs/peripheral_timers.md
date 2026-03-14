<!-- Version: v1.0 -->
# Temporizadores (Timers) del ATmega2560

Este documento detalla los temporizadores hardware disponibles en el microcontrolador ATmega2560 para la generación de señales PWM, con focus en el control de motores.

## 1. Visión General

El ATmega2560 dispone de **6 temporizadores** (Timer0 a Timer5), cada uno con diferente número de canales PWM:

| Timer | Canales PWM | Frecuencia Base | Uso Recomendado |
|-------|-------------|-----------------|-----------------|
| Timer0 | 2 | 1kHz (Prescale 64) | Sistema (delay_ms) |
| Timer1 | 3 | ~1kHz | Motor 2 (Central) |
| Timer2 | 2 | ~1kHz | Motor 1 (Frontal) |
| Timer3 | 3 | ~1kHz | Reservado / Encoders |
| Timer4 | 3 | ~1kHz | Reservado / Extensión |
| Timer5 | 3 | ~1kHz | Motor 3 (Trasero) |

**Total: 16 canales PWM disponibles**

## 2. Distribución de Pines por Timer (Nueva Arquitectura para Encoders)

### Timer2 (8-bit, 2 canales) - Motores Frontales
| Canal | Pin Arduino | Puerto | Registro OCR |
|-------|-------------|--------|--------------|
| OC2A | D10 | PB4 | OCR2A |
| OC2B | D9 | PH6 | OCR2B |

### Timer1 (16-bit, 3 canales) - Motores Centrales
| Canal | Pin Arduino | Puerto | Registro OCR |
|-------|-------------|--------|--------------|
| OC1A | D11 | PB5 | OCR1A |
| OC1B | D12 | PB6 | OCR1B |
| OC1C | D13 | PB7 | OCR1C |

### Timer5 (16-bit, 3 canales) - Motores Traseros
| Canal | Pin Arduino | Puerto | Registro OCR |
|-------|-------------|--------|--------------|
| OC5A | D46 | PL3 | OCR5A |
| OC5B | D45 | PL4 | OCR5B |
| OC5C | D44 | PL5 | OCR5C |

## 3. Asignación de Encoders (Interrupciones Externas)

Para los encoders se utilizan los pines con capacidad de **Interrupción Externa (INT)** para asegurar que no se pierda ningún pulso a alta velocidad:

| Encoder | Pin Arduino | Interrupción |
|---------|-------------|--------------|
| Frontal Derecho | D21 | INT0 |
| Frontal Izquierdo | D20 | INT1 |
| Central Derecho | D19 | INT2 |
| Central Izquierdo | D18 | INT3 |
| Trasero Derecho | D2 | INT4 |
| Trasero Izquierdo | D3 | INT5 |

## 4. Configuración de Frecuencia

La frecuencia del PWM se calcula con:

```
f_PWM = f_CPU / (Prescaler * 256)
```

Con `Prescale64` y `f_CPU = 16MHz`:
```
f_PWM = 16,000,000 / (64 * 256) ≈ 976 Hz ≈ 1 kHz
```

Valores de Prescaler disponibles en `arduino-hal`:
- `Prescale1`, `Prescale8`, `Prescale64`, `Prescale256`, `Prescale1024`

## 4. Reglas de Uso para Motores

### Principio: Un Timer por puente-H

Cada driver L298N controlling 2 motors debe usar **timers diferentes** para evitar conflictos.

### Casos de Uso

#### 2 Motores (1 puente-H)
- **Opción A**: Timer2 (D10, D9) → suficiente
- **Opción B**: Timer2 + Timer3 → mayor redundancia

#### 4 Motores (2 puentes-H)
- Timer2 (puente 1)
- Timer3 (puente 2)

#### 6 Motores (3 puentes-H)
- Timer2 (puente frontal)
- Timer3 (puente central)
- Timer4 (puente trasero)

## 5. Conflictos Comunes y Soluciones

### Conflicto 1: Pines de dirección vs PWM
**Problema**: Usar pines PWM como pines de dirección (IN1, IN2).
**Solución**: Usar pines no-PWM para dirección (ej: D22, D23, D24, D25).

### Conflicto 2: Múltiples canales del mismo timer
**Problema**:Timer2 solo tiene 2 canales, insuficiente para más de 2 motores.
**Solución**: Distribuir en Timer2, Timer3, Timer4.

### Conflicto 3: Timer0 reservado
**Problema**: Timer0 usado internamente por `arduino-hal` para `delay_ms()`.
**Solución**: NO usar Timer0 para PWM de motores.

## 6. Ejemplo de Inicialización

```rust
use arduino_hal::simple_pwm::{Timer2Pwm, Timer3Pwm, Prescaler};

// Timer para motor derecho (puente 1)
let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);

// Timer para motor izquierdo (puente 2) - usar timer diferente
let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);

// Motor derecho: PWM en D10 (OC2A), dirección en D22, D23
let right_pwm = pins.d10.into_output().into_pwm(&mut timer2);

// Motor izquierdo: PWM en D2 (OC3B), dirección en D28, D29
let left_pwm = pins.d2.into_output().into_pwm(&mut timer3);
```

## 7. Tabla de Referencia Rápida (Configuración Encoders)

| Motores | Timer 2 (Front) | Timer 1 (Cent) | Timer 5 (Rear) | Pines PWM | Pines Encoder |
|---------|-----------------|----------------|----------------|-----------|---------------|
| 2 | D10, D9 | - | - | D10, D9 | D21, D20 |
| 4 | D10, D9 | D11, D12 | - | D10, D9, D11, D12 | D21-D18 |
| 6 | D10, D9 | D11, D12 | D46, D45 | D10, D9, D11, D12, D46, D45 | D21, D20, D19, D18, D2, D3 |

## 8. Notas de Robustez

1. **Separación EMI**: Usar timers diferentes reduce interferencia electromagnética
2. **Redundancia**: Si un timer falla, los otros siguen funcionando
3. **Frecuencia única**: Todos los timers con mismo Prescaler = misma frecuencia (~1kHz)
4. **No usar Timer0**: Reservado para el sistema