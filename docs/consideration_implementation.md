# Implementation Considerations
<!-- Version: v2.1 -->

This document records non-obvious design decisions, constraints, and the
reasoning behind them. It is intended as a reference for future maintainers.

---

## 1. Type Erasure for Motor Arrays (`ErasedMotor`)

### The Problem

`RoverController` needs to store 6 motors in a homogeneous array
`[DriveChannel<M, E>; 6]`. Rust requires every element of an array to have
the **exact same concrete type**. This is straightforward for the encoder
(`HallEncoder` is always the same type), but becomes a hard constraint for
motors.

In `avr-hal`, the L298N motor driver is typed as:

```rust
L298NMotor<TC, PwmPin, In1Pin, In2Pin>
```

The direction pins (`In1Pin`, `In2Pin`) can be type-erased using
`.downgrade()`, which converts them to the `Dynamic` marker type. However,
**the PWM pin cannot be erased** because `avr-hal`'s `simple_pwm` module
bakes the hardware Timer Counter type (`TC`) and the specific pin type
(`PwmPin`) into the type signature. The duty-cycle register address is
resolved at compile time through the `PwmPinOps<TC>` trait.

This means each motor on a different timer or pin produces a distinct
concrete type:

```
m0 → L298NMotor<Timer2Pwm, PB4, Dynamic, Dynamic>
m1 → L298NMotor<Timer2Pwm, PH6, Dynamic, Dynamic>   // same timer, different pin
m2 → L298NMotor<Timer3Pwm, PE3, Dynamic, Dynamic>   // different timer
m3 → L298NMotor<Timer3Pwm, PE4, Dynamic, Dynamic>
m4 → L298NMotor<Timer4Pwm, PH3, Dynamic, Dynamic>
m5 → L298NMotor<Timer4Pwm, PH4, Dynamic, Dynamic>
```

Storing these in a `[DriveChannel<M, E>; 6]` with a single `M` is
impossible without type erasure.

### Why `dyn Motor` is Not Viable Here

The idiomatic Rust solution for heterogeneous collections is trait objects
(`&dyn Motor` or `Box<dyn Motor>`). Neither is available here:

- `Box<dyn Motor>` requires heap allocation (`alloc`), which is not
  available on AVR targets compiled with `#![no_std]`.
- `&mut dyn Motor` references require careful lifetime management and cannot
  be stored in a struct without explicit lifetimes tied to the original
  motors, which complicates the `RoverController` API significantly.
- `&'static mut dyn Motor` would work, but forces the user to declare each
  motor as a `static mut` variable — an unergonomic and `unsafe` pattern
  that pollutes the application code.

### Why 6 Separate Generics is Not Viable

`SixWheelRover<M1, M2, M3, M4, M5, M6>` already uses this pattern for
simple differential drive. It works, but the `RoverController` requires
iterating over all channels in a loop (for stall detection). With 6 separate
generic types there is no way to express "call `check_stall` on each channel"
without manually repeating the call 6 times, which is fragile and defeats
the purpose of the array-based design.

### Chosen Solution: Manual Vtable via Function Pointers (`ErasedMotor`)

`ErasedMotor` implements type erasure manually using two ingredients:

1. A `*mut ()` raw pointer to the concrete motor on the stack.
2. Two function pointers (`set_speed_fn`, `stop_fn`) that are
   monomorphized at the call site and cast to a type-erased signature.

```rust
pub struct ErasedMotor {
    data: *mut (),
    set_speed_fn: unsafe fn(*mut (), i16),
    stop_fn:      unsafe fn(*mut ()),
}
```

The monomorphized shims are regular `unsafe fn` items (not closures), so
they can be coerced to function pointers without capturing any state:

```rust
unsafe fn set_speed_impl<M: Motor>(ptr: *mut (), speed: i16) {
    (*(ptr as *mut M)).set_speed(speed);
}
```

This is structurally identical to what the Rust compiler generates for
`dyn Trait` vtables, but done manually to avoid the heap requirement.

`ErasedMotor` implements the `Motor` trait, giving it a fixed, known size
and making it storable in a `[DriveChannel<ErasedMotor, E>; 6]` array.

### Safety Invariant

`ErasedMotor::new` is marked `unsafe`. The caller must guarantee that the
concrete motor outlives the `ErasedMotor`. In practice, both motors and the
`RoverController` are stack-allocated inside `main() -> !`, which never
returns, so the stack frame is permanent and the lifetime invariant is
trivially satisfied.

The `unsafe` surface is limited to:
- `ErasedMotor::new()` — where the raw pointer is created.
- The two dispatch shims — where it is dereferenced.

All call sites in `Motor` trait impl use safe wrappers that encapsulate the
`unsafe` blocks.

### Trade-offs vs Other Options

| Approach | Heap | `unsafe` | Loop-based stall | Ergonomics |
|---|---|---|---|---|
| `[M; 6]` single generic | No | No | Yes | Fails: all types must match |
| `<M0..M5>` six generics | No | No | No (manual repeat) | Poor |
| `Box<dyn Motor>` | Yes | No | Yes | Not available in `no_std` |
| `&'static mut dyn Motor` | No | Yes | Yes | Ugly call site |
| **`ErasedMotor` (chosen)** | **No** | **Isolated** | **Yes** | **Good** |

---

## 2. PWM Timer Assignment for 6 Motors

Each L298N motor requires one PWM-capable pin. On the ATmega2560, PWM pins
are grouped by Timer Counter:

| Timer | Pins (Arduino) | Assigned use |
|---|---|---|
| Timer 0 | D4, D13 | **Sistema** — `delay_ms`/`delay_us` (no tocar) |
| Timer 1 | D11, D12, D13 | **Servo** — D11 (OC1A) a 50Hz |
| Timer 2 | D9, D10 | Front Right (OC2B), Front Left (OC2A) |
| Timer 3 | D5, D2, D3 | Center Right (OC3A) — D2/D3 libres para INT4/INT5 |
| Timer 4 | D6, D7, D8 | Center Left (OC4A), Rear Right (OC4B), Rear Left (OC4C) |
| Timer 5 | D46, D45, D44 | Libre — disponible para expansión |

Two motors can share a timer if they use different OC channels (e.g. Timer4
drives three motors on OC4A, OC4B, OC4C independently). They share the same
prescaler and frequency, which is acceptable since all motors run at the same
PWM frequency (~1 kHz with Prescale64).

**Do not assign a motor to Timer 0** — it is used by `arduino_hal` for
`delay_ms`/`delay_us` and modifying it corrupts timing functions.

**Timer 1 is reserved for the servo.** Hardware Timer1 (16-bit) generates
an exact 50Hz signal for the servo (Prescale8, TOP=39999). Do not assign
motors to D11, D12, or D13.

---

## 3. Stall Detection Threshold

`DriveChannel::check_stall` triggers a stall when the encoder count has not
changed for more than 50 consecutive calls to `update()`, while the
commanded speed is above 20% (|speed| > 20).

```
50 calls × 20 ms per call = 1 second of no encoder movement at speed > 20%
```

`RoverController::update` declares an emergency stop when **2 or more motors
on the same side stall simultaneously**. A single motor stalling alone (e.g.,
one wheel hitting a rock briefly) does not trigger the emergency — only
sustained bilateral blockage does.

These thresholds were chosen conservatively for a rocky-terrain rover. They
may need tuning based on the actual motor response time and the update loop
frequency.

---

## 4. Sensor Architecture — Layered Safety Model

### Context

The rover has three sensing modalities physically attached to the Arduino:
- **HC-SR04** (ultrasonic, D38/D39) — short range, ~2–400 cm, ±3 cm accuracy
- **VL53L0X** (ToF I2C, D42/D43 soft I2C) — short range, 3–200 cm, ±3% accuracy
- **Hall encoders** (×6, INT0–INT5) — motor shaft rotation, stall detection

En addition, the RPi5 carries a **camera** used for AI-based navigation.

> TF-Luna (USART2, D16/D17) fue descartado por no estar disponible en inventario.
> El driver `src/sensors/tf_luna.rs` se mantiene para referencia. Ver
> `docs/decision-log.md` §Semana 4 — Cambio de sensor TF-Luna → VL53L0X.

### The Question

Should the Arduino read its own sensors and act on them, or should it just
report raw data to the RPi5 and let the RPi5 make all decisions?

### Why Pure-Slave Architecture Is Insufficient

If the Arduino only executes commands from the RPi5 (no local sensor
processing), then the safety response chain is:

```
obstacle appears
  → camera frame captured (~33 ms at 30 fps)
  → AI inference (~50–150 ms)
  → RPi5 sends AVD over UART (~1 ms)
  → Arduino receives and acts (~20 ms loop)
  ─────────────────────────────────────────
  Total latency: ~100–200 ms
```

At 0.5 m/s the rover travels 5–10 cm before stopping. For a close obstacle
(< 30 cm) this is not safe. Furthermore, if the UART link drops or the RPi5
crashes, the rover keeps moving indefinitely.

### Chosen Architecture: Three Independent Safety Layers

Each layer operates at its own frequency and has authority to override
lower-priority layers:

```
Layer 3 — RPi5 + Camera (strategic, ~100 ms)
  Purpose : path planning, navigation, AI decisions
  Action  : sends EXP:L:R, AVD:L/R, STB commands
  Scope   : everything the camera can see

Layer 2 — VL53L0X on Arduino (tactical, ~20 ms loop, soft I2C D42/D43)
  Purpose : detect obstacles the camera may miss (low objects, dust)
  Action  : if dist < 150 mm while EXPLORE → FAULT
  Scope   : forward arc, 3–200 cm

Layer 1 — HC-SR04 on Arduino (emergency, ~20 ms loop, D38/D39)
  Purpose : last-resort physical barrier protection
  Action  : if dist < 200 mm → FAULT (hard stop, motors off)
  Scope   : forward arc, < 30 cm
  Authority: overrides RPi5 commands — cannot be suppressed remotely
```

### Why the Arduino Must Act Locally for Safety

The SRS defines `Task_Safety` as the **highest-priority task on Nodo B**.
This means the Arduino must be capable of stopping the rover independently
of the RPi5. Reasons:

1. **UART link can fail** — watchdog handles communication loss, but sensor
   data in transit can arrive late or not at all.
2. **Camera has blind spots** — objects below the camera frame, dust clouds,
   or sudden close-range appearances after a maneuver are invisible to AI.
3. **RPi5 processing latency** — ~100–200 ms end-to-end is too slow for
   emergency stops at close range.
4. **Fail-safe by design** — if the RPi5 crashes, the Arduino should still
   protect the hardware. A pure-slave design violates this principle.

### Telemetry Extension

To allow the RPi5 to fuse sensor data with camera data, the Arduino will
extend the TLM frame to include sensor readings:

```
Current : TLM:<SAFETY>:<STALL_MASK>\n
Extended: TLM:<SAFETY>:<STALL_MASK>:<HC_CM>:<TF_CM>\n
Example : TLM:NORMAL:000000:038:0120\n
                              ^     ^
                              HC-SR04 (38 cm)
                                    TF-Luna (120 cm)
```

This lets the RPi5 use Arduino sensor data as a secondary input to its
navigation model without giving up local safety authority.

### Responsibility Matrix

| Sensor | Node | Decision | Max latency |
|---|---|---|---|
| Camera | RPi5 | Navigation, path planning | ~150 ms |
| VL53L0X (D42/D43) | Arduino | Tactical avoidance < 150 mm → FAULT | ~20 ms |
| HC-SR04 (D38/D39) | Arduino | Emergency hard stop < 200 mm → FAULT | ~20 ms |
| Encoders (INT0–INT5) | Arduino | Stall → FAULT | ~20 ms |

### Current Implementation Status (v2.10)

As of firmware v2.10, all local safety layers are active:

- **Layer 1 — HC-SR04**: active, reads every 5 cycles (~100 ms), FAULT if < 200 mm.
  Driver v1.2: `with_timeout(1_750 µs)` limits blocking to ~1.75 ms; returns `Result`.
- **Layer 2 — VL53L0X**: active, soft I2C D42/D43, FAULT si distancia < 150 mm.
  Driver v1.1: `read_mm()` returns `Result<u16, SensorError>`.
  Distancia reportada en campo `DIST` del frame TLM.
- **Layer 3 — RPi5**: active via USART0 (USB) at 115200, full MSM protocol.
  Cambio a USART3 (D14/D15) para producción pendiente de flash.
- **Stall detection**: active, 6 encoders via INT0–INT5, FAULT if |speed|>20%
  and encoder frozen for >50 cycles (~1 s).
- **Overcurrent detection**: active, 6× ACS712-30A on A0–A5, two-tier graduated
  response (see §8 for full design). Thresholds: Warn ≥1200 mA, Limit ≥1600 mA,
  Fault ≥2000 mA.
- **Battery temperature**: active, 6× NTC en A7–A12, Warn >45°C, Limit >55°C, Fault >65°C.
- **Timestamp TLM**: campo `tick_ms` (u32, ms desde arranque) en cada frame TLM.
- **Voltage monitoring**: active, INA226 en bus soft I2C D42/D43 (dirección 0x40,
  compartido con VL53L0X 0x29). Campos `batt_mv` y `batt_ma` en TLM.
  Requiere shunt externo 10 mΩ / 5W en serie con la carga.

El frame TLM incluye corrientes, temperatura ambiente, temperaturas de celda,
distancia frontal, tensión y corriente de batería. Ver §9 para el formato completo.

---

## 5. HC-SR04 Polling vs Interrupt-Based Measurement

### Current Implementation (v1.2) — Timeout + Result API

The HC-SR04 driver (`sensors/hc_sr04.rs`) uses busy-wait loops to measure
echo pulse duration:

```rust
while self.echo.is_low()  { count += 1; if count > 30_000 { return Err(SensorError::Timeout); } }
while self.echo.is_high() { duration_us += 1; delay_us(1); if duration_us > self.echo_timeout_us { return Err(SensorError::Timeout); } }
```

Without a limit, an obstacle at ~4 m causes ~30 ms of blocking, exceeding
the 20 ms main loop period.

**Two mitigations applied:**

1. **`with_timeout(echo_timeout_us)`** — builder that limits the echo-wait
   loop to the range that matters for safety. The firmware configures
   `HC_ECHO_TIMEOUT_US = 1_750` µs (`src/config.rs`), corresponding to
   ~300 mm (1.5× the emergency threshold). This reduces max blocking from
   ~30 ms to ~1.75 ms.

2. **`HC_READ_PERIOD = 5`** — the sensor is read every 5 cycles (~100 ms)
   instead of every cycle, since each measurement still adds latency.
   At 0.5 m/s the rover travels ~5 cm between readings — acceptable for
   the 200 mm emergency threshold.

**Result API**: `measure_mm()` returns `Result<u16, SensorError>`:
- `Ok(mm)` — valid distance.
- `Err(Timeout)` — echo did not arrive within the configured window (object
  out of range — not an emergency).
- `Err(OutOfRange)` — distance is outside the HC-SR04's 2–4000 mm spec.

The previous `last_valid` / `consecutive_errors` caching was removed.
It was silently masking read failures and making stale data invisible to
the caller. The caller (`main.rs`) already handles transient errors by
simply skipping the FAULT trigger if no `Ok` is received.

### Future Migration: Interrupt-Based Measurement

The correct long-term solution is to measure the echo pulse using a hardware
external interrupt:

1. **Trigger** the pulse from the main loop (10 µs HIGH on D38).
2. **Rising edge ISR** on D39 (INT6/PE6 — check pin availability): record
   `start = current_timer_ticks`.
3. **Falling edge ISR** on D39: compute `duration = current_timer_ticks - start`,
   store result in a `Mutex<Cell<Option<u16>>>`.
4. **Main loop** reads the stored value without blocking.

This approach would:
- Eliminate blocking from the main loop entirely
- Allow reading every cycle (20 ms) instead of every 100 ms
- Free CPU cycles during echo wait

**Prerequisite**: verify D39 (PG2) supports external interrupts on the
ATmega2560. PG2 is not on a standard INTn pin — it may require using
Pin Change Interrupts (PCINT) instead, which add complexity. Check the
ATmega2560 datasheet (Table 13-1) before implementing.

**Also note**: the encoder ISRs already use `avr_device::interrupt::Mutex<Cell<T>>`
for the same pattern. The HC-SR04 interrupt implementation should follow that
same structure.

---

## 6. Encoder Integration — Static ISR Pattern

### Why `static` for HallEncoders

AVR interrupt service routines (ISRs) are global functions with no arguments.
They cannot capture local variables. The only way to share state between an
ISR and the main loop is through `static` variables.

`HallEncoder` contains `Mutex<Cell<i32>>` from `avr_device::interrupt`.
`avr_device::Mutex<T>` is `Sync` for any `T` on AVR (single-core, so mutual
exclusion is achieved by disabling interrupts, not with hardware locks). This
makes `HallEncoder` safe to declare as `static`.

```rust
static ENCODER_FR: HallEncoder = HallEncoder::new(); // requires const fn new()
```

`HallEncoder::new()` is `const fn`, so this is zero-cost — the struct is
placed in `.bss` / `.data` at link time, no runtime initialization.

### ISR Declaration

avr-device requires the chip name in **lowercase** in the interrupt attribute.
Using `ATmega2560` (PascalCase) causes a compile error even though the PAC
enum uses `ATmega2560`:

```rust
// ✓ correct
#[avr_device::interrupt(atmega2560)]
fn INT0() { ENCODER_FR.pulse(); }

// ✗ wrong — compile_error: Couldn't find interrupt INT0, for MCU ATmega2560
#[avr_device::interrupt(ATmega2560)]
fn INT0() { ENCODER_FR.pulse(); }
```

### Why USART3 Was Critical for 6 Encoders

The original firmware used USART1 (D18/D19) for RPi5 communication.
This blocked INT2 (D19) and INT3 (D18), leaving only 4 encoder slots
(INT0, INT1, INT4, INT5 = front and rear wheels).

Moving to USART3 (D14/D15) freed D18 and D19 for encoders, enabling
full 6-motor stall detection without hardware conflicts.

| INT   | Pin | Encoder       | Available with USART1 | Available with USART3 |
|-------|-----|---------------|-----------------------|-----------------------|
| INT0  | D21 | Front Right   | ✅                    | ✅                    |
| INT1  | D20 | Front Left    | ✅                    | ✅                    |
| INT2  | D19 | Center Right  | ❌ (USART1 RX)        | ✅                    |
| INT3  | D18 | Center Left   | ❌ (USART1 TX)        | ✅                    |
| INT4  | D2  | Rear Right    | ✅                    | ✅                    |
| INT5  | D3  | Rear Left     | ✅                    | ✅                    |

### Stall Detection Algorithm

Each cycle (~20 ms) the loop reads all 6 encoder counters and compares
with the previous cycle. If a motor runs above `STALL_SPEED_MIN` (20%)
and its encoder count does not change, a per-motor `stall_timer` increments.
When `stall_timer > STALL_THRESHOLD` (50 cycles = ~1 s), the bit for that
motor is set in `stall_mask`, and `msm.update_safety(stall_mask)` is called.

This mirrors the logic in `DriveChannel::check_stall` (controller/mod.rs)
but operates directly in `main.rs` to avoid the `ErasedMotor` complexity
that `RoverController` would require. See §1 for context.

### Hardware Register Setup (EICRA / EICRB / EIMSK)

The PAC exposes EXINT registers via getter methods (not public fields):

```rust
dp.EXINT.eicra().write(|w| unsafe { w.bits(0xFF) }); // INT0–INT3 rising edge
dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x0F) }); // INT4–INT5 rising edge
dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) }); // enable INT0–INT5
unsafe { avr_device::interrupt::enable() };
```

`0xFF` in EICRA sets ISCn1=1, ISCn0=1 for each of INT0–INT3 (rising edge).
`0x0F` in EICRB sets ISC4=rising, ISC5=rising (bits 1:0 and 3:2).
`0x3F` = 0b00111111 enables INT0 through INT5 in EIMSK.

### Reducing to Fewer Than 6 Encoders

If fewer encoders are physically installed, remove or comment the
corresponding `static`, ISR, and speed entry. The `stall_mask` bit for
that motor will never be set (counter stays at 0, speed check fails).
No other code needs to change.

---

## 8. ADC Multiplexado, Promediado y Protección de Corriente — ACS712 + LM335

### Hardware ADC del ATmega2560

El ATmega2560 dispone de **un único ADC SAR de 10 bits** con un multiplexor
analógico de 16 canales (A0–A15, registros ADMUX/ADCSRA). Solo puede realizar
**una conversión a la vez**; los 16 canales se leen secuencialmente.

Cada conversión toma **13 ciclos del reloj ADC**. Con el prescaler por defecto
de `arduino_hal` (128 → 125 kHz de reloj ADC):

```
T_conversion = 13 / 125_000 Hz ≈ 104 µs por canal
```

### Canales usados

| Canal | Pin  | Sensor        | Función                |
|-------|------|---------------|------------------------|
| ADC0  | A0   | ACS712 FR     | Corriente motor Front Right  |
| ADC1  | A1   | ACS712 FL     | Corriente motor Front Left   |
| ADC2  | A2   | ACS712 CR     | Corriente motor Center Right |
| ADC3  | A3   | ACS712 CL     | Corriente motor Center Left  |
| ADC4  | A4   | ACS712 RR     | Corriente motor Rear Right   |
| ADC5  | A5   | ACS712 RL     | Corriente motor Rear Left    |
| ADC6  | A6   | LM335         | Temperatura ambiente         |

### Por qué se promedia (macro `adc_avg!`)

Una sola lectura ADC tiene ruido de ±2–3 LSB (~±10–15 mV). Para el ACS712
(66 mV/A) esto equivale a **±150–225 mA** de ruido por muestra — suficiente
para causar falsos positivos en la detección de sobrecorriente.

Con promedio de N muestras, el ruido se reduce por √N:

```
N=1:  σ ≈ ±225 mA
N=8:  σ ≈ ±225 / √8 ≈ ±80 mA   ← implementado (SEN_SAMPLES = 8)
N=16: σ ≈ ±225 / √16 ≈ ±56 mA
```

Para el LM335 (10 mV/K), N=8 da σ ≈ **±0.5 °C** frente a ±1.5 °C sin promediado.

### Implementación — Macro `adc_avg!`

La macro `adc_avg!` en `main.rs` centraliza el patrón:

```rust
macro_rules! adc_avg {
    ($pin:expr, $adc:expr, $n:expr) => {{
        let mut sum = 0u32;
        for _ in 0..$n { sum += $pin.analog_read(&mut $adc) as u32; }
        (sum / $n as u32) as u16
    }};
}
```

### Protección de corriente graduada — Diseño de dos tiers

El L298N soporta **2 A en continuo** y **3 A de pico** por canal. Un único umbral
de fault a 500 ms dejaba el chip desprotegido durante medio segundo ante un
bloqueo mecánico o cortocircuito.

La solución adopta dos velocidades de muestreo independientes:

```
Fast tier  — cada 3 ciclos (~60 ms), 2 muestras/canal
             Solo detecta Fault (≥ OVERCURRENT_FAULT_MA)
             ~1.25 ms bloqueantes por ejecución

Slow tier  — cada 25 ciclos (~500 ms), 8 muestras/canal
             Clasifica Warn / Limit / Fault + actualiza sensor_frame para TLM
             ~5.8 ms bloqueantes por ejecución
```

**Umbrales vs rating del L298N:**

| Constante | Valor | Fracción de 2A | Acción |
|---|---|---|---|
| `OVERCURRENT_WARN_MA`  | 1200 mA | 60 % | Notifica vía TLM, rover sigue |
| `OVERCURRENT_LIMIT_MA` | 1600 mA | 80 % | Recorta velocidad a `LIMIT_SPEED_CAP` (60 %) |
| `OVERCURRENT_FAULT_MA` | 2000 mA | 100 % | Para todo, espera `RST` |

**Por qué umbrales distintos en cada tier:**
- Fault necesita reacción rápida pero baja precisión — 2 muestras son suficientes
  para detectar una corriente claramente fuera de rango.
- Warn/Limit son estados sostenidos — las 8 muestras promediadas evitan falsos
  positivos causados por el ruido ADC (±80 mA con N=8, ver tabla arriba).

**El macro `sync_drive!` aplica el cap de velocidad automáticamente:**

```rust
// Fragmento del macro sync_drive!
_ => {
    let (l, r) = if msm.safety == SafetyState::Limit {
        (drive.left.clamp(-LIMIT_SPEED_CAP, LIMIT_SPEED_CAP), ...)
    } else {
        (drive.left, drive.right)
    };
    rover.set_speeds(l, r);
}
```

Esto garantiza que el cap se aplica en todos los puntos del loop donde se
llama `sync_drive!`, sin código duplicado.

**Limitación importante:** El firmware no puede reemplazar protección hardware.
A 60 ms de latencia, un cortocircuito real puede dañar el L298N antes de que el
firmware actúe. La protección primaria debe ser un **polyfuse de 2 A** en la
línea de alimentación de cada driver. El firmware actúa como segunda línea de
defensa para condiciones de sobrecarga sostenida.

**Tiempos totales bloqueantes:**

```
Fast tier: 6 canales × 2 muestras × 104 µs = ~1.25 ms  cada ~60 ms  (2.1 % CPU)
Slow tier: 7 canales × 8 muestras × 104 µs = ~5.8 ms   cada ~500 ms (1.2 % CPU)
```

### Diseño HAL-independiente de los drivers

Los drivers `ACS712` y `LM335` reciben el valor ADC crudo (`u16`) en lugar
de tomar `&mut Adc` directamente. Esto:

1. Permite testearlos en x86 sin HAL (igual que la MSM).
2. Desacopla la conversión matemática del hardware de adquisición.
3. Mantiene el código de manejo del ADC en un solo lugar (`main.rs`).

---

## 9. Telemetría — Diseño del Frame TLM

### Contexto

El rover genera los siguientes datos de estado que la RPi5 necesita recibir:

| Dato | Fuente | Inclusión en TLM |
|------|--------|-----------------|
| Safety + stall mask | MSM | Campos 1–2 |
| Timestamp Arduino | main loop (`elapsed_ms`) | Campo 3 |
| Corriente 6 motores | ACS712 A0–A5 (slow tier, 500 ms) | Campos 4–9 |
| Temperatura ambiente | LM335 A6 | Campo 10 |
| Temperatura 6 celdas batería | NTC A7–A12 | Campos 11–16 |
| Distancia frontal | VL53L0X D42/D43 | Campo 17 |

### Opciones evaluadas

**Opción A — TLM extendido único (elegida)**
```
TLM:<SAFETY>:<STALL>:<TS>ms:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C:<B0>:...<B5>C:<DIST>mm\n
```
- Un solo frame periódico, fácil de parsear en RPi5
- Requiere `RESP_BUF` de 160 bytes (longitud máxima del frame TLM extendido ~130 bytes + margen)
- La `SensorFrame` vive en `state_machine` para mantenerse testeable en x86

**Opción B — SEN compacto separado**
```
TLM:NORMAL:000000\n
SEN:1200:980:1100:1050:1200:1180:27\n
```
- Mínimo cambio: solo compactar los 7 valores SEN en un frame
- Dos flujos — RPi5 debe correlacionarlos por tiempo

**Opción C — Frame extendido en main.rs sin modificar la MSM**
- Construir el frame directamente en `main.rs` ignorando `format_response`
- Más flexible pero rompe la separación de responsabilidades

### Por qué Opción A

La RPi5 recibe un único frame con todo el estado del LLC. No necesita
correlacionar múltiples streams. El parser en `rover_bridge` solo implementa
un tipo de frame de telemetría.

La `SensorFrame` se mantiene en el módulo `state_machine` para seguir el mismo
patrón de los tests nativos x86 ya existentes.

### Estructura `SensorFrame`

```rust
pub struct SensorFrame {
    pub tick_ms: u32,           // ms desde arranque (monotónico, wrapping ~49 días)
    pub batt_mv: u16,           // tensión bus batería en mV (INA226). 0 = sin lectura
    pub batt_ma: i32,           // corriente total batería en mA (INA226). 0 = sin lectura
    pub currents: [i32; 6],     // mA por motor [FR, FL, CR, CL, RR, RL]
    pub temp_c: i32,            // temperatura ambiente en °C
    pub batt_temps: [i32; 6],   // temperaturas celdas batería en °C [B0–B5]
    pub dist_mm: u32,           // distancia VL53L0X en mm (0 = sin lectura)
}
```

### Formato final (v2.12)

```
TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C:<B0>:<B1>:<B2>:<B3>:<B4>:<B5>C:<DIST>mm:<EL>:<ER>\n
```

Ejemplo:
```
TLM:NORMAL:000000:1000ms:14800mV:1200mA:1150:980:1100:1050:1200:1180:27C:28:29:28:30:29:28C:342mm:60:62
```

Longitud máxima estimada: ~185 bytes → `RESP_BUF = 200`.
Los campos EL/ER se añadieron en v2.12 para odometría (ver §10).

### Decisión: timestamp relativo (v2.7)

Se añadió el campo `tick_ms` (tercer campo) para cumplir SRS-020 (trazabilidad de misión).
El contador `elapsed_ms: u32` en `main.rs` usa `wrapping_add(LOOP_MS)` cada ciclo.
Overflow a ~49 días — dentro del margen de la misión. Ver `decision-log.md` §Semana 4.

### Decisión: INA226 en bus soft I2C compartido (v2.8)

El INA226 (0x40) comparte el bus soft I2C D42/D43 con el VL53L0X (0x29).
Las direcciones no colisionan. La lectura INA226 ocurre en el slow tier (~500 ms),
intercalada con las lecturas ACS712/LM335/NTC. Ver `docs/vl53l0x.md` §Bus compartido.

---

## 7. `no_std` and the Native Test Limitation

`cargo test --target x86_64-unknown-linux-gnu` fails because `.cargo/config.toml`
sets `build-std = ["core"]` globally. This causes a `core` symbol duplication
when the host target tries to use its own precompiled `core`.

The project's embedded HAL (`arduino_hal`) is AVR-specific and cannot be
compiled for x86. This means **unit tests that exercise HAL-dependent code
cannot run on a developer machine**. Only pure-logic tests (like
`tests/motor_logic_test.rs`) are portable, and they must be kept free of
any `use arduino_hal::...` imports.

Future work: consider a `--cfg test` feature flag to swap HAL types with
stub implementations for host-side testing.

---

## 10. Odometría diferencial — Diseño de la extensión TLM y OdometryTracker

### Contexto

RNF-003 requiere precisión de navegación < 5 % del trayecto recorrido. El rover
dispone de encoders Hall en los seis motores (ya integrados para stall detection),
lo que hace viable la odometría diferencial sin sensor adicional.

### Dónde integrar la cinemática: LLC vs. HLC

La cinemática diferencial requiere `sin` y `cos` para actualizar la pose. Esto
implica:

- **AVR (ATmega2560, `no_std`)**: sin `libm`. Añadir `avr-libm` o una crate de
  punto fijo aumenta el tamaño de flash y añade latencia al ciclo de 20 ms, que
  ya tiene restricciones por ADC y sensores. La cinemática sería implementable
  pero a un coste injustificado.
- **RPi5 (Python)**: `math.sin`/`math.cos` disponibles en stdlib. El estado de
  pose (`x_mm, y_mm, theta_rad`) puede persistir entre frames TLM sin
  restricciones de memoria. La latencia de ~1 s entre frames TLM es más que
  suficiente para el cómputo Python.

**Decisión**: el LLC acumula conteos brutos y los incluye en el frame TLM. El
HLC integra la cinemática en `OdometryTracker`.

### Por qué acumuladores de lado y no por rueda

Transmitir los seis conteos individuales habría añadido seis campos de hasta
11 dígitos (≈ 72 bytes extra), excediendo el `RESP_BUF` de 160 bytes. El modelo
diferencial solo necesita la distancia media de cada lado; la suma FL+CL+RL
provee exactamente eso con dos campos en vez de seis.

El denominador en la cinemática (`3 * TICKS_PER_REV`) absorbe el factor de la
suma: no hay pérdida de información.

### Por qué extender el frame TLM y no crear un mensaje nuevo

Ver §9. Los mismos argumentos aplican: un único stream de telemetría simplifica
el parser del HLC. Añadir al final preserva la compatibilidad: los parsers
existentes que leen los 20 primeros campos no necesitan cambios hasta que
activen la lectura de odometría.

### Por qué TICKS_PER_REV es TBD

El motor NFP-5840-31ZY-EN no documenta ni el PPR del encoder Hall ni la relación
exacta de la reductora. El procedimiento de calibración es:

```
1. Elevar el rover con las ruedas en el aire.
2. Girar una rueda una vuelta completa a mano.
3. Leer ENCODER_Fn.get_counts() antes y después.
4. Repetir 5 veces y promediar.
5. Actualizar TICKS_PER_REV en config.rs y TICKS_PER_REV en olympus_hlc/config.py.
   Ambos deben ser idénticos (el LLC no usa la constante para calcular nada;
   solo el HLC la usa en mm_per_tick).
```

### Comportamiento del acumulador ante overflow

Los conteos del encoder son `AtomicI32` (en `sensors/encoder.rs`), que hacen
`wrapping_add(1)` en cada pulso. El campo `enc_left`/`enc_right` en `SensorFrame`
es `i32` y se calcula como la suma de tres contadores con `wrapping_add`. En el
HLC, los deltas se calculan como `enc_left_new - enc_left_old` sobre `i32` en
Python; si hay overflow, el delta será incorrecto exactamente una vez (en el
ciclo donde ocurra el wrap). Para misiones de duración típica (< 2 h) el
wrap no es probable a velocidades de exploración normales, pero es una
limitación conocida.

### Tests

Los tests de `state_machine_test.rs` que construyen `SensorFrame` directamente
y verifican el formato ASCII del frame TLM fueron actualizados para incluir
`enc_left: 0, enc_right: 0` y el sufijo `:0:0` en los strings esperados.
138 tests en 3 suites, 0 fallos.
