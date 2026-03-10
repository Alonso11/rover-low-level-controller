# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega). Este proyecto utiliza una arquitectura modular de librería y principios SOLID para el control de actuadores y sensores.

## 🔌 Diagrama de Conexiones Detallado (Arduino Mega)

### 1. Comunicación Raspberry Pi 5 (UART GPIO)
Conexión directa por hardware. **Serial 1** del Arduino Mega.

| Conexión | Pin Arduino | Registro | Pin RPi 5 | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RX1** | **D19** | PD2 | **GPIO 14 (TX)** | Recibe datos de la Pi 5 |
| **TX1** | **D18** | PD3 | **GPIO 15 (RX)** | Envía datos a la Pi 5 (**Divisor 3.3V**) |
| **GND** | **GND** | GND | **GND** | **¡Obligatorio unir tierras!** |

> ⚠️ **PROTECCIÓN:** El TX del Arduino envía 5V. La RPi5 solo soporta 3.3V. Usa un divisor de tensión (Resistencias 1k y 2k Ω).

### 2. Motor de Alta Potencia (BTS7960 / IBT-2)
Requiere control PWM dual y pines de habilitación.

| Pin BTS7960 | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RPWM** | **D9** | PH6 | Forward PWM | Timer 2 (8-bit) |
| **LPWM** | **D10** | PB4 | Backward PWM | Timer 2 (8-bit) |
| **R_EN** | **D22** | PA0 | Right Enable | Conectar a 5V o pin digital |
| **L_EN** | **D23** | PA1 | Left Enable | Conectar a 5V o pin digital |
| **VCC** | **5V** | - | Lógica Power | Alimentación del chip interno |
| **GND** | **GND** | GND | Ground | Tierra común |
| **B+ / B-** | **BATERÍA** | - | Power Input | 7.4V - 24V externos |
| **M+ / M-** | **MOTOR** | - | Motor Output | Conexión al motor DC |

### 3. Puente-H Estándar (L298N)
Control de dos motores independientes.

| Terminal L298N | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **ENA** | **D9** | PH6 | PWM Motor A | Quitar jumper del driver |
| **IN1** | **D8** | PH5 | Dir 1 Motor A | Lógica Digital |
| **IN2** | **D7** | PH4 | Dir 2 Motor A | Lógica Digital |
| **ENB** | **D10** | PB4 | PWM Motor B | Quitar jumper del driver |
| **IN3** | **D6** | PH3 | Dir 1 Motor B | Lógica Digital |
| **IN4** | **D5** | PE3 | Dir 2 Motor B | Lógica Digital |

### 4. Encoders de Motores (Efecto Hall / Ópticos)
Utiliza interrupciones externas (INT) para evitar pérdida de pulsos.

| Motor | Fase A (Pin) | Registro | Fase B (Pin) | Registro | Función |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Derecho** | **D2** | PE4 (INT4) | **D24** | PA2 | Canal A (IRQ) + Canal B |
| **Izquierdo**| **D3** | PE5 (INT5) | **D25** | PA3 | Canal A (IRQ) + Canal B |

### 5. Servomotores (Dirección / Gimbal)
Control por software PWM a 50Hz.

| Cable Servo | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **Naranja** | **D11** | PB5 | Signal | Software PWM (50Hz) |
| **Rojo** | **5V** | - | VCC | Usar fuente externa si >1 servo |
| **Marrón** | **GND** | GND | Ground | Tierra común |

## 🛠️ Comandos de Ejecución

**Comunicación Serial (Echo):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_rpi_communication --target avr-none -Z build-std=core
```

**Test Servomotor (Safe 50Hz):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_servo --target avr-none -Z build-std=core
```

**Test Motores BTS7960:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_bts7960 --target avr-none -Z build-std=core
```

## 📂 Estructura del Proyecto
*   `src/lib.rs`: Punto de entrada de la librería compartida.
*   `src/motor_control/`: Drivers para L298N, BTS7960 y Servos.
*   `src/command_interface/`: Gestión de la comunicación serial (USART1).
*   `examples/`: Programas de prueba funcionales listos para hardware.
