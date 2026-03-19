<!-- Version: v1.2 -->
# Rover Low-Level Controller (Rust)

Modular firmware for a 6-wheel rover, implemented in **embedded Rust** for the **ATmega2560** (Arduino Mega 2560). Acts as the hardware abstraction layer (HAL), receiving commands from a **Raspberry Pi 5** (Yocto Linux) over UART and driving motors, encoders, and proximity sensors.

## Project Structure

```
src/
├── lib.rs                  # Library entry point
├── motor_control/
│   ├── mod.rs              # Motor / Servo traits
│   ├── l298n.rs            # L298N driver + SixWheelRover
│   ├── bts7960.rs          # BTS7960 high-power driver
│   ├── servo.rs            # Software PWM servo driver
│   └── erased.rs           # ErasedMotor — type erasure for motor arrays
├── sensors/
│   ├── mod.rs              # ProximitySensor trait
│   ├── encoder.rs          # HallEncoder (interrupt-safe)
│   ├── hc_sr04.rs          # HC-SR04 ultrasonic sensor
│   └── tf_luna.rs          # TF-Luna LiDAR (UART)
├── controller/
│   └── mod.rs              # RoverController — 6-channel stall detection
└── command_interface/
    └── mod.rs              # UART command buffer (RPi protocol)

examples/                   # Ready-to-flash programs
tests/                      # Host-side logic tests
docs/                       # Hardware diagrams, design notes
```

## Build

Requires a nightly Rust toolchain with `rust-src` and the AVR GCC toolchain:

```bash
# Install AVR tools (Debian/Ubuntu)
sudo apt-get install gcc-avr avr-libc

# Add nightly + rust-src
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

**Verify the full library compiles:**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --target avr-none -Z build-std=core
```

## Examples

Build any example with:
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --example <name> --target avr-none -Z build-std=core
```

| Example | Description |
|---|---|
| `control_6_motors_l298n` | 6-wheel differential drive via L298N, serial command interface |
| `test_controller` | RoverController with ErasedMotor and stall detection |
| `test_encoders` | Hall effect encoder readout |
| `test_proximity` | HC-SR04 and TF-Luna distance measurement |
| `test_l298n` | Single L298N motor test |
| `test_bts7960` | BTS7960 high-power motor test |
| `test_servo` | Servo sweep 0–180° |
| `control_motor_rpi` | RPi GPIO UART motor control |
| `control_motor_usb_l298n` | RPi USB serial motor control |
| `test_rpi_communication` | UART echo test with RPi |
| `validate_protocol` | Serial protocol validator |

## Flash to Hardware

```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --example <name> --target avr-none -Z build-std=core
```

## Design Notes

Non-obvious implementation decisions (type erasure, timer assignment,
stall thresholds, `no_std` test limitations) are documented in
[`docs/consideration_implementation.md`](docs/consideration_implementation.md).

Hardware pin mapping and peripheral timer assignments:
[`docs/the_pins_connections.md`](docs/the_pins_connections.md) —
[`docs/peripheral_timers.md`](docs/peripheral_timers.md)

RPi 5 ↔ Arduino Mega communication (why UART, wiring, protocol, RPi setup):
[`docs/rpi5_uart_communication.md`](docs/rpi5_uart_communication.md)

## Hardware References

- **ATmega2560 Datasheet** — [Microchip official](https://www.microchip.com/en-us/product/atmega2560) (download from product page)
- **Arduino Mega 2560 Pin Mapping** — [Arduino official docs](https://docs.arduino.cc/hacking/hardware/PinMapping2560)
- **HC-SR04 Datasheet** — [SparkFun](https://cdn.sparkfun.com/datasheets/Sensors/Proximity/HCSR04.pdf)
- **TF-Luna Datasheet** — [Benewake official](https://en.benewake.com/TF-Luna/index_proid_325.html)
