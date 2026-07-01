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
| **Front Right** | **D9**  | PH6 | Timer2/OC2B | D23 | D25 |
| **Front Left**  | **D10** | PB4 | Timer2/OC2A | D22 | D24 |
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
| **R_EN** | **D40** | PG1 | Right Enable | Pin libre, compatible con config 6-ruedas |
| **L_EN** | **D41** | PG0 | Left Enable | Pin libre, compatible con config 6-ruedas |
| **VCC** | **5V** | - | Lógica Power | Alimentación chip driver |
| **GND** | **GND** | GND | Ground | Tierra común |

> D40/D41 (PG1/PG0) se eligieron porque D22/D23 están reservados para IN1/IN2
> del motor Front Right (§3). Esta asignación es compatible con la configuración
> de 6 ruedas y con el feature `mixed-drivers`.

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

### 7. Sensores de Proximidad (Ultrasonido y ToF LiDAR)
Detección de obstáculos y navegación autónoma.

| Sensor | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **HC-SR04 (Trig)** | **D38** | PD7 | Trigger Out | Pulso de 20µs |
| **HC-SR04 (Echo)** | **D39** | PG2 | Echo In | Medición de tiempo |
| **VL53L0X (SDA)** | **D42** | PL7 | I2C Data | Soft I2C — open-drain, pull-up 4.7kΩ externo |
| **VL53L0X (SCL)** | **D43** | PL6 | I2C Clock | Soft I2C — open-drain, pull-up 4.7kΩ externo |
| **VL53L0X (VCC)** | **3.3V** | — | Power | Módulo GY incluye regulador; tolera 5V en VIN |
| **VL53L0X (GND)** | **GND** | GND | Ground | Tierra común |
| **GND** | **GND** | GND | Ground | Tierra común |

> D42/D43 (PL7/PL6) se usan para I2C software (bit-bang) para evitar conflicto con el
> bus TWI hardware (D20/D21) reservado para los encoders (INT0/INT1).
> La dirección I2C del VL53L0X es 0x29 (fija en hardware, no configurable).
> TF-Luna (USART2, D16/D17): componente reservado; driver `tf_luna.rs` mantenido pero
> no instanciado. Ver `docs/decision-log.md` §Semana 4.

### 7b. Monitor de Potencia (INA226) — Bus compartido D42/D43

Mide tensión y corriente total del pack de baterías. Comparte el bus soft I2C con el VL53L0X sin conflicto de dirección.

**Módulo:** [INA226 – componenteselectronicoscr.com](https://componenteselectronicoscr.com/product/monitor-de-derivacion-de-corriente-y-potencia-con-inter)

| Señal INA226 | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **SDA** | **D42** | PL7 | I2C Data | Compartido con VL53L0X |
| **SCL** | **D43** | PL6 | I2C Clock | Compartido con VL53L0X |
| **VBUS** | — | — | Entrada voltaje bus | Conectar al + de batería (hasta 36 V) |
| **IN+** | — | — | Terminal + shunt | En serie con la carga, lado batería |
| **IN−** | — | — | Terminal − shunt | En serie con la carga, lado motores |
| **A0** | **GND** | — | Selección dirección | A0=GND → dirección 0x40 |
| **A1** | **GND** | — | Selección dirección | A1=GND → dirección 0x40 |
| **VCC** | **3.3V / 5V** | — | Alimentación lógica | 2.7–5.5 V |
| **GND** | **GND** | GND | Tierra común | GND común con Arduino |

> **Shunt externo requerido:** el módulo no incluye shunt. Usar resistencia de **10 mΩ / 5 W**
> (0.01 Ω) para corrientes de hasta ~10 A. Constante `INA226_SHUNT_MOHM = 10` en `main.rs`.
> Con shunt de 100 mΩ reducir `INA226_SHUNT_MOHM` a 100 y verificar potencia máxima: P = I² × R.
> Dirección 0x40 no colisiona con VL53L0X (0x29). ADC 16-bit, ±0.1% de precisión.

### 8. Sensores Analógicos (Corriente y Temperatura)

| Sensor | Pin Arduino | Registro | Función | Notas |
| :--- | :--- | :--- | :--- | :--- |
| **ACS712 FR (OUT)**  | **A0** | PF0 | ADC canal 0  | Corriente motor Front Right  |
| **ACS712 FL (OUT)**  | **A1** | PF1 | ADC canal 1  | Corriente motor Front Left   |
| **ACS712 CR (OUT)**  | **A2** | PF2 | ADC canal 2  | Corriente motor Center Right |
| **ACS712 CL (OUT)**  | **A3** | PF3 | ADC canal 3  | Corriente motor Center Left  |
| **ACS712 RR (OUT)**  | **A4** | PF4 | ADC canal 4  | Corriente motor Rear Right   |
| **ACS712 RL (OUT)**  | **A5** | PF5 | ADC canal 5  | Corriente motor Rear Left    |
| **LM335 (OUT)**      | **A6** | PF6 | ADC canal 6  | Temp. ambiente, 10 mV/K, R_bias 2kΩ a 5V |
| **NTC Banco1-A (AO)**| **A7** | PF7 | ADC canal 7  | Temp. batería 18650 banco 1, sensor A |
| **NTC Banco1-B (AO)**| **A8** | PK0 | ADC canal 8  | Temp. batería 18650 banco 1, sensor B |
| **NTC Banco2-A (AO)**| **A9** | PK1 | ADC canal 9  | Temp. batería 18650 banco 2, sensor A |
| **NTC Banco2-B (AO)**| **A10**| PK2 | ADC canal 10 | Temp. batería 18650 banco 2, sensor B |
| **NTC Banco3-A (AO)**| **A11**| PK3 | ADC canal 11 | Temp. batería 18650 banco 3, sensor A |
| **NTC Banco3-B (AO)**| **A12**| PK4 | ADC canal 12 | Temp. batería 18650 banco 3, sensor B |
| **VCC** | **5V** | - | Power | Todos los sensores alimentados a 5V |
| **GND** | **GND** | GND | Ground | Tierra común |

> Módulo NTC: AD36958 — LM393 + NTC 10 kΩ (B=3950). Pull-up de 10 kΩ integrado en la placa.
> Thresholds en firmware: Warn >45 °C · Limit >55 °C · Fault >65 °C (thermal runaway 18650 ~80–90 °C).
> La salida DO del LM393 queda sin conectar en esta revisión (umbral ajustable por potenciómetro,
> no controlable por software sin recalibrar físicamente).
