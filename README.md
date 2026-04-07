<!-- Version: v2.1 -->
# Rover Low-Level Controller (LLC) — Rust/AVR

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Systems Engineering](https://img.shields.io/badge/Focus-Systems%20Engineering-blue.svg)](#)

## Overview

Firmware modular para un rover de 6 ruedas en **Rust embebido** sobre el
**ATmega2560** (Arduino Mega 2560). Actúa como controlador de bajo nivel (LLC):
recibe comandos del protocolo MSM desde la **Raspberry Pi 5** (Yocto Linux)
por UART y gestiona motores, encoders, sensores de proximidad y corriente.

---

## Arquitectura del sistema

```
┌──────────────────────────────┐        ┌────────────────────────────────────┐
│  Raspberry Pi 5 (HLC)        │        │  Arduino Mega 2560 (LLC)           │
│  olympus_controller.py       │        │  rover-low-level-controller        │
│  rover_bridge.so (Rust/PyO3) │        │                                    │
│                              │─ USB ──│─ USART0 (desarrollo)               │
│                              │─ GPIO ─│─ USART3 D14/D15 (producción)       │
└──────────────────────────────┘        │                                    │
                                        │  6 motores L298N (PWM Timer2/3/4)  │
                                        │  6 encoders Hall (INT0–INT5) + MPU-6050        │
│  MPU-6050 (Accel/Gyro) soft I2C     │
                                        │  HC-SR04 D38/D39 (emergencia)      │
                                        │  VL53L0X D42/D43 soft I2C (táctica)│
                                        │  6× ACS712-30A A0–A5 (corriente)   │
                                        │  LM335 A6 (temperatura ambiente)   │
                                        │  6× NTC A7–A12 (temperatura celdas)│
                                        └────────────────────────────────────┘
```

---

## Estructura del proyecto

```
src/
├── lib.rs                     # Punto de entrada de la librería
├── main.rs                    # Loop principal: watchdog → sensores → MSM → motores → TLM
├── config.rs                  # Constantes de compilación (tiempos, umbrales, periodos ADC)
├── motor_control/
│   ├── mod.rs                 # Traits Motor / Servo + SixWheelRover (lógica pura)
│   ├── l298n.rs               # Driver L298N (AVR)
│   ├── bts7960.rs             # Driver BTS7960 alta potencia (AVR)
│   ├── servo.rs               # PWM software para servo (AVR)
│   └── erased.rs              # ErasedMotor — type erasure para arrays de motores
├── sensors/
│   ├── mod.rs                 # Trait ProximitySensor + enum SensorError
│   ├── encoder.rs             # HallEncoder (interrupt-safe con AtomicI32) (AVR)
│   ├── hc_sr04.rs             # HC-SR04 ultrasónico D38/D39, Result API (AVR)
│   ├── vl53l0x.rs             # VL53L0X ToF I2C D42/D43 soft I2C, Result API (AVR)
│   ├── soft_i2c.rs            # I2C bit-bang con clock-stretch timeout (AVR)
│   ├── acs712.rs              # ACS712-30A corriente de motor (puro Rust)
│   ├── lm335.rs               # LM335 temperatura ambiente (puro Rust)
│   ├── ntc_thermistor.rs      # NTC AD36958 temperatura celdas (puro Rust)
│   └── tf_luna.rs             # TF-Luna LiDAR reservado, Result API (AVR)
├── controller/
│   └── mod.rs                 # RoverController — detección de stall por canal
├── state_machine/
│   └── mod.rs                 # MSM — 5 estados, watchdog, format_response
└── command_interface/
    └── mod.rs                 # Buffer de protocolo MSM (UART)

examples/                      # Programas para flashear al Arduino (AVR)
tests/
├── state_machine_test / ekf_test.rs      # Tests lógica pura MSM (x86, sin hardware)
├── motor_logic_test.rs        # Tests lógica de motores (x86, sin hardware)
├── sensors_test.rs            # Tests drivers ACS712/LM335 (x86, sin hardware)
└── hardware/
    ├── test_msm_protocol.py   # Verificación protocolo MSM desde PC via USB
    └── test_motors_debug.py   # Debug individual de motores desde PC via USB
docs/                          # Diagramas de hardware, notas de diseño
```

---

## Build

Requiere toolchain Rust nightly con `rust-src` y el toolchain AVR GCC:

```bash
# Instalar herramientas AVR (Debian/Ubuntu)
sudo apt-get install gcc-avr avr-libc

# Añadir nightly + rust-src
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

**Compilar la librería completa:**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build -Zjson-target-spec -Zbuild-std=core
```

**Compilar y flashear el firmware principal:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --release -Zjson-target-spec -Zbuild-std=core
```

---

## Tests

### Tests de lógica (x86, sin hardware)

Validan la MSM, drivers analógicos y lógica de motores en la máquina de desarrollo:

```bash
./test_native.sh
```

| Suite | Tests | Cobertura |
|-------|-------|-----------|
| `state_machine_test / ekf_test` | 46 | Todas las transiciones MSM, watchdog, format_response, parser TLM (incluyendo campos odometría) |
| `sensors_test` | 64 | ACS712 conversión mA, LM335 conversión °C, NTC interpolación LUT, umbrales Warn/Limit/Fault |
| `motor_logic_test` | 28 | Speed mapping, signos de dirección L298N/BTS7960, SixWheelRover, ErasedMotor |

### Tests de hardware (PC + Arduino via USB)

Requieren el Arduino conectado y el firmware flasheado:

| Script | Firmware requerido | Descripción |
|--------|-------------------|-------------|
| `tests/hardware/test_msm_protocol.py` | Firmware principal (v2.10+) | 13 tests automáticos del protocolo MSM + validación formato TLM |
| `tests/hardware/test_motors_debug.py` | `examples/debug_motors_l298n` | Control interactivo F/B/S para verificar cada motor individualmente |

```bash
# Instalar dependencia Python
pip install pyserial

# Verificar protocolo MSM completo (firmware principal)
python3 tests/hardware/test_msm_protocol.py [/dev/ttyUSB0]

# Debug de motores (flashear debug_motors_l298n primero)
python3 tests/hardware/test_motors_debug.py [/dev/ttyUSB0]
```

Ver [`docs/testing.md`](docs/testing.md) para la explicación completa de flags,
troubleshooting y el flujo de trabajo recomendado antes de cada commit.

---

## Examples

Programas completos para flashear al Arduino. Cada uno es un binario AVR independiente:

```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --example <nombre> --release -Zjson-target-spec -Zbuild-std=core
```

| Example | Descripción |
|---------|-------------|
| `control_motor_usb_l298n` | Control serial USB + L298N (desarrollo con PC) |
| `control_motor_rpi` | Control GPIO UART USART3 + L298N (producción RPi) |
| `control_6_motors_l298n` | Drive diferencial 6 ruedas, interfaz de comandos |
| `test_controller` | RoverController con ErasedMotor y detección de stall |
| `test_encoders` | Lectura de encoders Hall (INT0–INT5) + MPU-6050 |
| `test_proximity` | HC-SR04 + VL53L0X — medición de distancia |
| `test_l298n` | Test de un solo motor L298N |
| `test_bts7960` | Test motor alta potencia BTS7960 |
| `test_servo` | Barrido servo 0–180° (Timer1) |
| `test_rpi_communication` | Echo USART3 — verifica cableado GPIO RPi |
| `test_msm_protocol` | Validador del protocolo MSM por USB |
| `test_acs712_current` | Lectura de corriente ACS712 en A0 |
| `test_lm335_temperature` | Lectura de temperatura LM335 en A6 |
| `debug_motors_l298n` | Debug de pinout — activa un motor a la vez |
| `debug_hcsr04_raw` | Lecturas crudas HC-SR04 |
| `validate_protocol` | Validador de protocolo (terminal serie PC) |

---

## Protocolo MSM

Comunicación ASCII con terminador `\n` a 115200 baud 8N1.

### Comandos RPi5 → Arduino

| Comando | Acción |
|---------|--------|
| `PING` | Keepalive — resetea watchdog (~2 s sin PING → FAULT) |
| `STB` | Standby (motores parados) |
| `EXP:<l>:<r>` | Explorar con velocidades 0–100 (ej: `EXP:80:80`) |
| `AVD:L` / `AVD:R` | Evasión izquierda / derecha |
| `RET` | Retroceder |
| `FLT` | Forzar FAULT desde HLC |
| `RST` | Reset → Standby |

### Respuestas Arduino → RPi5

| Respuesta | Significado |
|-----------|-------------|
| `PONG` | Respuesta a PING |
| `ACK:<STATE>` | Transición confirmada (ej: `ACK:EXP`) |
| `ERR:ESTOP` | Comando rechazado (Arduino en FAULT) |
| `ERR:WDOG` | Watchdog expirado → FAULT |
| `ERR:UNKNOWN` | Comando no reconocido |

### Telemetría asíncrona (cada ~1 s)

```
TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C:<B0>:<B1>:<B2>:<B3>:<B4>:<B5>C:<DIST>mm:<EL>:<ER>
```

| Campo | Descripción |
|-------|-------------|
| `SAFETY` | `NORMAL` / `WARN` / `LIMIT` / `FAULT` |
| `STALL` | 6 bits '0'/'1': bit5=FR … bit0=RL |
| `TS` | ms desde boot (u32, monotónico) |
| `MV` / `MA` | tensión y corriente de batería (INA226) |
| `I0`–`I5` | corriente por motor en mA (ACS712, FR→RL) |
| `T` | temperatura ambiente en °C (LM335) |
| `B0`–`B5` | temperatura celdas batería en °C (NTC) |
| `DIST` | distancia frontal en mm (VL53L0X ToF) |
| `EL` | acumulador encoder izquierdo: FL+CL+RL (odometría) |
| `ER` | acumulador encoder derecho: FR+CR+RR (odometría) |

Ejemplo:
```
TLM:NORMAL:000000:1000ms:14800mV:1200mA:1150:980:1100:1050:1200:1180:27C:28:29:28:30:29:28C:342mm:60:62
```

---

## Documentación

| Doc | Contenido |
|-----|-----------|
| [`docs/the_pins_connections.md`](docs/the_pins_connections.md) | Mapa completo de pines del ATmega2560 |
| [`docs/rpi5_uart_communication.md`](docs/rpi5_uart_communication.md) | Comunicación RPi5 ↔ Arduino, protocolo MSM, cableado |
| [`docs/consideration_implementation.md`](docs/consideration_implementation.md) | Decisiones de diseño: ErasedMotor, timers, TLM, sensores, config.rs |
| [`docs/motors.md`](docs/motors.md) | Arquitectura de motores, PWM, encoders |
| [`docs/vl53l0x.md`](docs/vl53l0x.md) | Sensor ToF VL53L0X (táctica, D42/D43 soft I2C) |
| [`docs/hc_sr04.md`](docs/hc_sr04.md) | Sensor HC-SR04 (emergencia, D38/D39), API Result |
| [`docs/acs712.md`](docs/acs712.md) | Sensor de corriente ACS712-30A, protección graduada |
| [`docs/lm335.md`](docs/lm335.md) | Sensor temperatura LM335 |
| [`docs/encoder.md`](docs/encoder.md) | Encoders Hall, ISRs, stall detection |
| [`docs/peripheral_timers.md`](docs/peripheral_timers.md) | Asignación de timers PWM |
| [`docs/decision-log.md`](docs/decision-log.md) | Historial de decisiones de arquitectura |
| [`docs/testing.md`](docs/testing.md) | Guía de testing: flags, 131 tests x86, flujo de trabajo, troubleshooting |

---

## Referencias de hardware

- **ATmega2560 Datasheet** — [Microchip](https://www.microchip.com/en-us/product/atmega2560)
- **Arduino Mega 2560 Pin Mapping** — [Arduino docs](https://docs.arduino.cc/hacking/hardware/PinMapping2560)
- **HC-SR04 Datasheet** — [SparkFun](https://cdn.sparkfun.com/datasheets/Sensors/Proximity/HCSR04.pdf)
- **VL53L0X Datasheet** — [ST Microelectronics](https://www.st.com/en/imaging-and-photonics-solutions/vl53l0x.html)
- **ACS712 Datasheet** — [Allegro MicroSystems](https://www.allegromicro.com/en/products/sense/current-sensor-ics/zero-to-fifty-amp-integrated-conductor-sensor-ics/acs712)




## License

This project is distributed under the MIT License. See the LICENSE file for details.

---

## Author

Fabián Alonso Gómez Quesada     
Instituto Tecnológico de Costa Rica (TEC)        
School of Electronics Engineering           
SETEC Lab – Space Systems Laboratory     
