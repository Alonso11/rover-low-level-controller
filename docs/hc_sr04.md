# Sensor Ultrasónico HC-SR04

El **HC-SR04** es un sensor de proximidad ultrasónico que utiliza el tiempo de vuelo (ToF) de ondas sonoras para determinar la distancia a un objeto.

## Especificaciones Técnicas
| Parámetro | Valor |
| :--- | :--- |
| Voltaje de Operación | 5V DC |
| Corriente de Operación | 15 mA |
| Frecuencia ultrasónica | 40 kHz |
| Rango Máximo | 400 cm (4 m) |
| Rango Mínimo | 2 cm |
| Ángulo de medición | < 15° |
| Resolución | 0.3 cm |

## Principio de Funcionamiento
1. Se envía un pulso de nivel alto de **10µs** al pin `Trigger`.
2. El módulo envía automáticamente ocho ráfagas de 40 kHz y detecta si hay un pulso de retorno.
3. Si hay una señal de retorno, el pin `Echo` se pone en ALTO. 
4. La duración de este pulso ALTO es el tiempo que tarda el sonido en ir y volver del objeto.

### Fórmula de Cálculo
La distancia se calcula multiplicando el tiempo por la velocidad del sonido (343 m/s o 0.0343 cm/µs) y dividiendo por 2 (ida y vuelta).

$$Distancia (mm) = \frac{Tiempo (µs) \times 0.343}{2}$$

## Implementación en el Proyecto
En este proyecto, el driver se encuentra en `src/sensors/hc_sr04.rs` e implementa el trait `ProximitySensor`.

### Conexión en Arduino Mega 2560
| Pin Sensor | Pin Arduino | Función |
| :--- | :--- | :--- |
| VCC | 5V | Alimentación |
| Trig | **D14** | Disparo del pulso |
| Echo | **D15** | Recepción del eco |
| GND | GND | Tierra |

### Ejemplo de Código
```rust
let mut hc_sr04 = HCSR04::new(pins.d14.into_output(), pins.d15.into_floating_input().forget_imode());
if let Some(dist) = hc_sr04.get_distance_mm() {
    // Uso de la distancia en mm
}
```

## Notas de Implementación
- El driver utiliza `arduino_hal::delay_us` para medir la duración del pulso.
- Se ha implementado un **Timeout** de 30ms (equivalente a una distancia fuera de rango de ~5 metros) para evitar bloqueos del procesador si no se recibe eco.
- Se recomienda un intervalo de al menos 60ms entre mediciones para evitar interferencias.
