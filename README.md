# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega). Este proyecto utiliza una arquitectura modular de librería y principios SOLID para el control de actuadores y sensores.

## 🚀 Arquitectura de la Librería

El núcleo del proyecto reside en `src/lib.rs`, permitiendo que tanto el programa principal como los ejemplos utilicen la misma lógica de drivers sin duplicación de código. Se utilizan interfaces comunes (`Motor` y `Servo` Traits) para garantizar la escalabilidad.

### Drivers de Motores DC:

1.  **L298N:** Driver estándar para motores DC de baja/media potencia.
    *   **Control:** 1 pin PWM (Velocidad) + 2 pines digitales (Dirección).
2.  **BTS7960 (IBT-2):** Driver de alta potencia para motores grandes.
    *   **Control:** 2 pines PWM (RPWM para avance, LPWM para retroceso).

### Driver de Servomotores:

1.  **StandardServo:** Control de posición angular para servos de 0-180°.
    *   **Implementación:** Utiliza **Software PWM (Bit-banging)** con precisión de microsegundos a **50Hz**. Esto previene el sobrecalentamiento en servos estándar al evitar las altas frecuencias de los timers PWM por defecto.

## 🔌 Conexiones de Hardware (Arduino Mega)

### BTS7960
| Pin BTS7960 | Pin Arduino | Función |
| :--- | :--- | :--- |
| RPWM | D9 | Forward PWM |
| LPWM | D10 | Backward PWM |
| R_EN / L_EN | 5V | Enable |

### L298N
| Pin L298N | Pin Arduino | Función |
| :--- | :--- | :--- |
| ENA | D9 | Speed PWM |
| IN1 / IN2 | D8 / D7 | Direction |

### Servomotor
| Cable Servo | Pin Arduino | Función |
| :--- | :--- | :--- |
| Naranja | **D11** | Señal (50Hz) |
| Rojo / Marrón| 5V / GND | Alimentación |

## 🛠️ Comandos de Ejecución

Este proyecto utiliza **Rust Nightly** para AVR.

### Subir Ejemplos al Hardware:

Sustituye `/dev/ttyUSB0` por tu puerto local.

**Probar Servo (50Hz Seguro):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_servo --target avr-none -Z build-std=core
```

**Probar BTS7960 (Alta Potencia):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_bts7960 --target avr-none -Z build-std=core
```

**Probar L298N:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_l298n --target avr-none -Z build-std=core
```

## 📂 Estructura
*   `src/lib.rs`: Punto de entrada de la librería de drivers.
*   `src/motor_control/`: Implementaciones de drivers (SOLID).
*   `examples/`: Programas de prueba funcionales para hardware real.
*   `tests/`: Validaciones de lógica ejecutables en PC.
