#!/usr/bin/env python3
"""
test_msm_protocol.py — Verificación del protocolo MSM desde PC via USB
=======================================================================
Requisitos:
  - Arduino Mega 2560 conectado via USB (/dev/ttyUSB0 por defecto)
  - Firmware feature/msm-main-integration flasheado en modo USB (USART0)
  - pip install pyserial

Uso:
  python3 tests/test_msm_protocol.py [/dev/ttyUSBx]

Contexto:
  Este script verifica el protocolo ASCII MSM implementado en
  src/state_machine/mod.rs. Ver docs/debug_usart_overflow.md para
  el análisis de los problemas encontrados durante la depuración.
"""

import serial
import time
import sys

# ── Configuración ─────────────────────────────────────────────────────────────

PORT     = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyUSB0"
BAUD     = 115200
TIMEOUT  = 3.0   # segundos
BOOT_WAIT = 2.0  # espera tras reset por DTR

# ── Helpers ───────────────────────────────────────────────────────────────────

RESPONSE_PREFIXES = ("PONG", "ACK:", "ERR:")

def send(s: serial.Serial, cmd: bytes) -> str:
    """Envía un comando y retorna la primera línea de respuesta del protocolo MSM.

    Descarta líneas de telemetría (TLM:), debug (DBG:), hex dump y vacías.
    Solo acepta líneas que empiecen con un prefijo válido del protocolo.
    """
    s.write(cmd)
    for _ in range(32):  # máximo 32 líneas antes de rendirse
        line = s.readline().decode(errors="ignore").strip()
        if any(line.startswith(p) for p in RESPONSE_PREFIXES):
            return line
    return ""

def check(label: str, got: str, expected: str):
    ok = got == expected
    status = "PASS" if ok else "FAIL"
    print(f"  [{status}] {label:20s}  got={got!r:20s}  expected={expected!r}")
    return ok

# ── Main ──────────────────────────────────────────────────────────────────────

def main():
    print(f"Conectando a {PORT} @ {BAUD}...")
    s = serial.Serial(PORT, BAUD, timeout=TIMEOUT)
    time.sleep(BOOT_WAIT)
    s.reset_input_buffer()
    print("Listo.\n")

    passed = 0
    total  = 0

    print("=== Test protocolo MSM ===\n")

    # 1. Ping/Pong — verifica que el firmware está vivo
    total += 1
    resp = send(s, b"PING\n")
    if check("PING → PONG", resp, "PONG"): passed += 1

    # 2. Standby desde estado inicial
    total += 1
    resp = send(s, b"STB\n")
    if check("STB → ACK:STB", resp, "ACK:STB"): passed += 1

    # 3. Explore con velocidades simétricas
    total += 1
    resp = send(s, b"EXP:60:60\n")
    if check("EXP:60:60 → ACK:EXP", resp, "ACK:EXP"): passed += 1

    # 4. Ping durante movimiento — resetea watchdog
    total += 1
    resp = send(s, b"PING\n")
    if check("PING (en EXP) → PONG", resp, "PONG"): passed += 1

    # 5. Standby durante movimiento → ERR:ESTOP si encoders en stall
    #    (sin hardware: stall inmediato → FAULT → ERR:ESTOP)
    #    Con hardware real → ACK:STB
    total += 1
    resp = send(s, b"STB\n")
    ok_hw    = resp == "ACK:STB"
    ok_nohw  = resp == "ERR:ESTOP"
    if ok_hw or ok_nohw:
        note = "(hardware)" if ok_hw else "(sin encoders — stall → FAULT esperado)"
        print(f"  [PASS] {'STB tras EXP':20s}  got={resp!r}  {note}")
        passed += 1
    else:
        print(f"  [FAIL] {'STB tras EXP':20s}  got={resp!r}  expected=ACK:STB or ERR:ESTOP")

    # 6. Reset — sale de FAULT
    total += 1
    resp = send(s, b"RST\n")
    if check("RST → ACK:STB", resp, "ACK:STB"): passed += 1

    # 7. Explore asimétrico (giro)
    total += 1
    resp = send(s, b"EXP:-50:50\n")
    if check("EXP:-50:50 → ACK:EXP", resp, "ACK:EXP"): passed += 1

    # 8. Reset de nuevo
    total += 1
    resp = send(s, b"RST\n")
    if check("RST → ACK:STB", resp, "ACK:STB"): passed += 1

    # 9. Avoid Left
    total += 1
    resp = send(s, b"AVD:L\n")
    if check("AVD:L → ACK:AVD", resp, "ACK:AVD"): passed += 1

    # 10. Reset
    total += 1
    resp = send(s, b"RST\n")
    if check("RST → ACK:STB", resp, "ACK:STB"): passed += 1

    # 11. Retreat
    total += 1
    resp = send(s, b"RET\n")
    if check("RET → ACK:RET", resp, "ACK:RET"): passed += 1

    # 12. Comando inválido
    total += 1
    resp = send(s, b"FOOBAR\n")
    if check("FOOBAR → ERR:UNKNOWN", resp, "ERR:UNKNOWN"): passed += 1

    s.close()

    print(f"\n{'='*40}")
    print(f"Resultado: {passed}/{total} tests pasaron")
    if passed == total:
        print("TODOS LOS TESTS PASARON")
    else:
        print(f"FALLARON {total - passed} tests")
    return 0 if passed == total else 1

if __name__ == "__main__":
    sys.exit(main())
