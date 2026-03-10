# Rover Low-Level Controller (Rust)

Controlador de bajo nivel para un rover, desarrollado en Rust para el microcontrolador ATmega2560 (Arduino Mega).

## 🔌 Diagrama de Conexiones Completo (Arduino Mega)

### 1. Motor de Alta Potencia (BTS7960) - Tracción Principal
Este driver requiere pines PWM y pines de habilitación.

| Pin BTS7960 | Pin Arduino Mega | Función en Software | Notas |
| :--- | :--- | :--- | :--- |
| **RPWM** | **D9** (PH6) | Forward PWM (Timer 2) | Control de velocidad adelante |
| **LPWM** | **D10** (PB4) | Backward PWM (Timer 2) | Control de velocidad atrás |
| **R_EN** | **D22** (PA0) | Right Enable | Habilitación del giro horario |
| **L_EN** | **D23** (PA1) | Left Enable | Habilitación del giro antihorario |
| **VCC** | **5V** | Lógica Power | Alimentación del chip del driver |
| **GND** | **GND** | Ground | Tierra común con el Arduino |
| **B+ / B-** | **BATERÍA** | Power Input | 7.4V - 24V externos |
| **M+ / M-** | **MOTOR DC** | Motor Output | Salida al motor |

### 2. Puente-H Estándar (L298N) - Motores Secundarios
Configuración típica para dos motores (A y B).

| Terminal L298N | Pin Arduino Mega | Función | Notas |
| :--- | :--- | :--- | :--- |
| **ENA** | **D9** (PH6) | PWM Motor A | Quitar jumper del L298N |
| **IN1** | **D8** (PH5) | Dir 1 Motor A | Lógica de dirección |
| **IN2** | **D7** (PH4) | Dir 2 Motor A | Lógica de dirección |
| **ENB** | **D10** (PB4) | PWM Motor B | Quitar jumper del L298N |
| **IN3** | **D6** (PH3) | Dir 1 Motor B | Lógica de dirección |
| **IN4** | **D5** (PE3) | Dir 2 Motor B | Lógica de dirección |

### 3. Servomotores (Dirección / Cámara)
Utiliza control por software (50Hz) para evitar ruidos y calentamiento.

| Cable Servo | Pin Arduino Mega | Función | Notas |
| :--- | :--- | :--- | :--- |
| **Naranja (Signal)**| **D11** (PB5) | PWM 50Hz | Control de posición |
| **Rojo (VCC)** | **5V** | Power | Usar fuente externa si >1 servo |
| **Marrón (GND)** | **GND** | Ground | Tierra común |

### 4. Sensores de Distancia
| Sensor | Pin Arduino Mega | Función | Notas |
| :--- | :--- | :--- | :--- |
| **HC-SR04 Trig** | **D12** (PB6) | Trigger | Pulso de disparo |
| **HC-SR04 Echo** | **D13** (PB7) | Echo | Medición de tiempo |
| **TF-Luna TX** | **RX0 (D0)** | UART RX | Recibe de la RPi5 / Sensor |
| **TF-Luna RX** | **TX0 (D1)** | UART TX | Envía a la RPi5 / Sensor |

### 5. Comunicación Raspberry Pi 5 (Yocto)
| Conexión | Tipo | Notas |
| :--- | :--- | :--- |
| **Puerto USB-B** | **USB a RPi5** | Comunicación Serial @ 115200 baudios |

---

## 🛠️ Comandos de Ejecución (Power User)

**Probar Comunicación Serial (Echo):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_serial_echo --target avr-none -Z build-std=core
```

**Probar Movimiento Servo:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_servo --target avr-none -Z build-std=core
```

**Probar Motores BTS7960:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_bts7960 --target avr-none -Z build-std=core
```
