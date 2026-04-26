#!/usr/bin/env python3
"""
Test TF02 LiDAR UART reception via firmware Olympus LLC.

Reads the serial port for 15 s and reports:
  - Raw bytes captured from USART2 (TF02_RAW: hex dump)
  - SIG value of the first valid frame (INFO:TF02_SIG)
  - Distance readings from TLM (last field: dist_far_mm)

Usage:
    python3 tests/hardware/test_tf02.py [port]
    make test-tf02 PORT=/dev/ttyACM1

Connection (USART2 @ 115200 8N1):
    TF02 green (TX) -> Mega D17 (RX2)
    TF02 red        -> 5V
    TF02 black      -> GND
    TF02 white (RX) -> not connected
"""

import sys
import serial
import time

PORT    = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyUSB0"
BAUD    = 115200
TIMEOUT = 15

def parse_dist_far(line):
    if not line.startswith("TLM:"):
        return None
    try:
        last = line.rsplit(":", 1)[-1]
        if last.endswith("mm"):
            return int(last[:-2])
    except (ValueError, IndexError):
        pass
    return None

def parse_raw_dump(line):
    """Parse TF02_RAW:XX XX XX ... and return list of ints."""
    try:
        hex_part = line.split("TF02_RAW:")[1]
        return [int(b, 16) for b in hex_part.split()]
    except (IndexError, ValueError):
        return []

def annotate_frame(raw):
    """Try to identify TF02 frames in a raw byte list."""
    lines = []
    i = 0
    while i < len(raw):
        if i + 1 < len(raw) and raw[i] == 0x59 and raw[i+1] == 0x59:
            frame = raw[i:i+9]
            if len(frame) == 9:
                dist_cm = frame[3] << 8 | frame[2]
                strength = frame[5] << 8 | frame[4]
                sig = frame[6]
                check = sum(frame[:8]) & 0xFF
                ok = "OK" if frame[8] == check else f"BAD(expected {check:02X})"
                lines.append(f"  Frame @ byte {i:2d}: dist={dist_cm}cm  str={strength}  sig={sig}  chk={ok}")
                i += 9
                continue
        i += 1
    return lines

def run():
    print(f"\n=== TF02 LiDAR UART test — {PORT} ({TIMEOUT} s) ===\n")
    try:
        ser = serial.Serial(PORT, BAUD, timeout=0.5)
    except serial.SerialException as e:
        print(f"ERROR: {e}")
        sys.exit(1)

    time.sleep(0.5)
    ser.reset_input_buffer()

    raw_rx       = None
    sig_value    = None
    raw_bytes    = []
    dist_samples = []
    deadline     = time.time() + TIMEOUT

    print(f"{'t(s)':<6} Evento")
    print("-" * 55)

    while time.time() < deadline:
        data = ser.readline()
        if not data:
            continue
        line = data.decode("ascii", errors="replace").strip()
        if not line:
            continue

        t = f"{time.time() - (deadline - TIMEOUT):.1f}"

        if "TF02_NO_DATA" in line:
            print(f"{t:<6} {line}")
            if "RX=" in line:
                try:
                    raw_rx = int(line.split("RX=")[1])
                except ValueError:
                    pass

        elif "TF02_RAW:" in line:
            raw_bytes = parse_raw_dump(line)
            print(f"{t:<6} TF02_RAW: {len(raw_bytes)} bytes capturados")

        elif "TF02_SIG" in line:
            print(f"{t:<6} {line}")
            try:
                sig_value = int(line.split("TF02_SIG:")[1])
            except (ValueError, IndexError):
                pass

        elif "TF02_INIT" in line:
            print(f"{t:<6} {line}")

        elif line.startswith("TLM:"):
            dist = parse_dist_far(line)
            if dist is not None and dist > 0:
                dist_samples.append(dist)
                print(f"{t:<6} dist_far_mm = {dist} mm")

    ser.close()

    # ── Resumen ──────────────────────────────────────────────────────────────
    print("\n" + "=" * 55)
    print("RESULTADO")
    print("=" * 55)

    if raw_bytes:
        hex_str = " ".join(f"{b:02X}" for b in raw_bytes)
        print(f"\n  Bytes crudos USART2 ({len(raw_bytes)}):")
        # Imprimir en grupos de 9 (tamaño de un frame)
        for i in range(0, len(raw_bytes), 9):
            chunk = raw_bytes[i:i+9]
            print(f"    [{i:2d}]  {' '.join(f'{b:02X}' for b in chunk)}")
        frames = annotate_frame(raw_bytes)
        if frames:
            print("\n  Frames identificados:")
            for f in frames:
                print(f)
        else:
            print("\n  No se identifico el patron 0x59 0x59 en los bytes capturados.")
            if any(b == 0x59 for b in raw_bytes):
                idx = [i for i, b in enumerate(raw_bytes) if b == 0x59]
                print(f"  0x59 encontrado en posiciones: {idx} (frame cortado o baud incorrecto)")

    if sig_value is not None:
        print(f"\n  Frames validos: SI  (SIG={sig_value})")
        if dist_samples:
            print(f"  Distancias:     {min(dist_samples)}–{max(dist_samples)} mm ({len(dist_samples)} muestras)")
        print("\n  TF02 conectado y funcionando correctamente.")
    elif raw_rx == 0:
        print(f"\n  Bytes en USART2: 0")
        print("\n  FALLO: ningún byte llega a D17 (RX2).")
        print("  Verificar cable verde y alimentacion 5V.")
    elif raw_rx is not None:
        print(f"\n  Bytes en USART2: {raw_rx}")
        print("\n  Llegan bytes pero ningun frame valido.")
        print("  Ver dump arriba para diagnosticar baud/protocolo.")
    else:
        print("  Sin datos TF02 en la ventana de observacion.")
        print("  Verificar que el firmware es v2.17+.")

if __name__ == "__main__":
    run()
