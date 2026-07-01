# Debug: USART RX Overflow — Hallazgos y Soluciones (26 mar 2026)

## Contexto

Sesión de verificación del protocolo MSM desde PC (Ubuntu) → Arduino Mega 2560
via USB (USART0). Firmware: `feature/msm-main-integration` v2.2.

---

## Problema 1 — `avr-atmega2560.json`: campos incompatibles con nightly actual

### Síntoma

```
error: error loading target specification: version: unknown field `version`
```

### Causa

El campo `"version": "1.1"` en `avr-atmega2560.json` fue eliminado del schema
de target specs en versiones recientes del compilador nightly.

### Fix aplicado

Eliminar el campo `"version"` del JSON.

---

## Problema 2 — Linker AVR no soporta `--eh-frame-hdr`

### Síntoma

```
/usr/lib/gcc/avr/7.3.0/../../../avr/bin/ld: unrecognized option '--eh-frame-hdr'
```

### Causa

El nightly actual pasa `--eh-frame-hdr` al linker por defecto. El linker AVR
(`avr-ld`) no implementa esta opción.

### Fix aplicado

Agregar `"eh-frame-header": false` en `avr-atmega2560.json`.

---

## Problema 3 — HC-SR04 bloqueante satura el FIFO USART

### Síntoma

Todos los comandos retornaban `ERR:UNKNOWN` aunque el firmware respondía.

### Causa

El driver `HCSR04::measure_mm()` usa busy-wait bloqueante:

```rust
// Espera a que el echo suba (timeout = 20000 iteraciones sin delay)
while self.echo.is_low() {
    count += 1;
    if count > 20000 { return None; }
}
// Espera a que el echo baje (timeout = 30 ms)
while self.echo.is_high() {
    duration_us += 1;
    arduino_hal::delay_us(1);
    if duration_us > 30000 { return None; }
}
```

Con el HC-SR04 **desconectado**, el pin echo flota. Dependiendo del estado
eléctrico del pin:
- Si flota HIGH: el segundo `while` espera hasta el timeout de **30 ms**.
- Si flota LOW: el primer `while` espera ~20000 iteraciones ≈ **5 ms**.

Durante ese tiempo, el CPU no puede leer el FIFO USART.

El ATmega2560 tiene un FIFO USART de **3 bytes** (2 en buffer + 1 en shift
register). A 115200 baudios, un comando de 5 bytes (`PING\n`) tarda ~434 µs.
En 5–30 ms se puede recibir el comando entero pero los bytes 4 y 5 se pierden
por overflow (DOR — Data OverRun).

### Fix temporal (para test sin hardware)

Comentar el bloque HC-SR04 en `main.rs` (ver sección marcada).

### Fix de producción requerido

Implementar USART RX interrupt con ring buffer (ver §Pendientes).

---

## Problema 4 — `delay_ms(20)` causa overflow USART incluso sin HC-SR04

### Síntoma

Con HC-SR04 deshabilitado, los comandos seguían llegando truncados.
Evidencia del echo hex debug (ver `main.rs`, sección `// DEBUG`):

```
PING\n  → Arduino recibió: 50 49        (P I — solo 2 bytes de 5)
STB\n   → Arduino recibió: 53 54        (S T — solo 2 bytes de 4)
RST\n   → Arduino recibió: 52 53        (R S — solo 2 bytes de 4)
EXP:60:60\n → recibió: 45 58 50 3a     (E X P : — 4 de 10 bytes)
```

### Causa

El loop principal termina con `arduino_hal::delay_ms(LOOP_MS)` (20 ms de
busy-wait). Si el comando llega **mientras el CPU está en ese delay**, los
bytes se acumulan en el FIFO de 3 bytes y los restantes se pierden.

Secuencia de overflow con `PING\n` (5 bytes, 434 µs totales):

| Byte | Hex | Acción |
|------|-----|--------|
| `P`  | 50  | → FIFO[0] |
| `I`  | 49  | → FIFO[1] |
| `N`  | 4E  | → FIFO[2] (shift register) |
| `G`  | 47  | → **DOR: byte perdido** |
| `\n` | 0A  | → **DOR: byte perdido** |

Sin `\n`, `poll_command()` nunca retorna `true` y el comando se descarta.

### Fix aplicado

Eliminar el `delay_ms(LOOP_MS)` al final del loop. Sin delay, `poll_command`
se ejecuta en cada iteración del loop (cada ~decenas de µs), antes de que el
FIFO pueda llenarse.

```rust
// ANTES (producción, causa overflow en test USB):
arduino_hal::delay_ms(LOOP_MS);

// DESPUÉS (test USB, sin delay):
// Sin delay: poll_command corre continuamente para no saturar FIFO USART (3 bytes)
// Producción: implementar USART RX interrupt con ring buffer
```

**Efecto secundario conocido:** sin `delay_ms`, el watchdog MSM dispara en
~100 iteraciones de loop (microsegundos). No es problema para el test manual
interactivo pero se debe tener en cuenta.

---

## Resultados de verificación del protocolo MSM

Test ejecutado con `tests/test_msm_protocol.py` desde PC via USB a 115200.

| Comando    | Respuesta esperada | Respuesta obtenida | Estado |
|------------|--------------------|--------------------|--------|
| `PING`     | `PONG`             | `PONG`             | ✅     |
| `EXP:60:60`| `ACK:EXP`          | `ACK:EXP`          | ✅     |
| `STB`      | `ERR:ESTOP`        | `ERR:ESTOP`        | ✅ (*) |
| `RST`      | `ACK:STB`          | `ACK:STB`          | ✅     |

(*) `STB` retorna `ERR:ESTOP` porque `EXP:60:60` sin encoders físicos activa
FAULT por stall inmediato (`TLM:FAULT:111111`). Comportamiento correcto: en
FAULT todos los comandos excepto `RST` retornan `ERR:ESTOP`.

Telemetría observada:
- `TLM:NORMAL:000000` — estado idle correcto
- `TLM:FAULT:111111`  — stall en los 6 encoders al comandar EXP sin hardware

---

## Pendientes de producción

### 1. USART RX interrupt + ring buffer

El `delay_ms(20)` del loop de producción es necesario para cadenciar el ciclo
de control (encoders, HC-SR04, telemetría). Para que coexista con la recepción
USART confiable, se debe implementar:

1. ISR `USART3_RX` que lea `UDR3` y lo escriba en un ring buffer estático.
2. `poll_command()` pasa a leer del ring buffer en vez de `UDR3` directamente.
3. El ring buffer (32–64 bytes) absorbe ráfagas de comandos sin overflow.

### 2. HC-SR04 no bloqueante

Opciones:
- **Timer + comparador**: iniciar medición por timer, leer resultado en
  interrupción (no bloquea el loop).
- **Timeout corto**: reducir el timeout de 30 ms a 2–3 ms aceptando menor
  alcance máximo (suficiente para detección de obstáculos a <1 m).

### 3. Revertir USART0→USART3 para producción

`main.rs` actualmente usa `default_serial!` (USART0/USB) para testing.
Para despliegue en rover revertir a:

```rust
let serial_rpi = arduino_hal::Usart::new(
    dp.USART3,
    pins.d15,
    pins.d14.into_output(),
    115200_u32.into_baudrate(),
);
```
