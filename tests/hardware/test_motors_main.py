#!/usr/bin/env python3
"""
test_motors_main.py — Test de motores con firmware principal (EXP mode).
Requiere: make flash-no-oc primero.

Controles:
    w / s    → adelante / atrás    (EXP:50:50 / EXP:-50:-50)
    a / d    → giro izq / der      (EXP:-30:30 / EXP:30:-30)
    x / 0    → stop                (EXP:0:0)
    N        → velocidad simétrica (-99..99)
    L,R      → diferencial manual  (ej: 60,-60)
    q        → stop + salir
"""

import serial
import serial.tools.list_ports
import sys
import threading
import time

BAUD = 115200
DEFAULT_PORT = "/dev/ttyUSB0"


def read_loop(ser, stop_event):
    while not stop_event.is_set():
        try:
            if ser.in_waiting:
                line = ser.readline().decode("utf-8", errors="replace").strip()
                if line:
                    print(f"  [Arduino] {line}")
        except Exception:
            break
        time.sleep(0.02)


def ping_loop(ser, stop_event):
    while not stop_event.is_set():
        try:
            ser.write(b"PING\n")
        except Exception:
            break
        time.sleep(1.5)


def send_exp(ser, left, right):
    ser.write(f"EXP:{left}:{right}\n".encode())
    time.sleep(0.15)


def parse_input(raw):
    cmd = raw.strip().upper()
    if cmd == "Q":         return "quit"
    if cmd == "":          return "noop"
    if cmd == "W":         return (50, 50)
    if cmd == "S":         return (-50, -50)
    if cmd == "A":         return (-30, 30)
    if cmd == "D":         return (30, -30)
    if cmd in ("X", "0"): return (0, 0)
    if cmd == "R":         return "rst"
    if "," in cmd:
        parts = cmd.split(",", 1)
        try:
            return (max(-99, min(99, int(parts[0]))),
                    max(-99, min(99, int(parts[1]))))
        except ValueError:
            pass
    try:
        v = max(-99, min(99, int(cmd)))
        return (v, v)
    except ValueError:
        pass
    return None


def main():
    port = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PORT

    print(f"\nPuertos disponibles:")
    for p in serial.tools.list_ports.comports():
        print(f"  {p.device:20s} — {p.description}")

    print(f"\nConectando a {port} @ {BAUD} baud...")
    try:
        ser = serial.Serial(port, BAUD, timeout=1)
    except serial.SerialException as e:
        print(f"\nERROR: {e}")
        sys.exit(1)

    print("Esperando reset Arduino (2s)...")
    time.sleep(2)
    ser.reset_input_buffer()

    stop_event = threading.Event()
    reader = threading.Thread(target=read_loop, args=(ser, stop_event), daemon=True)
    pinger = threading.Thread(target=ping_loop, args=(ser, stop_event), daemon=True)
    reader.start()
    pinger.start()

    time.sleep(1.5)  # leer banner

    print("\n" + "=" * 50)
    print("  MOTORES — LLC firmware principal (EXP mode)")
    print("=" * 50)
    print("  W/S → adelante/atrás   A/D → giro   X → stop")
    print("  R → reset FAULT   N → velocidad simétrica   L,R → diferencial")
    print("  q → salir")
    print("=" * 50)

    while True:
        try:
            raw = input("\n  > ")
        except (EOFError, KeyboardInterrupt):
            break

        result = parse_input(raw)
        if result == "quit":
            send_exp(ser, 0, 0)
            print("  → EXP:0:0  (stop)")
            break
        if result == "noop":
            continue
        if result is None:
            print("  ? Usa W/S/A/D/X/R/q, un número o L,R")
            continue
        if result == "rst":
            print("  → RST")
            ser.write(b"RST\n")
            continue
        l, r = result
        print(f"  → EXP:{l}:{r}")
        send_exp(ser, l, r)

    stop_event.set()
    ser.close()
    print("  Puerto cerrado.")


if __name__ == "__main__":
    main()
