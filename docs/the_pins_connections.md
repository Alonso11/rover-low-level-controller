## Diagrama de Conexiones Detallado (Arduino Mega)

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

### 6. Sensores de Proximidad (Ultrasonido y LiDAR)
Detección de obstáculos y navegación autónoma.

| Sensor | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **HC-SR04 (Trig)** | **D14** | PJ1 | Trigger Out | Pulso de 10µs |
| **HC-SR04 (Echo)** | **D15** | PJ0 | Echo In | Medición de tiempo |
| **TF-Luna (RX)** | **D16** | PH1 | TX2 (Out) | Baud: 115200 (¡Divisor 3.3V recomendado!) |
| **TF-Luna (TX)** | **D17** | PH0 | RX2 (In) | Lectura de paquetes (9-byte frame) |
| **VCC** | **5V** | - | Power | Alimentación 5V DC |
| **GND** | **GND** | GND | Ground | Tierra común |
