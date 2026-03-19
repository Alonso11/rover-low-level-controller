# Implementation Considerations
<!-- Version: v1.1 -->

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

| Timer | Pins (Arduino) | Motors assigned |
|---|---|---|
| Timer 1 | D11, D12, D13 | (available) |
| Timer 2 | D10, D9 | Front Right, Front Left |
| Timer 3 | D5, D2 | Center Right, Center Left |
| Timer 4 | D6, D7 | Rear Right, Rear Left |
| Timer 5 | D46, D45 | (available) |

Using separate timers per motor pair avoids contention on shared timer
registers and allows independent duty cycle control. Two motors sharing the
same timer can still have independent duty cycles (OC channel A vs B), but
share the same frequency and prescaler — which is acceptable since all
motors run at the same PWM frequency.

**Do not assign a motor to Timer 0** — it is used by `arduino_hal` for
`delay_ms`/`delay_us` and modifying it corrupts timing functions.

**Timer 1 is available for motors or other peripherals.** `StandardServo`
uses software PWM via `delay_us` on any digital output pin — it does not
depend on any hardware timer.

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

## 4. `no_std` and the Native Test Limitation

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
