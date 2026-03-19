<!-- Version: v1.1 -->
# ATmega2560 Hardware Timers

This document details the hardware timers available on the ATmega2560 for
PWM generation, with focus on motor control.

## 1. Overview

The ATmega2560 has **6 timers** (Timer0–Timer5):

| Timer | Bits | PWM Channels | Recommended Use |
|-------|------|--------------|-----------------|
| Timer0 | 8  | 2 | **System reserved** — `delay_ms` / `delay_us` |
| Timer1 | 16 | 3 | Available (D11, D12, D13) |
| Timer2 | 8  | 2 | **Motor 1 — Front** (D10, D9) |
| Timer3 | 16 | 3 | **Motor 2 — Center** (D5, D2, D3) |
| Timer4 | 16 | 3 | **Motor 3 — Rear** (D6, D7, D8) |
| Timer5 | 16 | 3 | Available (D46, D45, D44) |

**Total: 16 PWM channels available** (Timer0 excluded from motor use)

## 2. Pin Map by Timer

### Timer2 (8-bit, 2 channels) — Front Motors

| Channel | Arduino Pin | Port | OCR Register |
|---------|-------------|------|--------------|
| OC2A | D10 | PB4 | OCR2A |
| OC2B | D9  | PH6 | OCR2B |

### Timer3 (16-bit, 3 channels) — Center Motors

| Channel | Arduino Pin | Port | OCR Register | Note |
|---------|-------------|------|--------------|------|
| OC3A | D5 | PE3 | OCR3A | Safe |
| OC3B | D2 | PE4 | OCR3B | ⚠️ Shared with INT4 |
| OC3C | D3 | PE5 | OCR3C | ⚠️ Shared with INT5 |

### Timer4 (16-bit, 3 channels) — Rear Motors

| Channel | Arduino Pin | Port | OCR Register |
|---------|-------------|------|--------------|
| OC4A | D6 | PH3 | OCR4A |
| OC4B | D7 | PH4 | OCR4B |
| OC4C | D8 | PH5 | OCR4C |

### Timer1 (16-bit, 3 channels) — Available

| Channel | Arduino Pin | Port | OCR Register |
|---------|-------------|------|--------------|
| OC1A | D11 | PB5 | OCR1A |
| OC1B | D12 | PB6 | OCR1B |
| OC1C | D13 | PB7 | OCR1C |

### Timer5 (16-bit, 3 channels) — Available

| Channel | Arduino Pin | Port | OCR Register |
|---------|-------------|------|--------------|
| OC5A | D46 | PL3 | OCR5A |
| OC5B | D45 | PL4 | OCR5B |
| OC5C | D44 | PL5 | OCR5C |

## 3. Encoder Interrupt Pin Assignment

External interrupt lines used for Hall encoder pulse counting:

| Encoder | Arduino Pin | Interrupt | Conflict |
|---------|-------------|-----------|----------|
| Front Right  | D21 | INT0 (PD0) | None |
| Front Left   | D20 | INT1 (PD1) | None |
| Center Right | D19 | INT2 (PD2) | Shared with USART1 RX (RPi link) |
| Center Left  | D18 | INT3 (PD3) | Shared with USART1 TX (RPi link) |
| Rear Right   | D2  | INT4 (PE4) | Shared with Timer3 OC3B (motor PWM) |
| Rear Left    | D3  | INT5 (PE5) | None |

> Note: Arduino's `attachInterrupt()` uses a remapped numbering where
> interrupt number 0 maps to D2 (hardware INT4). This firmware uses
> register-level code, so the hardware numbers above apply directly.

## 4. PWM Frequency Calculation

```
f_PWM = f_CPU / (Prescaler × 256)    [for 8-bit timers in Fast PWM mode]
```

With `Prescale64` and `f_CPU = 16 MHz`:
```
f_PWM = 16,000,000 / (64 × 256) ≈ 976 Hz ≈ 1 kHz
```

Available prescalers in `arduino-hal`:
`Prescale1`, `Prescale8`, `Prescale64`, `Prescale256`, `Prescale1024`

## 5. Usage Rules

### One timer per L298N driver

Each L298N controls 2 motors and must use a **single timer** (both channels
of that timer). Two drivers must never share a timer to avoid duty-cycle
register conflicts.

### Motor configurations

| Config | Front | Center | Rear |
|--------|-------|--------|------|
| 2 motors (1 driver) | Timer2 (D10, D9) | — | — |
| 4 motors (2 drivers) | Timer2 (D10, D9) | Timer3 (D5, D2) | — |
| 6 motors (3 drivers) | Timer2 (D10, D9) | Timer3 (D5, D2) | Timer4 (D6, D7) |

### Do not use Timer0

Timer0 is used internally by `arduino-hal` for `delay_ms()` and `delay_us()`.
Repurposing it will corrupt all timing functions.

## 6. Initialization Example

```rust
use arduino_hal::simple_pwm::{Timer2Pwm, Timer3Pwm, Timer4Pwm, Prescaler};

let timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64); // Front
let timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64); // Center
let timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64); // Rear

// Front right: D10 (OC2A)
let fr_pwm = pins.d10.into_output().into_pwm(&timer2);
// Center right: D5 (OC3A)
let cr_pwm = pins.d5.into_output().into_pwm(&timer3);
// Rear right: D6 (OC4A)
let rr_pwm = pins.d6.into_output().into_pwm(&timer4);
```

## 7. Known Conflicts

| Conflict | Pins | Cause | Mitigation |
|---|---|---|---|
| Motor PWM vs encoder INT | D2 (PE4) | OC3B + INT4 same pin | Use OC3C (D3) for motor, or use PCINT for encoder |
| Motor PWM vs encoder INT | D3 (PE5) | OC3C + INT5 same pin | Only if OC3C is used; OC3B (D2) avoids this |
| UART RPi vs encoder INT | D18, D19 | USART1 + INT3/INT2 | Move RPi to Serial2 (D16/D17) or Serial3 (D14/D15) |
| Timer0 system use | D4, D13 | OC0B, OC0A reserved | Do not configure Timer0 for motor PWM |
