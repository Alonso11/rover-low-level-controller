# RPi 5 ↔ Arduino Mega 2560 Communication
<!-- Version: v2.0 -->

This document explains the communication architecture between the
Raspberry Pi 5 (high-level controller, running a custom Yocto image)
and the ATmega2560 (low-level firmware), including the rationale for
choosing UART-over-USB, the physical wiring, the software protocol,
and how both sides are configured.

---

## 1. System Overview

```
┌─────────────────────────────────┐        ┌────────────────────────────┐
│  Raspberry Pi 5                 │        │  Arduino Mega 2560         │
│  (Yocto — meta-olympus)         │        │  (rover-low-level-controller)│
│                                 │  USB   │                            │
│  rover_bridge.so (Rust/PyO3) ───┼────────┼─── USART0 (USB/testing)   │
│  /dev/arduino_mega              │        │                            │
│                                 │ GPIO   │                            │
│  (producción — USART3) ─────────┼────────┼─── USART3 (D14/D15)       │
└─────────────────────────────────┘        └────────────────────────────┘
```

The RPi 5 runs a **custom Yocto Linux image** (not Raspbian). It sends
ASCII commands over serial following the **MSM protocol** and receives
text responses and telemetry. The Arduino firmware handles the real-time
motor and sensor control loop.

**USART assignment:**

| USART | Pines | Uso | Estado |
|-------|-------|-----|--------|
| USART0 | USB (ATmega16U2) | Desarrollo / testing con cable USB | Activo en v2.7 |
| USART3 | D14 (TX3) / D15 (RX3) | Producción GPIO directo RPi5 | Pendiente de flash |
| USART1 | D18 (TX1) / D19 (RX1) | **Libre** — ocupado por encoders INT2/INT3 | No usar |
| USART2 | D16 (TX2) / D17 (RX2) | Reservado (TF-Luna, no instanciado) | Libre |

> USART3 es el puerto de producción cuando no hay cable USB. El cambio
> USART0 → USART3 en `main.rs` está pendiente de flash al hardware.
> Ver `docs/decision-log.md` §Semana 4 — Pendiente.

---

## 2. Why UART

Several communication interfaces were evaluated:

| Interface | Verdict | Reason |
|---|---|---|
| **UART over USB** | **Chosen (primary/dev)** | Zero hardware modification, isolated ground, works with stock Mega |
| GPIO UART (USART3) | Chosen (production) | No USB cable needed in field; requires voltage divider (5V→3.3V) |
| SPI | Discarded | Requires master/slave negotiation; not ideal for continuous streaming |
| I²C | Discarded | Multi-master arbitration; not designed for point-to-point command links |
| Ethernet / Wi-Fi | Out of scope | Requires networking stack; adds latency and complexity |

USB was preferred during development because:
- **No voltage level conversion needed** — the USB bridge chip (CH340 or ATmega16U2) handles isolation.
- **Reliable device identification** — udev rules create a persistent `/dev/arduino_mega` symlink.
- **Arduino bootloader compatibility** — DTR-based reset works over USB.
- **Simpler wiring** — one cable for both power and data during development.

---

## 3. Physical Connection

### Primary — USB Serial (desarrollo)

```
Arduino Mega 2560                     Raspberry Pi 5
─────────────────                     ──────────────
USB port (CH340 / ATmega16U2) ──USB── USB port → /dev/arduino_mega
```

The Arduino Mega appears as `/dev/ttyUSB0` (CH340 clone) or `/dev/ttyACM0`
(genuine Mega with ATmega16U2). Udev rules in the Yocto image create a
stable symlink `/dev/arduino_mega` for both:

```
# VID:PID for genuine Arduino Mega
SUBSYSTEM=="tty", ATTRS{idVendor}=="2341", ATTRS{idProduct}=="0042",
  SYMLINK+="arduino_mega", MODE="0666"

# VID:PID for CH340/CH341 clone
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="7523",
  SYMLINK+="arduino_mega", MODE="0666"

# Fallback
SUBSYSTEM=="tty", KERNEL=="ttyACM*", SYMLINK+="arduino_mega", MODE="0666"
SUBSYSTEM=="tty", KERNEL=="ttyUSB*", SYMLINK+="arduino_mega", MODE="0666"
```

Firmware correspondiente: `main.rs` con `arduino_hal::default_serial!(dp, pins, 115200)` → USART0.

### Secondary — GPIO UART / USART3 (producción)

Para despliegue en campo sin cable USB. Usa **USART3** (D14/D15) que deja
libres D18/D19 para los encoders de los motores centrales (INT2/INT3).

> ⚠️ Por qué **no** USART1: D18 y D19 (USART1) son los pines INT3/INT2 del
> ATmega2560 — necesarios para los encoders del centro. Usar USART1 impide
> instalar los 6 encoders. Ver `docs/consideration_implementation.md` §6.

```
Arduino Mega 2560                     Raspberry Pi 5
─────────────────                     ──────────────
D15  RX3 (PJ0) ◄──────────────────── GPIO14 (TX)  pin 8
D14  TX3 (PJ1) ──[1kΩ]──┬──────────► GPIO15 (RX)  pin 10
                         [2kΩ]
                          │
                         GND
GND             ◄──────────────────── GND           pin 6
```

> ⚠️ El ATmega2560 TX (D14) emite **5V**. El RPi 5 GPIO es **3.3V only**.
> El divisor resistivo es obligatorio — sin él el SoC del RPi 5 queda dañado.
> La dirección RPi 5 TX → Arduino RX es segura sin divisor
> (ATmega2560 acepta 3.3V como HIGH válido).

El UART primario del RPi 5 (`/dev/ttyAMA0`) está habilitado en la imagen
Yocto via `enable_uart=1` y `dtoverlay=disable-bt` en `RPI_EXTRA_CONFIG`
(Bluetooth desactivado para liberar el hardware UART).

Firmware correspondiente (pendiente):
```rust
// main.rs — cambio pendiente USART0 → USART3
let serial = arduino_hal::Usart::new(
    dp.USART3,
    pins.d15.into_pull_up_input(),   // RX3 ← RPi TX
    pins.d14.into_output(),          // TX3 → RPi RX (via divisor)
    115200.into_baudrate(),
);
```

---

## 4. Protocolo MSM (Master State Machine)

`CommandInterface` (`src/command_interface/mod.rs`) implementa el protocolo
ASCII MSM sobre cualquier USART del ATmega2560.

### Formato de trama

```
<COMANDO>\n
```

Comandos en ASCII plano, terminados con `\n`. Buffer interno de **80 bytes**.

### Comandos RPi5 → Arduino

| Comando | Acción MSM |
|---------|------------|
| `PING` | Keepalive — resetea watchdog Arduino (~2 s sin PING → FAULT) |
| `STB` | Standby (motores parados) |
| `EXP:<l>:<r>` | Explorar con velocidades -99–99 (positivo=avance, negativo=retroceso; ej: `EXP:80:80`) |
| `AVD:L` | Girar izquierda (evasión) |
| `AVD:R` | Girar derecha (evasión) |
| `RET` | Retroceder |
| `FLT` | Forzar FAULT desde HLC |
| `RST` | Reset → Standby |

### Respuestas Arduino → RPi5

| Respuesta | Significado |
|-----------|-------------|
| `PONG` | Respuesta a PING |
| `ACK:<STATE>` | Transición confirmada (ej: `ACK:EXP`, `ACK:STB`) |
| `ERR:ESTOP` | Comando rechazado (Arduino en FAULT) |
| `ERR:WDOG` | Watchdog expirado → FAULT |
| `ERR:UNKNOWN` | Comando no reconocido |

### Telemetría asíncrona (TLM)

El Arduino emite un frame TLM cada ~1 s sin ser solicitado:

```
TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C:<B0>:<B1>:<B2>:<B3>:<B4>:<B5>C:<DIST>mm\n
```

| Campo | Descripción |
|-------|-------------|
| SAFETY | Estado: NORMAL / WARN / LIMIT / FAULT |
| STALL | Máscara stall 6 bits (bit0=FR … bit5=RL) |
| TS | Tick Arduino en ms (monotónico desde arranque) |
| MV | Tensión bus batería en mV (INA226, D42/D43). 0 = sin lectura |
| MA | Corriente total batería en mA con signo (INA226). 0 = sin lectura |
| I0–I5 | Corrientes motores FR/FL/CR/CL/RR/RL en mA (ACS712) |
| T | Temperatura ambiente (LM335, A6) en °C |
| B0–B5 | Temperaturas celdas batería (NTC A7–A12) en °C |
| DIST | Distancia frontal (VL53L0X, D42/D43) en mm (0 = sin lectura) |

Ejemplo:
```
TLM:NORMAL:000000:1000ms:14800mV:1200mA:1150:980:1100:1050:1200:1180:27C:28:29:28:30:29:28C:342mm
```

El HLC (`rover_bridge.so`) lee los TLM con `recv_tlm()` (timeout 50 ms, no bloqueante)
y descarta los frames TLM intercalados en `send_command()`.

### Non-blocking polling

`poll_command()` drena el FIFO USART y retorna inmediatamente si no ha
llegado una trama completa, manteniendo el loop de control desbloqueado:

```rust
loop {
    if interface.poll_command() {
        let cmd = interface.get_command();
        // handle MSM command
    }
    // sensor reads, motor updates, TLM emission
}
```

---

## 5. RPi 5 Side — Yocto Configuration

La imagen RPi 5 se construye con la capa **meta-olympus**. El soporte
serie está configurado vía:

**`build/conf/local.conf`** — UART hardware y Bluetooth:
```
RPI_EXTRA_CONFIG = " \
    enable_uart=1 \n \
    dtoverlay=disable-bt \n \
"
```

**`recipes-core/custom-udev-rules`** — instala `99-arduino.rules` con
el symlink persistente `/dev/arduino_mega`.

**`recipes-apps/python3-rover-bridge`** — extensión Rust nativa (PyO3)
que encapsula el puerto serie con `Mutex` y lo expone a Python:

```rust
// Rover::new() en la extensión PyO3
let port = serialport::new(&port_name, baud_rate)
    .timeout(Duration::from_millis(300))
    .open()?;

std::thread::sleep(Duration::from_secs(2)); // espera reset Arduino por DTR
```

```python
# Uso desde Python (olympus_controller.py)
import rover_bridge

rover = rover_bridge.Rover("/dev/arduino_mega", 115200)
resp = rover.send_command("PING")    # → "PONG"
resp = rover.send_command("STB")     # → "ACK:STB"
resp = rover.send_command("EXP:80:80")  # → "ACK:EXP"

tlm = rover.recv_tlm()  # → "TLM:NORMAL:..." o None (50 ms timeout)
```

---

## 6. Baud Rate Calculation

Ambos lados deben coincidir. El ATmega2560 a 16 MHz:

```
UBRR = (f_CPU / (16 × baud)) − 1
     = (16,000,000 / (16 × 115200)) − 1
     ≈ 8   →  error baud real ≈ 0.16%
```

0.16% está dentro de la tolerancia UART de ±2% — sin errores de framing en la práctica.

---

## 7. Relevant Files

| File | Side | Description |
|---|---|---|
| `src/command_interface/mod.rs` | Arduino | Buffer de protocolo MSM y wrapper USART |
| `src/state_machine/mod.rs` | Arduino | Máquina de estados maestra (5 estados, watchdog) |
| `src/main.rs` | Arduino | Loop principal: watchdog → sensores → MSM → motores → TLM |
| `examples/control_motor_usb_l298n.rs` | Arduino | Test USB USART0 + L298N (desarrollo) |
| `examples/test_rpi_communication.rs` | Arduino | Echo test USART3 — verifica cableado GPIO |
| `layers/meta-olympus/recipes-core/custom-udev-rules/` | RPi 5 | Reglas udev para `/dev/arduino_mega` |
| `layers/meta-olympus/recipes-apps/python3-rover-bridge/` | RPi 5 | Rust/PyO3 bridge (`rover_bridge.so`) |
| `layers/meta-olympus/recipes-apps/python3-rover-bridge/files/test_bridge_interactive.py` | RPi 5 | Control manual MSM interactivo |
| `layers/meta-olympus/recipes-apps/python3-rover-bridge/files/test_bridge.py` | RPi 5 | Test automático del bridge Rust |
| `layers/meta-olympus/recipes-apps/python3-rover-bridge/files/olympus_controller.py` | RPi 5 | Controlador HLC completo (v1.6) |
