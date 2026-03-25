<!-- Version: v1.3 -->
# ATmega2560 Hardware Timers

This document details the hardware timers available on the ATmega2560 for
PWM generation, with focus on motor control.

## 1. Overview

The ATmega2560 has **6 timers** (Timer0–Timer5):

| Timer | Bits | PWM Channels | Assigned Use |
|-------|------|--------------|--------------|
| Timer0 | 8  | 2 | **Sistema** — `delay_ms` / `delay_us` (no tocar) |
| Timer1 | 16 | 3 | **Servo** — D11 (OC1A) a 50Hz |
| Timer2 | 8  | 2 | **Motores Frontales** — D9 (OC2B), D10 (OC2A) |
| Timer3 | 16 | 3 | **Motor Central Derecho** — D5 (OC3A) |
| Timer4 | 16 | 3 | **Central Izq + Traseros** — D6 (OC4A), D7 (OC4B), D8 (OC4C) |
| Timer5 | 16 | 3 | **Libre** — D44 (OC5C), D45 (OC5B), D46 (OC5A) |

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

### Un timer por motor

Cada motor usa un canal PWM independiente de su timer. Dos motores pueden
compartir el mismo timer (p.ej. Center Right y Center Left en Timer3) siempre
que usen **canales distintos** (OC3A y OC3B), ya que cada canal tiene su
propio registro OCR. Dos motores **nunca deben compartir el mismo canal**.

### Motor configurations

| Config | Front Right | Front Left | Center R | Center L | Rear Right | Rear Left |
|--------|-------------|------------|----------|----------|------------|-----------|
| 2 motors | Timer2/D9 | Timer2/D10 | — | — | — | — |
| 4 motors | Timer2/D9 | Timer2/D10 | Timer3/D5 | Timer4/D6 | — | — |
| 6 motors | Timer2/D9 | Timer2/D10 | Timer3/D5 | Timer4/D6 | Timer4/D7 | Timer4/D8 |

### Do not use Timer0

Timer0 is used internally by `arduino-hal` for `delay_ms()` and `delay_us()`.
Repurposing it will corrupt all timing functions.

## 6. Initialization Example

```rust
use arduino_hal::simple_pwm::{Timer2Pwm, Timer3Pwm, Timer4Pwm, Prescaler};

// Timer1 reservado para servo (50Hz hardware PWM en D11)
let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64); // Motores frontales
let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64); // Motor central derecho
let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64); // Central izq + traseros

// Front right:   D9  (OC2B, Timer2, PH6)
let fr_pwm = pins.d9.into_output().into_pwm(&mut timer2);
// Front left:    D10 (OC2A, Timer2, PB4)
let fl_pwm = pins.d10.into_output().into_pwm(&mut timer2);
// Center right:  D5  (OC3A, Timer3, PE3)
let cr_pwm = pins.d5.into_output().into_pwm(&mut timer3);
// Center left:   D6  (OC4A, Timer4, PH3) — D2/INT4 libre para encoder
let cl_pwm = pins.d6.into_output().into_pwm(&mut timer4);
// Rear right:    D7  (OC4B, Timer4, PH4)
let rr_pwm = pins.d7.into_output().into_pwm(&mut timer4);
// Rear left:     D8  (OC4C, Timer4, PH5)
let rl_pwm = pins.d8.into_output().into_pwm(&mut timer4);
```

## 7. Known Conflicts

| Conflict | Pins | Cause | Mitigation |
|---|---|---|---|
| Motor PWM vs encoder INT | D2 (PE4) | OC3B + INT4 same pin | Use OC3C (D3) for motor, or use PCINT for encoder |
| Motor PWM vs encoder INT | D3 (PE5) | OC3C + INT5 same pin | Only if OC3C is used; OC3B (D2) avoids this |
| UART RPi vs encoder INT | D18, D19 | USART1 + INT3/INT2 | Move RPi to Serial2 (D16/D17) or Serial3 (D14/D15) |
| Timer0 system use | D4, D13 | OC0B, OC0A reserved | Do not configure Timer0 for motor PWM |
