# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega). Este proyecto utiliza una arquitectura de librería modular y principios SOLID.

## 🔌 Diagrama de Conexiones Detallado (Arduino Mega)

### 1. Comunicación con Raspberry Pi 5 (GPIO UART)
Conexión por hardware utilizando el **Serial 1** (USART1) del Arduino Mega.

| Conexión | Pin Arduino | Registro | Pin RPi 5 | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RX1** | **D19** | PD2 | **GPIO 14 (TX)** | Recibe de la Pi 5 |
| **TX1** | **D18** | PD3 | **GPIO 15 (RX)** | Envía a la Pi 5 (**¡Divisor 3.3V obligatorio!**) |
| **GND** | **GND** | GND | **GND** | Tierra común necesaria |

> ⚠️ **AVISO DE SEGURIDAD:** El pin TX del Arduino (D18) envía pulsos de **5V**. La Raspberry Pi 5 solo tolera **3.3V**. Es imperativo usar un divisor de tensión (Resistencias de 1kΩ y 2kΩ) en el cable que va del Arduino a la Raspberry Pi.

### 2. Motor de Alta Potencia (BTS7960 / IBT-2)
| Pin BTS7960 | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RPWM** | **D9** | PH6 | Forward PWM | Control de velocidad horaria |
| **LPWM** | **D10** | PB4 | Backward PWM | Control de velocidad antihoraria |
| **R_EN / L_EN** | **5V** | - | Enable | Siempre activos (o pines D22/D23) |

### 3. Encoders de Motores
| Motor | Fase A (Interrupt) | Pin Arduino | Registro | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **Derecho** | INT4 | **D2** | PE4 | Conteo de pulsos alta prioridad |
| **Izquierdo** | INT5 | **D3** | PE5 | Conteo de pulsos alta prioridad |

## 🛠️ Comandos de Validación

### Validar Protocolo por USB (Simulación de RPi5):
Este comando permite probar los comandos `F`, `B` y `S` desde tu computadora usando el cable USB antes de conectar la Raspberry Pi.
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example validate_protocol --target avr-none -Z build-std=core
```

### Ejecutar Control Real por GPIO UART:
Usa este comando cuando ya tengas conectada la Raspberry Pi 5 a los pines D18/D19.
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example control_motor_rpi --target avr-none -Z build-std=core
```

## 📂 Estructura del Proyecto
*   `src/lib.rs`: Librería central de controladores.
*   `src/command_interface/`: Gestión de buffer y protocolo serial (USART).
*   `examples/`: Pruebas de hardware validadas.
