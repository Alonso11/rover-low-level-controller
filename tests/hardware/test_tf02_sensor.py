#!/usr/bin/env python3
"""
test_tf02_sensor.py — Verificación rápida del sensor TF02 LiDAR (standalone)

NO requiere el Arduino Mega. Conectar el TF02 directamente a la PC o RPi5.

═══════════════════════════════════════════════════════════════════
 GUÍA DE CABLEADO SEGURO — leer antes de conectar
═══════════════════════════════════════════════════════════════════

  RPi5 (recomendado):
    TF02 rojo  (5V)    → RPi pin 2 ó 4 (5V VBUS USB)
    TF02 negro (GND)   → RPi pin 6 (GND)
    TF02 verde (TX)    → RPi pin 10 (GPIO15 / UART0 RX)
    TF02 blanco (RX)   → NO CONECTAR

    En RPi5: habilitar UART con "sudo raspi-config → Interface Options → Serial Port"
    Puerto: /dev/ttyAMA0  ó  /dev/serial0

  USB-TTL 3.3V (CP2102 / CH340 / FT232RL):
    TF02 rojo  (5V)    → USB-TTL VCC 5V  (verificar que el módulo tenga pin 5V)
    TF02 negro (GND)   → USB-TTL GND
    TF02 verde (TX)    → USB-TTL RX
    TF02 blanco (RX)   → NO CONECTAR
    Puerto: /dev/ttyUSB0

  ⚠️  PRECAUCIONES:
    - NO conectar TF02 y Arduino al mismo tiempo si comparten 5V
    - El FT232H en modo MPSSE/sigrok NO funciona aquí — necesita modo VCP
    - El TF02 TX es LVTTL 3.3V → sin riesgo para el RX de RPi o USB-TTL 3.3V
    - NO apuntar el láser a ojos (850 nm NIR, clase 1 — seguro en uso normal)

═══════════════════════════════════════════════════════════════════

Uso:
    python3 test_tf02_sensor.py [puerto] [--samples N] [--verbose]

    python3 test_tf02_sensor.py /dev/ttyUSB0
    python3 test_tf02_sensor.py /dev/serial0 --samples 30
    python3 test_tf02_sensor.py /dev/ttyUSB0 --verbose
"""

import sys
import time
import argparse
import statistics

try:
    import serial
except ImportError:
    print("ERROR: pyserial no instalado. Ejecutar: pip install pyserial")
    sys.exit(1)

BAUD       = 115200
HEADER     = 0x59
FRAME_SIZE = 9


def parse_args():
    parser = argparse.ArgumentParser(description="Test TF02 LiDAR sensor")
    parser.add_argument("port", nargs="?", default=None,
                        help="Puerto serie (e.g. /dev/ttyUSB0, /dev/serial0)")
    parser.add_argument("--samples", type=int, default=20,
                        help="Número de frames a leer (default 20)")
    parser.add_argument("--verbose", action="store_true",
                        help="Mostrar cada frame recibido")
    return parser.parse_args()


def detect_port():
    """Detecta automáticamente el primer puerto USB-TTL disponible."""
    import glob
    candidates = glob.glob("/dev/ttyUSB*") + glob.glob("/dev/ttyACM*") + \
                 glob.glob("/dev/serial*") + glob.glob("/dev/ttyAMA*")
    return candidates[0] if candidates else None


def checksum_ok(buf: bytes) -> bool:
    """CHECK = (B0+B1+B2+B3+B4+B5+B6+B7) & 0xFF — igual que el driver Rust."""
    return (sum(buf[:8]) & 0xFF) == buf[8]


def parse_frame(buf: bytes) -> dict | None:
    """Parsea un frame TF02 de 9 bytes. Retorna dict o None si no es válido."""
    if len(buf) != 9 or buf[0] != HEADER or buf[1] != HEADER:
        return None
    if not checksum_ok(buf):
        return None
    sig  = buf[6]
    dist_cm   = buf[2] | (buf[3] << 8)
    strength  = buf[4] | (buf[5] << 8)
    time_byte = buf[7]
    return {
        "dist_cm":  dist_cm,
        "dist_mm":  dist_cm * 10,
        "strength": strength,
        "sig":      sig,
        "reliable": sig in (7, 8),
        "oor":      dist_cm >= 2200,
        "time":     time_byte,
        "raw":      buf.hex(" ").upper(),
    }


def read_frames(ser: serial.Serial, n: int, verbose: bool) -> list[dict]:
    """Lee n frames válidos del stream TF02."""
    frames = []
    bad_checksum = 0
    bad_sig      = 0
    oor_count    = 0
    byte_buf     = bytearray()

    deadline = time.time() + n * 0.5 + 5.0  # timeout generoso

    while len(frames) < n and time.time() < deadline:
        chunk = ser.read(1)
        if not chunk:
            continue
        byte_buf.extend(chunk)

        # Buscar el header 0x59 0x59 y extraer frames
        while len(byte_buf) >= FRAME_SIZE:
            # Sincronizar
            idx = 0
            while idx < len(byte_buf) - 1:
                if byte_buf[idx] == HEADER and byte_buf[idx + 1] == HEADER:
                    break
                idx += 1

            if idx > 0:
                byte_buf = byte_buf[idx:]  # descartar bytes de basura
                continue

            if len(byte_buf) < FRAME_SIZE:
                break

            frame_bytes = bytes(byte_buf[:FRAME_SIZE])
            byte_buf = byte_buf[FRAME_SIZE:]

            if not checksum_ok(frame_bytes):
                bad_checksum += 1
                continue

            f = parse_frame(frame_bytes)
            if f is None:
                continue

            if f["oor"]:
                oor_count += 1
                if verbose:
                    print(f"  [FRAME oor] dist=OOR  str={f['strength']:5d}  sig={f['sig']}")
                continue

            if not f["reliable"]:
                bad_sig += 1
                if verbose:
                    print(f"  [FRAME low_sig] dist={f['dist_mm']:5d}mm  str={f['strength']:5d}  sig={f['sig']}")
                continue

            frames.append(f)
            if verbose:
                print(f"  [FRAME {len(frames):2d}/{n}] dist={f['dist_mm']:5d}mm  "
                      f"str={f['strength']:5d}  sig={f['sig']}  time=0x{f['time']:02X}  "
                      f"| {f['raw']}")

    print(f"\n  Descartados → checksum_err={bad_checksum}  sig_bajo={bad_sig}  fuera_rango={oor_count}")
    return frames


def run_checks(frames: list[dict]) -> list[tuple[str, bool, str]]:
    """Evalúa criterios de aceptación. Retorna lista (nombre, ok, detalle)."""
    checks = []
    n = len(frames)

    # 1. Se recibieron frames
    checks.append(("frames_recibidos", n > 0, f"{n} frames"))

    if n == 0:
        return checks

    dists = [f["dist_mm"] for f in frames]
    strs  = [f["strength"] for f in frames]
    sigs  = [f["sig"] for f in frames]

    # 2. Todos los frames son confiables (SIG 7 u 8)
    all_reliable = all(s in (7, 8) for s in sigs)
    checks.append(("sig_fiable", all_reliable,
                   f"SIG mín={min(sigs)} máx={max(sigs)}"))

    # 3. Distancia en rango físico del sensor (40 cm – 21.99 m)
    in_range = all(400 <= d <= 21990 for d in dists)
    checks.append(("dist_en_rango", in_range,
                   f"min={min(dists)}mm  max={max(dists)}mm"))

    # 4. Estabilidad: std dev < 5 % de la mediana (superficie estática)
    median = statistics.median(dists)
    if median > 0:
        std    = statistics.stdev(dists) if n > 1 else 0
        stable = std < (0.05 * median)
        checks.append(("estabilidad", stable,
                       f"mediana={median:.0f}mm  std={std:.1f}mm  ({100*std/median:.1f}%)"))

    # 5. Señal suficiente (strength > 100 — sensor ve algo real)
    good_str = all(s > 100 for s in strs)
    checks.append(("strength_ok", good_str,
                   f"mín={min(strs)}  promedio={statistics.mean(strs):.0f}"))

    return checks


def main():
    args = parse_args()

    port = args.port or detect_port()
    if port is None:
        print("ERROR: No se encontró ningún puerto serie.")
        print("  Conectar el USB-TTL y especificar el puerto: python3 test_tf02_sensor.py /dev/ttyUSB0")
        sys.exit(1)

    print(f"\n{'='*60}")
    print(f"  TF02 LiDAR — Test de verificación standalone")
    print(f"{'='*60}")
    print(f"  Puerto : {port}")
    print(f"  Baud   : {BAUD}")
    print(f"  Frames : {args.samples}")
    print(f"{'='*60}\n")

    try:
        ser = serial.Serial(port, BAUD, timeout=1.0)
    except serial.SerialException as e:
        print(f"ERROR: No se puede abrir {port}: {e}")
        sys.exit(1)

    # Purgar buffer de arranque (~100 ms de frames acumulados)
    time.sleep(0.2)
    ser.reset_input_buffer()
    print(f"  Leyendo {args.samples} frames fiables...\n")

    t_start = time.time()
    frames  = read_frames(ser, args.samples, args.verbose)
    elapsed = time.time() - t_start
    ser.close()

    # Resultados
    checks  = run_checks(frames)
    passed  = sum(1 for _, ok, _ in checks if ok)
    total   = len(checks)

    print(f"\n{'─'*60}")
    print(f"  Resultados ({elapsed:.1f}s)")
    print(f"{'─'*60}")
    for name, ok, detail in checks:
        icon = "✓" if ok else "✗"
        print(f"  {icon}  {name:<22} {detail}")

    print(f"\n{'═'*60}")
    if passed == total and len(frames) > 0:
        dists = [f["dist_mm"] for f in frames]
        print(f"  PASS  {passed}/{total} checks — sensor operativo")
        print(f"  Distancia promedio: {statistics.mean(dists):.0f} mm  "
              f"({statistics.mean(dists)/10:.1f} cm)")
    else:
        print(f"  FAIL  {passed}/{total} checks")
        if len(frames) == 0:
            print("\n  Diagnóstico:")
            print("  - Verificar cableado: TF02 verde (TX) → RX del adaptador")
            print("  - Verificar 5V en TF02 rojo (medir con multímetro)")
            print("  - Verificar que el puerto es el correcto")
            print("  - Verificar que no hay otro proceso usando el puerto")
    print(f"{'═'*60}\n")

    sys.exit(0 if passed == total and len(frames) > 0 else 1)


if __name__ == "__main__":
    main()
