<!-- Version: v1.0 -->
# Rover Low-Level Controller (Rust)

A high-performance, modular firmware for a multi-terrain rover, implemented in **embedded Rust** for the **ATmega2560** (Arduino Mega). This project serves as the hardware abstraction layer (HAL), providing low-level execution for a **Raspberry Pi 5** (running Yocto Linux) through a dedicated GPIO UART communication bridge.

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
Control PWM dual para motores de alta corriente.

| Pin BTS7960 | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RPWM** | **D9** | PH6 | Forward PWM | Timer 2 (8-bit) |
| **LPWM** | **D10** | PB4 | Backward PWM | Timer 2 (8-bit) |
| **R_EN** | **D22** | PA0 | Right Enable | Conectar a 5V o pin digital |
| **L_EN** | **D23** | PA1 | Left Enable | Conectar a 5V o pin digital |
| **VCC** | **5V** | - | Lógica Power | Alimentación chip driver |
| **GND** | **GND** | GND | Ground | Tierra común |

### 3. Puente-H Estándar (L298N)
Control de dos motores de baja/media potencia.

| Terminal L298N | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **ENA** | **D9** | PH6 | PWM Motor A | Quitar jumper del driver |
| **IN1** | **D8** | PH5 | Dir 1 Motor A | Lógica Digital |
| **IN2** | **D7** | PH4 | Dir 2 Motor A | Lógica Digital |
| **ENB** | **D10** | PB4 | PWM Motor B | Quitar jumper del driver |
| **IN3** | **D6** | PH3 | Dir 1 Motor B | Lógica Digital |
| **IN4** | **D5** | PE3 | Dir 2 Motor B | Lógica Digital |

### 4. Encoders de Motores (Efecto Hall / Ópticos)
Utiliza interrupciones externas para evitar pérdida de pulsos.

| Motor | Fase A (Pin) | Registro | Fase B (Pin) | Registro | Función |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Derecho** | **D2** | PE4 (INT4) | **D24** | PA2 | Canal A (IRQ) + Canal B |
| **Izquierdo**| **D3** | PE5 (INT5) | **D25** | PA3 | Canal A (IRQ) + Canal B |

### 5. Servomotores (Dirección / Gimbal)
Control de precisión por software a 50Hz.

| Cable Servo | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **Naranja** | **D11** | PB5 | Signal | Software PWM (50Hz) |
| **Rojo** | **5V** | - | VCC | Usar fuente externa si >1 servo |
| **Marrón** | **GND** | GND | Ground | Tierra común |

## Compilación Segura (Dry Run)

Para verificar que el código es correcto y compila sin errores antes de flashear el hardware, utiliza el siguiente comando. Este comando recompila la librería estándar (`core`) para asegurar compatibilidad total con el ATmega2560.

**Compilar Todo el Proyecto:**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --target avr-none -Z build-std=core
```

**Compilar Ejemplo de 6 Motores (Nuevo):**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --example control_6_motors_l298n --target avr-none -Z build-std=core
```

## Comandos de Validación (Flasheo)

**Validar Protocolo por USB (PC):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example validate_protocol --target avr-none -Z build-std=core
```

**Ejecutar Control Real por GPIO UART:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example control_motor_rpi --target avr-none -Z build-std=core
```

**Probar Ejemplo L298N:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_l298n --target avr-none -Z build-std=core
```

**Probar Ejemplo Servo (Seguro):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_servo --target avr-none -Z build-std=core
```

## 📂 Estructura del Proyecto
*   `src/lib.rs`: Punto de entrada de la librería compartida.
*   `src/motor_control/`: Drivers para L298N, BTS7960 y Servos.
*   `src/command_interface/`: Gestión de buffer y protocolo serial (USART).
*   `examples/`: Programas de prueba funcionales listos para hardware.
*   `tests/`: Validaciones de lógica ejecutables en PC.
