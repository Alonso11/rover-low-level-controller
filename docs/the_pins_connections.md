## Diagrama de Conexiones Detallado (Arduino Mega)

### 1. Comunicación con Raspberry Pi 5 (GPIO UART)
Conexión por hardware utilizando el **Serial 3** (USART3) del Arduino Mega.
USART1 (D18/D19) queda libre para los encoders de los motores centrales (INT2/INT3).

| Conexión | Pin Arduino | Registro | Pin RPi 5 | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RX3** | **D15** | PJ0 | **GPIO 14 (TX)** | Recibe de la Pi 5 |
| **TX3** | **D14** | PJ1 | **GPIO 15 (RX)** | Envía a la Pi 5 (**¡Divisor 3.3V obligatorio!**) |
| **GND** | **GND** | GND | **GND** | Tierra común necesaria |

> ⚠️ **AVISO DE SEGURIDAD:** El pin TX del Arduino (D14) envía pulsos de **5V**. La Raspberry Pi 5 solo tolera **3.3V**. Es imperativo usar un divisor de tensión (Resistencias de 1kΩ y 2kΩ) en el cable que va del Arduino a la Raspberry Pi.

### 2. Servo (Gimbal / Dirección)
Control de precisión con PWM hardware a 50Hz usando Timer1 (16-bit).

| Cable Servo | Pin Arduino | Registro | Timer/Canal | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **Signal** | **D11** | PB5 | Timer1/OC1A | 50Hz — Prescale8, TOP=39999 |
| **VCC** | **5V** | — | — | Usar fuente externa si >1 servo |
| **GND** | **GND** | GND | — | Tierra común |

> Timer1 (16-bit) se reserva exclusivamente para el servo. No asignar motores a D11, D12 ni D13.

### 3. Motores DC — 6 Ruedas / 3 Drivers L298N
Layout integrado compatible con encoders, servo y todos los sensores.

| Motor | PWM Pin | Puerto | Timer/Canal | IN1 | IN2 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Front Right** | **D9**  | PH6 | Timer2/OC2B | D22 | D23 |
| **Front Left**  | **D10** | PB4 | Timer2/OC2A | D24 | D25 |
| *(separador)*   | —       | —   | —           | **D26** | **D27** |
| **Center Right**| **D5**  | PE3 | Timer3/OC3A | D28 | D29 |
| **Center Left** | **D6**  | PH3 | Timer4/OC4A | D30 | D31 |
| *(separador)*   | —       | —   | —           | **D32** | **D33** |
| **Rear Right**  | **D7**  | PH4 | Timer4/OC4B | D34 | D35 |
| **Rear Left**   | **D8**  | PH5 | Timer4/OC4C | D36 | D37 |

### 4. Motor de Alta Potencia (BTS7960 / IBT-2)
Control PWM dual para motores de alta corriente.

| Pin BTS7960 | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **RPWM** | **D9** | PH6 | Forward PWM | Timer 2 (8-bit) |
| **LPWM** | **D10** | PB4 | Backward PWM | Timer 2 (8-bit) |
| **R_EN** | **D22** | PA0 | Right Enable | Conectar a 5V o pin digital |
| **L_EN** | **D23** | PA1 | Left Enable | Conectar a 5V o pin digital |
| **VCC** | **5V** | - | Lógica Power | Alimentación chip driver |
| **GND** | **GND** | GND | Ground | Tierra común |

### 5. Puente-H Estándar (L298N — 1 Driver, 2 Motores)
Ejemplo de prueba con un solo driver (`examples/test_l298n.rs`).
Control de dos motores de baja/media potencia.

| Terminal L298N | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **ENA** | **D9** | PH6 | PWM Motor A | Quitar jumper del driver |
| **IN1** | **D8** | PH5 | Dir 1 Motor A | Lógica Digital |
| **IN2** | **D7** | PH4 | Dir 2 Motor A | Lógica Digital |
| **ENB** | **D10** | PB4 | PWM Motor B | Quitar jumper del driver |
| **IN3** | **D6** | PH3 | Dir 1 Motor B | Lógica Digital |
| **IN4** | **D5** | PE3 | Dir 2 Motor B | Lógica Digital |

### 6. Encoders de Motores (Efecto Hall)
Utiliza interrupciones externas (Fase A) para conteo sin pérdida de pulsos.
D18/D19 disponibles gracias a que RPi5 usa USART3 (D14/D15), no USART1.

| Motor         | Fase A (INT) | Pin  | Registro      | Bit stall_mask |
| :---          | :---         | :--- | :---          | :---           |
| **Front Right**  | INT0      | D21  | PD0 (INT0)    | bit 0          |
| **Front Left**   | INT1      | D20  | PD1 (INT1)    | bit 1          |
| **Center Right** | INT2      | D19  | PD2 (INT2)    | bit 2          |
| **Center Left**  | INT3      | D18  | PD3 (INT3)    | bit 3          |
| **Rear Right**   | INT4      | D2   | PE4 (INT4)    | bit 4          |
| **Rear Left**    | INT5      | D3   | PE5 (INT5)    | bit 5          |

> Fase B no se usa en la implementación actual (solo detección de stall, no dirección).

### 7. Sensores de Proximidad (Ultrasonido y LiDAR)
Detección de obstáculos y navegación autónoma.

| Sensor | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **HC-SR04 (Trig)** | **D38** | PD7 | Trigger Out | Pulso de 20µs |
| **HC-SR04 (Echo)** | **D39** | PG2 | Echo In | Medición de tiempo |
| **TF-Luna (RX)** | **D16** | PH1 | TX2 (Out) | Baud: 115200 (¡Divisor 3.3V recomendado!) |
| **TF-Luna (TX)** | **D17** | PH0 | RX2 (In) | Lectura de paquetes (9-byte frame) |
| **VCC** | **5V** | - | Power | Alimentación 5V DC |
| **GND** | **GND** | GND | Ground | Tierra común |

### 8. Sensores Analógicos (Corriente y Temperatura)

| Sensor | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **ACS712-30A (OUT)** | **A0** | PF0 | ADC canal 0 | Corriente ±30A, 66 mV/A, V_zero=2.5V |
| **LM335 (OUT)**      | **A1** | PF1 | ADC canal 1 | Temperatura, 10 mV/K, R_bias 2kΩ a 5V |
| **VCC** | **5V** | - | Power | Ambos sensores alimentados a 5V |
| **GND** | **GND** | GND | Ground | Tierra común |
