#!/usr/bin/env python3
"""
test_msm_protocol.py — Verificación del protocolo MSM desde PC via USB
=======================================================================
Version: v2.2

Cambios v2.2:
  - Actualizado TLM_PATTERN al formato v2.8 completo:
      TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:...:<I5>:<T>C:<B0>:...<B5>C:<DIST>mm
    Añadidos: batt_mv (INA226 tensión bus), batt_ma (INA226 corriente total).
  - Actualizado el bloque de validación del test #13 con los nuevos grupos del regex.

Cambios v2.1:
  - Actualizado TLM_PATTERN al formato v2.7 completo:
      TLM:<SAFETY>:<STALL>:<TS>ms:<I0>:...:<I5>:<T>C:<B0>:...<B5>C:<DIST>mm
    Añadidos: tick_ms, 6 temperaturas NTC de celdas (B0–B5), distancia VL53L0X.

Cambios v2.0:
  - Soporte para el formato TLM extendido con sensores:
      TLM:<SAFETY>:<STALL_MASK>:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C
  - Añadido helper read_tlm() y test de formato TLM (#13).

Requisitos:
  - Arduino Mega 2560 conectado via USB (/dev/ttyUSB0 por defecto)
  - Firmware v2.7+ flasheado (feature/msm-main-integration)
  - pip install pyserial

Uso:
  python3 tests/hardware/test_msm_protocol.py [/dev/ttyUSBx]

Contexto:
  Este script verifica el protocolo ASCII MSM implementado en
  src/state_machine/mod.rs. Ver docs/debug_usart_overflow.md para
  el análisis de los problemas encontrados durante la depuración.
"""

import re
import serial
import time
import sys

# ── Configuración ─────────────────────────────────────────────────────────────

PORT     = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyUSB0"
BAUD     = 115200
TIMEOUT  = 3.0   # segundos
BOOT_WAIT = 2.0  # espera tras reset por DTR

# Formato TLM v2.12:
#   TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:<I1>:<I2>:<I3>:<I4>:<I5>:<T>C:<B0>:<B1>:<B2>:<B3>:<B4>:<B5>C:<DIST>mm:<EL>:<ER>
# Ejemplo:
#   TLM:NORMAL:000000:1000ms:14800mV:1200mA:1150:980:1100:1050:1200:1180:27C:28:29:28:30:29:28C:342mm:60:62
# Grupos: 1=safety, 2=stall, 3=tick_ms, 4=batt_mv, 5=batt_ma, 6-11=I0-I5, 12=T, 13-18=B0-B5, 19=dist_mm, 20=enc_left, 21=enc_right
TLM_PATTERN = re.compile(
    r"^TLM:(NORMAL|WARN|LIMIT|FAULT):"     # 1: safety state
    r"([01]{6}):"                           # 2: stall mask 6 bits
    r"(\d+)ms:"                             # 3: tick_ms (timestamp Arduino)
    r"(\d+)mV:"                             # 4: tensión batería en mV (INA226)
    r"(-?\d+)mA:"                           # 5: corriente batería en mA (INA226)
    r"(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):"  # 6-11: corrientes motores I0-I5
    r"(-?\d+)C:"                            # 12: temperatura ambiente LM335 (°C)
    r"(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+)C:"  # 13-18: NTC celdas B0-B5 (°C)
    r"(\d+)mm:"                             # 19: distancia VL53L0X (mm)
    r"(-?\d+):(-?\d+)$"                     # 20-21: enc_left, enc_right (odometría)
)

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

def read_tlm(s: serial.Serial, max_lines: int = 64) -> str:
    """Espera y retorna la primera línea TLM recibida, o '' si hay timeout."""
    for _ in range(max_lines):
        line = s.readline().decode(errors="ignore").strip()
        if line.startswith("TLM:"):
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

    # 13. Formato TLM v2.8 completo
    #     TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:...:<I5>:<T>C:<B0>:...<B5>C:<DIST>mm
    print("\n--- Test TLM v2.8 ---")
    total += 1
    send(s, b"RST\n")   # aseguramos STANDBY
    time.sleep(0.1)
    s.reset_input_buffer()
    send(s, b"PING\n")  # dispara ciclo del loop que emite TLM periódico
    tlm = read_tlm(s, max_lines=128)
    if tlm:
        m = TLM_PATTERN.match(tlm)
        if m:
            print(f"  [PASS] {'TLM formato OK':20s}  got={tlm!r}")
            tick_ms    = int(m.group(3))
            batt_mv    = int(m.group(4))
            batt_ma    = int(m.group(5))
            currents   = [int(m.group(i)) for i in range(6, 12)]
            temp_amb   = int(m.group(12))
            cell_temps = [int(m.group(i)) for i in range(13, 19)]
            dist_mm    = int(m.group(19))
            # Rangos físicos razonables (sin hardware puede dar 0)
            if batt_mv > 0 and not 8000 <= batt_mv <= 20000:
                print(f"  [WARN] Tensión batería fuera de rango: {batt_mv} mV")
            if not all(-30000 <= c <= 30000 for c in currents):
                print(f"  [WARN] Corrientes motores fuera de rango: {currents}")
            if not -40 <= temp_amb <= 125:
                print(f"  [WARN] Temp ambiente fuera de rango: {temp_amb} C")
            if not all(-40 <= t <= 125 for t in cell_temps):
                print(f"  [WARN] Temp celdas fuera de rango: {cell_temps}")
            if not 0 <= dist_mm <= 2000:
                print(f"  [WARN] Distancia fuera de rango: {dist_mm} mm")
            if batt_mv == 0:
                print(f"  [WARN] batt_mv=0 (INA226 no conectado o sin shunt)")
            if tick_ms == 0:
                print(f"  [WARN] tick_ms=0 (firmware recién arrancado, normal)")
            print(f"  [INFO] Batería: {batt_mv} mV / {batt_ma} mA")
            passed += 1
        else:
            print(f"  [FAIL] {'TLM formato':20s}  got={tlm!r}")
            print(f"         Esperado: TLM:<SAFETY>:<STALL>:<TS>ms:<MV>mV:<MA>mA:<I0>:...:<I5>:<T>C:<B0>:...<B5>C:<DIST>mm")
    else:
        print(f"  [FAIL] {'TLM no recibido':20s}  (timeout esperando TLM:)")

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
