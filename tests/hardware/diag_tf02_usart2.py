#!/usr/bin/env python3
"""
diag_tf02_usart2.py — Diagnóstico TF02 a través del Mega (USART0 TLM)

Lee dist_far_mm del stream TLM del Mega para verificar que USART2 recibe
frames del TF02. A diferencia de test_tf02_sensor.py, este script NO
requiere conexión directa del TF02 al PC — solo el cable USB del Mega.

Diagnóstico típico:
  dist_far_mm = 0 siempre  → USART2 no recibe bytes del TF02
                             Causas: cableado, rango < 40 cm, baud rate
  dist_far_mm > 0 variable → TF02 operativo, sensor integrado OK

Rango mínimo del TF02: 40 cm (400 mm). A menor distancia SIG < 7 y el
driver Rust descarta el frame, dejando dist_far_mm en 0.

Uso:
    python3 diag_tf02_usart2.py [PORT]
    python3 diag_tf02_usart2.py /dev/ttyUSB0
"""

import sys
import time
import re
import argparse

try:
    import serial
except ImportError:
    print("ERROR: pyserial no instalado. Ejecutar: pip install pyserial")
    sys.exit(1)

# Campo dist_far_mm es el último campo numérico de la trama TLM
# Formato: TLM:<STATE>:...<dist_mm>mm:<x>:<y>:<theta>:<encL>:<encR>:<dist_far_mm>mm
TLM_RE = re.compile(r'TLM:(\w+):\d+:(\d+)ms:.*?:(\d+)mm:[-\d]+:[-\d]+:[-\d]+:[-\d]+:[-\d]+:(\d+)mm')


def parse_args():
    p = argparse.ArgumentParser(description="Diagnóstico TF02 via TLM del Mega")
    p.add_argument("port", nargs="?", default="/dev/ttyUSB0")
    p.add_argument("--seconds", type=int, default=10)
    return p.parse_args()


def main():
    args = parse_args()
    print(f"\n{'='*55}")
    print(f"  Diagnóstico TF02 vía USART2 — Mega {args.port}")
    print(f"  Duración: {args.seconds}s   Rango mínimo TF02: 40 cm")
    print(f"{'='*55}\n")
    print("  Apunta el TF02 a una superficie a ≥50 cm...\n")

    try:
        s = serial.Serial(args.port, 115200, timeout=1.0)
    except serial.SerialException as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    time.sleep(0.5)
    s.reset_input_buffer()

    readings = []
    deadline = time.time() + args.seconds

    while time.time() < deadline:
        line = s.readline().decode(errors="replace").strip()
        if not line.startswith("TLM:"):
            continue
        m = TLM_RE.match(line)
        if not m:
            continue
        state       = m.group(1)
        tick_ms     = int(m.group(2))
        dist_mm     = int(m.group(3))   # VL53L0X
        dist_far_mm = int(m.group(4))   # TF02
        readings.append(dist_far_mm)
        icon = "✓" if dist_far_mm > 0 else "✗"
        print(f"  {icon}  tick={tick_ms:6d}ms  VL53L0X={dist_mm:4d}mm  TF02={dist_far_mm:6d}mm  [{state}]")

    s.close()

    print(f"\n{'─'*55}")
    if not readings:
        print("  ERROR: No se recibieron tramas TLM del Mega.")
        sys.exit(1)

    nonzero = [r for r in readings if r > 0]
    print(f"  Frames TLM leídos : {len(readings)}")
    print(f"  Frames con TF02>0 : {len(nonzero)}")

    print(f"\n{'═'*55}")
    if nonzero:
        avg = sum(nonzero) / len(nonzero)
        print(f"  PASS — TF02 operativo via USART2")
        print(f"  Distancia promedio: {avg:.0f} mm  ({avg/10:.1f} cm)")
        sys.exit(0)
    print(f"  FAIL — dist_far_mm = 0 en todos los frames")
    print(f"\n  Acciones:")
    print(f"  1. Verificar rango: apuntar a superficie a ≥50 cm")
    print(f"  2. Verificar pin: TF02 verde (TX) → D17 (RX2) del Mega")
    print(f"  3. Verificar 5V en TF02 rojo con multímetro")
    print(f"  4. Probar TF02 standalone: python3 test_tf02_sensor.py <USB-TTL>")
    print(f"{'═'*55}\n")
    sys.exit(1)


if __name__ == "__main__":
    main()
