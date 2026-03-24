#!/usr/bin/env bash
# test_native.sh — Corre los tests de lógica pura en x86 (sin Arduino).
#
# Estrategia:
#   --no-default-features  → desactiva el feature "avr", omitiendo todos los
#                            módulos que dependen de arduino-hal.
#   --test state_machine_test → solo compila/corre ese integration test.
#   build-std deshabilitado → evita el conflicto de core duplicado.
#   RUSTFLAGS -C panic=unwind → overrride de panic=abort del perfil dev,
#                               necesario porque la core precompilada x86
#                               usa panic=unwind.
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

echo "[test_native] Corriendo tests nativos x86..."
RUSTFLAGS="-C panic=unwind" \
    cargo test \
        --no-default-features \
        --test state_machine_test \
        --target x86_64-unknown-linux-gnu \
        "$@"
