<!-- Version: v1.1 -->
# Testing Guide — rover-low-level-controller

Este documento explica cómo verificar que el código no se rompe tras cada
cambio, sin necesidad de un Arduino conectado para la mayoría de los tests.

---

## 1. Estrategia de testing

El firmware tiene dos tipos de código con ciclos de feedback muy distintos:

```
┌──────────────────────────────────────────────────────────────────┐
│  Código puro Rust (portable)          │  Código AVR (hardware)   │
│  state_machine, sensors analógicos,   │  main.rs, ISRs, drivers  │
│  lógica de motores                    │  que usan arduino-hal    │
│                                       │                          │
│  ✓ Compila y corre en x86             │  ✗ Solo compila para AVR │
│  ✓ Tests unitarios e integración      │  ✓ Tests de hardware     │
│  ✓ Feedback inmediato (segundos)      │  ✓ Requiere Arduino + USB│
└──────────────────────────────────────────────────────────────────┘
```

La clave es el feature gate `avr` en `Cargo.toml`:

```toml
[features]
default = ["avr"]          # ON para builds normales (AVR)
avr = ["arduino-hal", ...]  # OFF para tests x86 (--no-default-features)
```

Con `--no-default-features` el binario `main.rs` se excluye
(`required-features = ["avr"]`) y solo compila la librería portable.

---

## 2. Tests de lógica — x86, sin hardware

### Comando rápido

```bash
./test_native.sh
```

Este script corre las tres suites de tests x86 secuencialmente y reporta
un resultado global. Salida esperada:

```
[test_native] ── state_machine_test ────────────────────────
running 46 tests
test test_parse_ping ... ok
...
test result: ok. 46 passed; 0 failed

[test_native] ── sensors_test ────────────────────────
running 57 tests
...
test result: ok. 57 passed; 0 failed

[test_native] ── motor_logic_test ────────────────────────
running 28 tests
...
test result: ok. 28 passed; 0 failed

[test_native] OK Todas las suites pasaron (3/3)
```

### Comando manual equivalente

Si el script no está disponible o se quiere correr una suite específica:

```bash
RUSTFLAGS="-C panic=unwind" \
  cargo +nightly test \
    --no-default-features \
    --test state_machine_test \
    --target x86_64-unknown-linux-gnu
```

| Flag | Por qué es necesario |
|------|----------------------|
| `RUSTFLAGS="-C panic=unwind"` | El perfil `dev` tiene `panic=abort` (necesario en AVR). En x86 la `core` precompilada usa `unwind` → hay que sobrescribir. |
| `+nightly` | El crate usa features de nightly (`abi_avr_interrupt`, `build-std`). |
| `--no-default-features` | Desactiva el feature `avr` → excluye `arduino-hal` y el binary `main.rs`. Sin esto el build falla con errores de target AVR. |
| `--test <suite>` | Selecciona un integration test concreto. Evita intentar compilar los `examples/` que dependen de `arduino-hal`. |
| `--target x86_64-unknown-linux-gnu` | Compila para la máquina de desarrollo en lugar del target AVR por defecto (`.cargo/config.toml` fija el target a `avr-atmega2560.json`). |

### Por qué no `cargo test --tests`

`--tests` intenta compilar todos los targets de tipo test **incluyendo los
`examples/`**, que dependen de `arduino-hal` y fallan en x86. Usar
`--test <nombre>` es más preciso y evita ese ruido.

---

## 3. Suites de tests x86

| Suite | Tests | Archivo | Qué cubre |
|-------|-------|---------|-----------|
| `state_machine_test` | 46 | `tests/state_machine_test.rs` | Parser de comandos MSM, todas las transiciones de estado, watchdog, format_response, TLM con/sin sensores |
| `sensors_test` | 57 | `tests/sensors_test.rs` | ACS712 conversión ADC→mA, LM335 conversión ADC→°C, NTC interpolación LUT, umbrales Warn/Limit/Fault |
| `motor_logic_test` | 28 | `tests/motor_logic_test.rs` | Speed mapping, signos de dirección L298N/BTS7960, SixWheelRover drive diferencial, ErasedMotor |

### Añadir un test nuevo

1. Abrir el archivo `.rs` de la suite correspondiente.
2. Añadir una función `#[test]` normal de Rust.
3. Verificar con `./test_native.sh`.

El código de los tests no tiene acceso a `arduino-hal` ni a ningún
periférico hardware — solo a los módulos portables de la librería
(`state_machine`, `sensors::acs712`, `sensors::lm335`, `sensors::ntc_thermistor`).

---

## 4. Tests de hardware — PC + Arduino via USB

Requieren el Arduino conectado con el firmware correcto flasheado.

### Requisitos previos

```bash
pip install pyserial
```

### test_msm_protocol.py — protocolo MSM completo

Valida el protocolo de comunicación de extremo a extremo:

```bash
python3 tests/hardware/test_msm_protocol.py [/dev/ttyUSB0]
```

**Firmware requerido:** firmware principal (`main.rs`) v2.10+

**Qué verifica (13 tests):**
1. Conexión serial al Arduino
2. PING → PONG (keepalive)
3. STB → ACK:STB (standby)
4. EXP:50:50 → ACK:EXP (explorar)
5. AVD:L → ACK:AVD (evasión izquierda)
6. AVD:R → ACK:AVD (evasión derecha)
7. RET → ACK:RET (retroceder)
8. RST → ACK:STB (reset)
9. Comando desconocido → ERR:UNKNOWN
10. FLT → ACK:FLT (forzar fault)
11. Comando en FAULT → ERR:ESTOP
12. RST desde FAULT → ACK:STB
13. Formato TLM v2.8 con regex (19 campos + rangos de batería)

### test_motors_debug.py — debug de motores

Control interactivo para verificar cada motor individualmente:

```bash
python3 tests/hardware/test_motors_debug.py [/dev/ttyUSB0]
```

**Firmware requerido:** `examples/debug_motors_l298n`

**Uso interactivo:**
```
> f   # Forward — todos adelante
> b   # Backward — todos atrás
> s   # Stop
> 1f  # Motor 1 adelante
> 3s  # Motor 3 stop
> q   # Salir
```

---

## 5. Flujo de trabajo recomendado

### Antes de cada commit

```bash
# 1. Correr tests de lógica (< 15 segundos)
./test_native.sh

# 2. Si todo pasa → commit
git add <archivos>
git commit -m "tipo(scope): descripción"
```

### Tras cambios en el protocolo MSM o sensores

```bash
# Después del commit de lógica, flashear y verificar en hardware
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --release -Zjson-target-spec -Zbuild-std=core

python3 tests/hardware/test_msm_protocol.py
```

### Antes de merge a `debug`

- [ ] `./test_native.sh` — todas las suites en verde
- [ ] `test_msm_protocol.py` — 13/13 tests en hardware
- [ ] Inspección visual de telemetría TLM (formato v2.10 correcto)
- [ ] Verificar watchdog: esperar >2 s sin PING → ERR:WDOG

---

## 6. ¿Por qué el compilador AVR no aplica a los tests?

El archivo `.cargo/config.toml` fija el target AVR por defecto:

```toml
[build]
target = "avr-atmega2560.json"
```

Esto significa que `cargo build` sin flags compila para AVR. Para los tests
x86 **siempre hay que pasar `--target x86_64-unknown-linux-gnu`
y `--no-default-features`** — o usar `./test_native.sh` que lo hace
automáticamente.

Si se olvida alguno de los flags, el error típico es:

```
error: Ensure that you are using an AVR target!
```

→ Falta `--target x86_64-unknown-linux-gnu`, o se está intentando compilar
`main.rs` sin el flag `--no-default-features`.

---

## 7. Troubleshooting

| Síntoma | Causa probable | Solución |
|---------|----------------|----------|
| `error: Ensure that you are using an AVR target!` | Falta `--target x86_64-unknown-linux-gnu` | Usar `./test_native.sh` o añadir el flag |
| `error: cannot find module arduino_hal` | Falta `--no-default-features` | Añadir el flag |
| `SIGSEGV` / `illegal instruction` en tests | `panic=abort` vs `panic=unwind` | Añadir `RUSTFLAGS="-C panic=unwind"` |
| Puerto serie no encontrado | Arduino desconectado o driver no cargado | `ls /dev/ttyUSB*` y verificar udev |
| `test_msm_protocol.py` timeout en test 13 | TLM no llega en 2 s | Verificar que el firmware es v2.10+ (TLM cada ~1 s) |
