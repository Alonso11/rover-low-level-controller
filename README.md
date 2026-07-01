<!-- Version: v2.1 -->
# Rover Low-Level Controller (LLC) — Rust/AVR

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Systems Engineering](https://img.shields.io/badge/Focus-Systems%20Engineering-blue.svg)](#)

## Overview

Firmware modular para un rover de 6 ruedas en **Rust embebido** sobre el
**ATmega2560** (Arduino Mega 2560). Actúa como controlador de bajo nivel (LLC):
recibe comandos del protocolo MSM desde la **Raspberry Pi 5** (Yocto Linux)
por UART y gestiona motores, encoders, sensores de proximidad y corriente.

La interfaz principal de build/flash/test es el **Makefile** — ver [Build & Flash con
Makefile](#build--flash-con-makefile).

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

## Prerrequisitos

```bash
# Herramientas AVR (Debian/Ubuntu)
sudo apt-get install gcc-avr avr-libc

# Toolchain Rust nightly + rust-src
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly

# Entorno Python (tests, scripts)
make setup
```

## Build & Flash con Makefile

El **Makefile** es la interfaz principal. Compila y flashea automáticamente con
**Ravedude** (el flasher AVR via USB). Solo necesitas definir `PORT`:

```bash
# Flash del firmware principal (ACS712-30A, default)
make flash PORT=/dev/ttyUSB0

# Flash con variantes de configuración (feature flags)
make flash-allbts      PORT=/dev/ttyUSB0   # 6× BTS7960 (operativo)
make flash-mixed       PORT=/dev/ttyUSB0   # FR/FL=L298N, resto=BTS7960
make flash-20a         PORT=/dev/ttyUSB0   # 6× ACS712-20A
make flash-allbts-rl-l298n PORT=/dev/ttyUSB0  # 5× BTS + RL=L298N

# Flash examples de depuración
make flash-debug-all   PORT=/dev/ttyUSB0   # 6 motores, 3s forward
make flash-debug-only-fr PORT=/dev/ttyUSB0 # solo FR (bisección)
```

### Targets principales

| Target | Descripción |
|--------|-------------|
| `make flash` | Firmware principal — ACS712-30A (default) |
| `make flash-allbts` | 6× BTS7960 + NTC4 + sin MPU/VL53L0X/LM335 (operativo) |
| `make flash-allbts-rl-l298n` | 5× BTS7960 + RL como L298N |
| `make flash-mixed` | FR/FL=L298N, CR/CL/RR/RL=BTS7960 |
| `make flash-20a` | 6× ACS712-20A |
| `make flash-allbts-bringup` | BTS7960 sin stall/OC/HC-SR04 (bring-up) |
| `make flash-allbts-rampfix` | BTS7960 + ramp-converge + idle-disable (fix FR en FAULT) |
| `make flash-no-oc` | Sobrecorriente desactivada (pruebas HW) |
| `make flash-motors-only` | Solo motores (sin OC/stall/LM335/HC-SR04) |
| `make test-unit` | Tests x86: Rust (3 suites) + pytest unit |
| `make monitor` | `screen` serial a 115200 (Ctrl+A → k para salir) |
| `make setup` | `uv sync` — instala deps Python |
| `make clean` | `cargo clean` + elimina `.venv` y `__pycache__` |

### Feature flags

Se activan vía `--features` en Cargo. El Makefile los combina en cada target:

| Flag | Efecto |
|------|--------|
| `all-bts7960` | 6 motores con driver BTS7960 (alta potencia) |
| `mixed-drivers` | FR/FL=L298N, CR/CL/RR/RL=BTS7960 |
| `all-20a` | 6× ACS712-20A (vs 30A default) |
| `rl-l298n` | RL como L298N (combinado con all-bts7960) |
| `no-oc` | Desactiva protección de sobrecorriente |
| `no-stall` | Desactiva detección de stall por encoder |
| `no-lm335` | Desactiva LM335 (temp. ambiente) |
| `no-hcsr04` | Desactiva HC-SR04 por completo |
| `hcsr04-no-fault` | HC-SR04 leído pero sin faultear por proximidad |
| `no-tof` | Desactiva VL53L0X |
| `no-mpu` | Desactiva MPU-6050 + EKF |
| `no-ntc` | Desactiva lectura de NTC de batería |
| `ntc4` | Solo 4 NTC conectados (A7–A10) |
| `ramp-converge` | Fuerza rampa a 0 en estados sin tracción |
| `no-watchdog` | Desactiva watchdog de comms |
| `idle-disable` | De-energiza drivers en Fault/Safe/Standby |

**Uso directo sin Makefile:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --release -Zjson-target-spec -Zbuild-std=core \
  --features all-bts7960,ntc4,no-tof,no-mpu,no-lm335
```

**Compilar solo (sin flashear):**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --release \
  -Zjson-target-spec -Zbuild-std=core
```

---

## Tests

### Tests de lógica (x86, sin hardware)

Validan la MSM, drivers analógicos y lógica de motores en la máquina de desarrollo:

```bash
make test-unit
# o directamente:
./test_native.sh
```

| Suite | Tests | Cobertura |
|-------|-------|-----------|
| `state_machine_test / ekf_test` | 46 | Todas las transiciones MSM, watchdog, format_response, parser TLM (incluyendo campos odometría) |
| `sensors_test` | 64 | ACS712 conversión mA, LM335 conversión °C, NTC interpolación LUT, umbrales Warn/Limit/Fault |
| `motor_logic_test` | 28 | Speed mapping, signos de dirección L298N/BTS7960, SixWheelRover, ErasedMotor |

### Tests de hardware (PC + Arduino via USB)

Requieren el Arduino conectado y el firmware flasheado. Se ejecutan con `make`:

```bash
# Escaneo I2C (verificar 0x29/0x40/0x68)
make i2c-scan PORT=/dev/ttyUSB0

# Sensores individuales (INT-04b)
make test-sensors PORT=/dev/ttyUSB0

# Protocolo MSM — 13 tests (INT-05)
make test-protocol PORT=/dev/ttyUSB0

# Motores interactivo (INT-07, requiere debug_motors_l298n flasheado)
make test-motors PORT=/dev/ttyUSB0

# Motores con firmware principal (INT-07 en EXP mode)
make test-motors-main PORT=/dev/ttyUSB0

# Calibración odometría + sensores (INT-08)
make calibrate PORT=/dev/ttyUSB0

# Suite completa INT-04b + INT-05 secuencial
make int-all PORT=/dev/ttyUSB0
```

| Script | Firmware requerido | Descripción |
|--------|-------------------|-------------|
| `tests/hardware/test_msm_protocol.py` | Firmware principal (v2.10+) | 13 tests automáticos del protocolo MSM + validación formato TLM |
| `tests/hardware/test_motors_debug.py` | `examples/debug_motors_l298n` | Control interactivo F/B/S para verificar cada motor individualmente |
| `tests/hardware/test_sensors_individual.py` | Firmware principal | Verifica 8 sensores en rango TLM |
| `tests/hardware/test_tf02.py` | Firmware principal | Verifica TF02 LiDAR UART |
| `tests/hardware/i2c_scan.py` | Firmware principal | Escanea dirección I2C |
| `tests/hardware/calibrate_odometry.py` | Firmware principal | Calibración de odometría

Ver [`docs/testing.md`](docs/testing.md) para la explicación completa de flags,
troubleshooting y el flujo de trabajo recomendado antes de cada commit.

---

## Examples

Programas completos para flashear al Arduino. Cada uno es un binario AVR independiente.
Se flashean con `make flash-<nombre>` o manualmente:

```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --example <nombre> --release -Zjson-target-spec -Zbuild-std=core
```

### Firmware principal

| Example | `make` target | Descripción |
|---------|---------------|-------------|
| `control_motor_usb_l298n` | — | Control serial USB + L298N (desarrollo con PC) |
| `control_motor_rpi` | — | Control GPIO UART USART3 + L298N (producción RPi) |
| `control_6_motors_l298n` | — | Drive diferencial 6 ruedas, interfaz de comandos |

### Tests de subsistemas

| Example | `make` target | Descripción |
|---------|---------------|-------------|
| `test_controller` | — | RoverController con ErasedMotor y detección de stall |
| `test_encoders` | — | Lectura de encoders Hall (INT0–INT5) + MPU-6050 |
| `test_proximity` | — | HC-SR04 + VL53L0X — medición de distancia |
| `test_l298n` | — | Test de un solo motor L298N |
| `test_bts7960` | — | Test motor alta potencia BTS7960 |
| `test_servo` | — | Barrido servo 0–180° (Timer1) |
| `test_rpi_communication` | — | Echo USART3 — verifica cableado GPIO RPi |
| `test_msm_protocol` | — | Validador del protocolo MSM por USB |
| `test_acs712_current` | — | Lectura de corriente ACS712 en A0 |
| `test_lm335_temperature` | — | Lectura de temperatura LM335 en A6 |
| `test_serial_echo` | — | Echo serie por USB |
| `test_hcsr04_filtered` | — | HC-SR04 con filtro de medición |
| `test_ina3221_dual_cell` | — | INA3221 dual cell battery monitor |
| `test_6_motors_all_bts` | `flash-test-6motors` | 6 motores BTS7960 fwd 40% 3s |
| `test_motors_encoders_acs` | `flash-test-motors-enc-acs` | Motores + encoders + ACS712 (CSV) |
| `test_motors_encoders_acs_rl_l298n` | `flash-test-motors-enc-acs-rl-l298n` | 5×BTS + RL=L298N + encoders + ACS |
| `test_only_rl_l298n` | `flash-test-only-rl` | Solo RL via L298N |
| `test_integration_full` | `flash-test-integration-full` | Validación integral (todos los sensores) |
| `test_integration_no_ina` | `flash-test-integration-no-ina` | Integración sin INA3221 |

### Debug / bisección de hardware

| Example | `make` target | Descripción |
|---------|---------------|-------------|
| `debug_motors_l298n` | — | Debug de pinout — activa un motor a la vez |
| `debug_motors_sequential` | `flash-debug-motors-seq` | 1 motor a la vez, pinout MSM |
| `debug_all_motors` | `flash-debug-all` | 6 motores forward 3s |
| `debug_all_motors_bts` | `flash-debug-all-bts` | 6× BTS7960 forward 3s |
| `debug_front_center` | `flash-debug-front-center` | FR+FL+CR+CL forward 3s (sin RR/RL) |
| `debug_only_fr` | `flash-debug-only-fr` | Solo FR (máximo aislamiento) |
| `debug_only_fl` | `flash-debug-only-fl` | Solo FL, HOLD 15s |
| `debug_only_cr` | `flash-debug-only-cr` | Solo CR |
| `debug_only_cl` | `flash-debug-only-cl` | Solo CL |
| `debug_only_rr` | `flash-debug-only-rr` | Solo RR |
| `debug_only_rl` | `flash-debug-only-rl` | Solo RL (L298N) |
| `debug_only_rl_bts` | `flash-debug-only-rl-bts` | Solo RL (BTS7960) |
| `debug_only_fr_fl` | `flash-debug-only-fr-fl` | FR+FL forward 3s |
| `debug_only_cr_cl` | `flash-debug-only-cr-cl` | CR+CL forward 3s |
| `debug_only_rr_rl` | `flash-debug-only-rr-rl` | RR(BTS)+RL(L298N) |
| `debug_only_rr_rl_bts` | `flash-debug-only-rr-rl-bts` | RR+RL ambos BTS |
| `debug_only_fr_enc` | `flash-debug-fr-enc` | FR vs FL ratio encoder |
| `debug_only_fl_enc` | `flash-debug-only-fl-enc` | FL motor + encoder (CSV) |
| `debug_encoders_drive` | `flash-debug-encoders-drive` | 6 motores + 6 encoders (CSV) |
| `debug_encoders_acs` | `flash-debug-encoders-acs` | 6 BTS + 6 encoders + 6 ACS |
| `debug_rr_acs` | `flash-debug-rr-acs` | RR motor + ACS712 A4 |
| `debug_fl_allint` | `flash-debug-fl-allint` | FL solo, 6 INT habilitados |
| `debug_fl_bisect` | `flash-debug-fl-bisect` | FL continuo, desenmascara INT |
| `debug_hcsr04_raw` | — | Lecturas crudas HC-SR04 |
| `debug_hcsr04_d38` | `flash-debug-hcsr04-d38` | HC-SR04 D38/D39 rango completo |

### Utilidades

| Example | `make` target | Descripción |
|---------|---------------|-------------|
| `validate_protocol` | — | Validador de protocolo (terminal serie PC) |
| `ina3221_battery_logger` | `flash-ina-logger` | Logger curva de descarga V, 3 bancos |
| `medir_bateria` | `flash-medir-bateria` | Medición de batería (2º Mega, divisor A0) |
| `medir_bateria_raw` | `flash-medir-bateria-raw` | ADC crudo, factor en Python |

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
