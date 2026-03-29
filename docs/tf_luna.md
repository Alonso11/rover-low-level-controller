# Sensor LiDAR TF-Luna

> **OBSOLETO — componente no adquirido.**
> El TF-Luna fue sustituido por el **GY-VL53L0XV2** (ST VL53L0X) al no estar
> disponible en inventario. El driver `src/sensors/tf_luna.rs` se mantiene
> para referencia y posible uso futuro si se adquiere el componente.
> Ver `docs/decision-log.md` §Semana 4 — Cambio TF-Luna → VL53L0X.
> La documentación activa del sensor de distancia táctica está en
> `docs/the_pins_connections.md` §7 (VL53L0X, D42/D43 soft I2C).

El **TF-Luna** es un sensor LiDAR de un solo punto, basado en el principio ToF (Time of Flight). Utiliza una fuente de luz infrarroja de 850nm para medir distancias con alta precisión y frecuencia.

## Especificaciones Técnicas
| Parámetro | Valor |
| :--- | :--- |
| Rango de Operación | 0.2 m a 8 m |
| Precisión | ±6 cm (en el rango de 0.2m a 3m) |
| Resolución | 1 cm |
| Frecuencia de actualización | 1 Hz a 250 Hz (100 Hz por defecto) |
| Voltaje de Operación | 5V ± 0.1V |
| Comunicación | UART (3.3V LVTTL) / I2C |
| Consumo de energía | ≤ 0.35 W |

## Protocolo de Comunicación (Serial)
El sensor envía continuamente paquetes de **9 bytes** a una tasa de 115200 baudios.

### Formato del Paquete (Frame)
| Byte | Nombre | Descripción |
| :--- | :--- | :--- |
| 0 | Header | Siempre `0x59` |
| 1 | Header | Siempre `0x59` |
| 2 | Dist_L | Distancia (Byte Bajo) |
| 3 | Dist_H | Distancia (Byte Alto) |
| 4 | Amp_L | Intensidad de señal (Byte Bajo) |
| 5 | Amp_H | Intensidad de señal (Byte Alto) |
| 6 | Temp_L | Temperatura (Byte Bajo) |
| 7 | Temp_H | Temperatura (Byte Alto) |
| 8 | Checksum | Suma de los primeros 8 bytes (Byte Bajo) |

### Algoritmo de Checksum
El checksum es la suma acumulativa de los primeros 8 bytes. En la implementación, validamos que los 8 bits menos significativos coincidan con el byte 8.
`Checksum = (Byte0 + Byte1 + ... + Byte7) & 0xFF`

## Implementación en el Proyecto
El driver se encuentra en `src/sensors/tf_luna.rs` e implementa el trait `ProximitySensor`.

### Conexión en Arduino Mega 2560
Se utiliza el puerto **Serial 2** del Mega para evitar interferencias con la comunicación principal (Serial 0/1).

| Pin Sensor | Pin Arduino | Registro | Función |
| :--- | :--- | :--- | :--- |
| VCC | 5V | - | Alimentación |
| TX | **D17** | PH0 | Recibe en Arduino (RX2) |
| RX | **D16** | PH1 | Envía desde Arduino (TX2) |
| GND | GND | - | Tierra |

> **Nota:** Aunque el sensor funciona a 5V, sus niveles lógicos UART son de 3.3V. El Arduino Mega suele detectar 3.3V como ALTO, pero para mayor seguridad en el envío (TX del Mega hacia RX del sensor), se recomienda un divisor de tensión si se van a enviar comandos de configuración.

### Ejemplo de Código
```rust
let serial2 = arduino_hal::Usart::new(dp.USART2, pins.d17.into_floating_input(), pins.d16.into_output(), 115200.into());
let mut tf_luna = TFLuna::new(serial2);
if let Some(dist_mm) = tf_luna.get_distance_mm() {
    // Uso de la distancia en mm
}
```

## Notas de Implementación
- El driver busca activamente la cabecera `0x59 0x59` para sincronizarse con el flujo de datos.
- Se implementan timeouts en la lectura de cada byte para evitar que el programa se cuelgue si el sensor se desconecta.
- La distancia obtenida originalmente en cm se multiplica por 10 para cumplir con la interfaz de milímetros del proyecto.
