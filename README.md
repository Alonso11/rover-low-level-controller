# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega). Este proyecto utiliza una arquitectura modular y principios SOLID para el control de actuadores y sensores.

## 🚀 Arquitectura de Control de Motores

El sistema utiliza una interfaz común (`Motor` Trait) que permite intercambiar drivers de hardware sin modificar la lógica principal.

### Drivers Soportados:

1.  **L298N:** Driver estándar para motores DC de baja/media potencia.
    *   **Control:** 1 pin PWM (Velocidad) + 2 pines digitales (Dirección).
2.  **BTS7960 (IBT-2):** Driver de alta potencia para motores grandes.
    *   **Control:** 2 pines PWM (RPWM para avance, LPWM para retroceso).

## 🔌 Conexiones de Hardware (Arduino Mega)

### BTS7960 (Driver de Alta Potencia)
| Pin BTS7960 | Pin Arduino | Función |
| :--- | :--- | :--- |
| RPWM | D9 | Forward PWM |
| LPWM | D10 | Backward PWM |
| R_EN / L_EN | 5V | Enable (Siempre activo) |
| VCC / GND | 5V / GND | Lógica y Tierra |

### L298N (Ejemplo Motor A)
| Pin L298N | Pin Arduino | Función |
| :--- | :--- | :--- |
| ENA | D9 | Speed PWM |
| IN1 | D8 | Direction 1 |
| IN2 | D7 | Direction 2 |

## 🛠️ Desarrollo y Compilación

Este proyecto requiere **Rust Nightly** y las herramientas de AVR instaladas.

### Comandos Principales:

**Subir el test actual (BTS7960):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --target avr-none -Z build-std=core
```

**Ejecutar el ejemplo del L298N:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_l298n --target avr-none -Z build-std=core
```

## 📂 Estructura del Proyecto
*   `src/motor_control/`: Contiene la lógica de los drivers.
    *   `mod.rs`: Definición del `trait Motor`.
    *   `l298n.rs`: Implementación para L298N.
    *   `bts7960.rs`: Implementación para BTS7960.
*   `examples/`: Programas de prueba completos para cada componente.
*   `tests/`: Pruebas unitarias de lógica (ejecutables en PC).
