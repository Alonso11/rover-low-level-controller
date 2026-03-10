# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega). Este proyecto utiliza una arquitectura de librería modular y principios SOLID.

## 🔌 Diagrama de Conexiones Completo (Arduino Mega)

### 1. Comunicación Raspberry Pi 5 (UART GPIO)
Conexión directa entre los pines GPIO de la RPi5 y el Serial 1 (USART1) del Arduino Mega. Esto libera el puerto USB del Arduino para otros usos.

| Conexión | Pin Arduino Mega | Pin RPi 5 | Función |
| :--- | :--- | :--- | :--- |
| **RX1** | **D19** (PD2) | **GPIO 14** (TX) | Recibe de la Pi 5 |
| **TX1** | **D18** (PD3) | **GPIO 15** (RX) | Envía a la Pi 5 |
| **GND** | **GND** | **GND** | **¡Obligatorio unir tierras!** |

> ⚠️ **IMPORTANTE:** La RPi5 usa 3.3V. El Arduino Mega usa 5V. **Debes usar un divisor de tensión** en el cable que va del TX del Arduino (D18) al RX de la RPi5 para no dañar la Raspberry Pi.

### 2. Encoders de Motores (Fase A y B)
Utiliza interrupciones externas para un conteo preciso de velocidad y dirección.

| Motor | Pin Fase A (Interrupt) | Pin Fase B (Digital) | Función |
| :--- | :--- | :--- | :--- |
| **Derecho** | **D2** (PE4 - INT4) | **D24** (PA2) | Conteo Derecho |
| **Izquierdo** | **D3** (PE5 - INT5) | **D25** (PA3) | Conteo Izquierdo |

### 3. Actuadores (Motores y Servos)
| Componente | Pines Arduino | Función |
| :--- | :--- | :--- |
| **BTS7960** | D9, D10 (PWM), D22, D23 (EN) | Tracción Alta Potencia |
| **L298N** | D9, D8, D7 (Mot A), D10, D6, D5 (Mot B) | Alternativa Baja Potencia |
| **Servo** | **D11** | Dirección (Software PWM 50Hz) |

## 🛠️ Comandos de Ejecución

**Probar Comunicación con Raspberry Pi 5:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_rpi_communication --target avr-none -Z build-std=core
```

## 📂 Estructura del Proyecto
*   `src/lib.rs`: Punto de entrada de la librería compartida.
*   `src/motor_control/`: Drivers para L298N, BTS7960 y Servos.
*   `src/command_interface/`: Gestión de la comunicación serial.
*   `examples/`: Pruebas funcionales (Echo RPi5, Motores, Servos).
