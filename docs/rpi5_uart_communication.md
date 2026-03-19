# RPi 5 ↔ Arduino Mega 2560 Communication

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
│  rover_bridge.so (Rust/PyO3) ───┼────────┼─── CommandInterface        │
│  /dev/arduino_mega              │        │    (USART0 / USART1)       │
└─────────────────────────────────┘        └────────────────────────────┘
```

The RPi 5 runs a **custom Yocto Linux image** (not Raspbian). It sends
ASCII commands over serial and receives text responses. The Arduino
firmware handles the real-time motor and sensor control loop.

---

## 2. Why UART

Several communication interfaces were evaluated:

| Interface | Verdict | Reason |
|---|---|---|
| **UART over USB** | **Chosen (primary)** | Zero hardware modification, isolated ground, works with stock Mega |
| GPIO UART (hardware) | Available (secondary) | Requires level shifter (5V→3.3V); useful if USB port is occupied |
| SPI | Discarded | Requires master/slave negotiation; not ideal for continuous streaming |
| I²C | Discarded | Multi-master arbitration; not designed for point-to-point command links |
| Ethernet / Wi-Fi | Out of scope | Requires networking stack; adds latency and complexity |

USB was preferred over direct GPIO UART because:
- **No voltage level conversion needed** — the USB bridge chip (CH340 or ATmega16U2) handles isolation.
- **Reliable device identification** — udev rules create a persistent `/dev/arduino_mega` symlink independent of enumeration order.
- **Arduino bootloader compatibility** — DTR-based reset works over USB; GPIO UART would require a manual reset circuit.
- **Simpler wiring** — one cable for both power and data during development.

---

## 3. Physical Connection

### Primary — USB Serial

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

Arduino firmware using USB: `examples/control_motor_usb_l298n.rs`

```rust
let serial = arduino_hal::default_serial!(dp, pins, 115200); // USART0
```

### Secondary — GPIO UART (hardware)

Available when the USB port needs to be freed for another peripheral.
Uses USART1 (D18/D19) and requires a **voltage divider** on the TX line:

```
Arduino Mega 2560              Raspberry Pi 5
─────────────────              ──────────────
D19  RX1 (PD2) ◄──────────── GPIO14 (TX)  pin 8
D18  TX1 (PD3) ──[1kΩ]──┬──► GPIO15 (RX)  pin 10
                         [2kΩ]
                          │
                         GND
GND            ◄────────── GND             pin 6
```

> ⚠️ The ATmega2560 TX outputs **5V**. The RPi 5 GPIO is **3.3V only**.
> The divider is mandatory — skipping it will damage the RPi 5 SoC.
> The RPi 5 TX → Arduino RX direction is safe without conversion
> (ATmega2560 accepts 3.3V as a valid HIGH).

The primary UART on RPi 5 (`/dev/ttyAMA0`) is enabled in the Yocto image
via `enable_uart=1` and `dtoverlay=disable-bt` in `RPI_EXTRA_CONFIG`
(Bluetooth is disabled to free the hardware UART).

Arduino firmware using GPIO UART: `examples/control_motor_rpi.rs`

```rust
let serial = arduino_hal::Usart::new(
    dp.USART1,
    pins.d19.into_pull_up_input(),   // RX1 ← RPi TX
    pins.d18.into_output(),          // TX1 → RPi RX (via divider)
    115200.into_baudrate(),
);
```

---

## 4. Command Protocol

`CommandInterface` (`src/command_interface/mod.rs`) implements a minimal
ASCII text protocol on top of any ATmega2560 USART peripheral.

### Frame format

```
<COMMAND>\n
```

Commands are plain ASCII, terminated by `\n` or `\r`. The internal
buffer is **32 bytes** — commands longer than 31 bytes are truncated.

### Command set

| Byte | Command | Action |
|---|---|---|
| `F` / `f` | Forward | All motors forward |
| `B` / `b` | Backward | All motors backward |
| `L` / `l` | Left | Tank-turn left |
| `R` / `r` | Right | Tank-turn right |
| `S` / `s` | Stop | All motors stop |

### Response

The Arduino replies with plain text terminated by `\n`:

```
OK: Ejecutando ADELANTE\n
```

### Non-blocking polling

`poll_command()` drains the USART FIFO and returns immediately if no
complete command has arrived, keeping the control loop unblocked:

```rust
loop {
    if interface.poll_command() {
        let cmd = interface.get_command();
        // handle cmd
    }
    // sensor reads, motor updates — run every iteration
}
```

---

## 5. RPi 5 Side — Yocto Configuration

The RPi 5 image is built with the **meta-olympus** Yocto layer. Serial
support is configured via:

**`build/conf/local.conf`** — hardware UART and Bluetooth override:
```
RPI_EXTRA_CONFIG = " \
    enable_uart=1 \n \
    dtoverlay=disable-bt \n \
"
```

**`recipes-core/custom-udev-rules`** — installs `99-arduino.rules` which
creates the `/dev/arduino_mega` persistent symlink.

**`recipes-core/python3-rover-bridge`** — Rust native extension (PyO3)
that wraps the serial port with a thread-safe `Mutex` and exposes it
to Python:

```rust
// Rover::new() in the PyO3 extension
let port = serialport::new(&port_name, baud_rate)
    .timeout(Duration::from_millis(100))
    .open()?;

std::thread::sleep(Duration::from_secs(2)); // Arduino bootloader reset wait
```

```python
# Python usage
import rover_bridge

rover = rover_bridge.Rover("/dev/arduino_mega", 115200)
rover.send_command("F")   # sends "F\n"
```

---

## 6. Baud Rate Calculation

Both sides must match. The ATmega2560 at 16 MHz:

```
UBRR = (f_CPU / (16 × baud)) − 1
     = (16,000,000 / (16 × 115200)) − 1
     ≈ 8   →  actual baud rate error ≈ 0.16%
```

0.16% is well within the ±2% UART tolerance — no framing errors in practice.

---

## 7. Relevant Files

| File | Side | Description |
|---|---|---|
| `src/command_interface/mod.rs` | Arduino | Protocol buffer and USART wrapper |
| `examples/control_motor_usb_l298n.rs` | Arduino | USB serial + L298N motor control |
| `examples/control_motor_rpi.rs` | Arduino | GPIO UART + BTS7960 motor control |
| `examples/test_rpi_communication.rs` | Arduino | USART1 echo test — verifies GPIO wiring |
| `examples/validate_protocol.rs` | Arduino | Protocol validator via USB (PC terminal) |
| `layers/meta-olympus/recipes-core/custom-udev-rules/` | RPi 5 | udev rules for `/dev/arduino_mega` |
| `layers/meta-olympus/recipes-core/python3-rover-bridge/` | RPi 5 | Rust/PyO3 serial bridge |
