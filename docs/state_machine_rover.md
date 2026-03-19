<!-- Version: v1.1 -->
# Protection Logic and Safety State Machine (Design Specification)

> **Status: Design specification — not yet implemented in firmware.**
>
> The stall detection currently implemented in `src/controller/mod.rs` uses
> a simpler encoder-count based algorithm (see Section 4). The full I²t
> thermal model described in Sections 1–3 is planned for a future iteration.

This document describes the target safety architecture for protecting the
L298N drivers and DC motors using virtual sensors (mathematical models +
encoder feedback).

---

## 1. Virtual Current Sensor Model

Since the hardware has no physical current sensors (e.g. ACS712), current
is estimated from the DC motor back-EMF model:

### Estimation Formula

$$I_{est} = \frac{(V_{bat} \cdot \text{DutyCycle}) - (K_e \cdot \omega)}{R_{motor}}$$

| Parameter | Description |
| :--- | :--- |
| $V_{bat}$ | Battery voltage (read via analog pin or assumed nominal) |
| $K_e$ | Back-EMF constant (V·s/rad) — determined experimentally |
| $\omega$ | Actual angular velocity from encoder (rad/s) |
| $R_{motor}$ | Motor winding resistance (Ω) |

---

## 2. Virtual Thermal Fuse (I²t Algorithm)

Accumulated thermal energy $E_{term}$ protects against sustained overcurrent:

### Accumulation Logic (per control cycle, $\Delta t = 100\,\text{ms}$)

1. Compute $I_{est}$.
2. If $I_{est} > I_{nominal}$:
$$E_{term} = E_{term} + (I_{est}^2 - I_{nominal}^2) \cdot \Delta t$$
3. If $I_{est} \leq I_{nominal}$:
$$E_{term} = \max\left(0,\; E_{term} - C_{cool} \cdot \Delta t\right)$$

$E_{term}$ is clamped to 0 from below (no negative thermal energy).

---

## 3. Safety State Machine

| State | Condition | Action | Recovery |
| :--- | :--- | :--- | :--- |
| **NORMAL** | $E_{term} < 70\%$ | Full operation | — |
| **WARN** | $70\% \le E_{term} < 90\%$ | Send `HIGH_LOAD` log to RPi | $E_{term}$ decreases |
| **LIMIT** | $E_{term} \ge 90\%$ | Cap PWM at 40% | $E_{term} < 60\%$ |
| **FAULT_STALL** | PWM > 30% and RPM < 5 | Immediate stop | `RESET` command |
| **FAULT_OVERHEAT** | $E_{term} \ge 100\%$ | Immediate stop | Cool down + `RESET` |

---

## 4. Stall Detection — Currently Implemented

The firmware in `src/controller/mod.rs` uses a simpler model: if the encoder
count does not change for 50 consecutive `update()` calls while the commanded
speed is above 20%, the channel is declared stalled and stopped.

```
50 calls × 20 ms = 1 second of no encoder movement at |speed| > 20%
```

An emergency stop is triggered when 2 or more motors on the same side stall
simultaneously.

```rust
// Pseudocode of the current implementation
if speed.abs() > 20 && encoder_count == last_count {
    stall_timer += 1;
} else {
    stall_timer = 0;
}
stalled = stall_timer > 50;
```

---

## 5. Suggested Calibration Parameters (12V 100RPM Motor)

| Parameter | Value |
| :--- | :--- |
| $R_{motor}$ | ~2.5 Ω |
| $I_{nominal}$ | 0.8 A |
| $I_{max\,L298N}$ | 2.0 A |
| $K_e$ | Determine experimentally |
