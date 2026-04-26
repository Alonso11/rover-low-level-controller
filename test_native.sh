#!/usr/bin/env bash
# test_native.sh — Corre los tests de lógica pura en x86 (sin Arduino).
#
# Estrategia:
#   --no-default-features  → desactiva el feature "avr", omitiendo todos los
#                            módulos que dependen de arduino-hal.
#   --test <suite>         → corre un integration test específico.
#   build-std deshabilitado → evita el conflicto de core duplicado.
#   RUSTFLAGS -C panic=unwind → override de panic=abort del perfil dev,
#                               necesario porque la core precompilada x86
#                               usa panic=unwind.
#
# Suites disponibles:
#   state_machine_test  — Máquina de estados maestra (MSM), parser, telemetría
#   sensors_test        — Drivers analógicos ACS712-30A y LM335
#   motor_logic_test    — Lógica de motor (speed mapping, signos)
#
# Uso: ./test_native.sh [-- <cargo test args>]

set -e

CONFIG=".cargo/config.toml"
BACKUP=".cargo/config.toml.bak"

cleanup() {
    if [ -f "$BACKUP" ]; then
        mv "$BACKUP" "$CONFIG"
        echo "[test_native] Config restaurada."
    fi
}
trap cleanup EXIT

cp "$CONFIG" "$BACKUP"

# Config temporal: sin build-std (no requerido para x86 con std)
cat > "$CONFIG" << 'EOF'
# TEMPORAL — generado por test_native.sh (no editar)
[build]
target = "avr-atmega2560.json"

[target.'cfg(target_arch = "avr")']
runner = "ravedude"
EOF

EXTRA_ARGS=("$@")   # args extra del script (e.g. -- --nocapture)
PASS=0
FAIL=0

run_suite() {
    local suite="$1"
    echo ""
    echo "[test_native] ── $suite ────────────────────────"
    if RUSTFLAGS="-C panic=unwind" \
        cargo +nightly test \
            --no-default-features \
            --test "$suite" \
            --target x86_64-unknown-linux-gnu \
            "${EXTRA_ARGS[@]}"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
}

run_suite state_machine_test
run_suite sensors_test
run_suite motor_logic_test

echo ""
if [ "$FAIL" -eq 0 ]; then
    echo "[test_native] OK Todas las suites pasaron ($PASS/$((PASS+FAIL)))"
else
    echo "[test_native] FALLO $FAIL suite(s) fallaron de $((PASS+FAIL))"
    exit 1
fi
