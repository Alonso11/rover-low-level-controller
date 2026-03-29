#!/usr/bin/env python3
"""
test_motors_debug.py — Test local de motores L298N
Conecta al Arduino por serial y envía comandos F/B/S al ejemplo debug_motors_l298n.

Uso:
    python3 tests/test_motors_debug.py [puerto]
    python3 tests/test_motors_debug.py /dev/ttyUSB0
    python3 tests/test_motors_debug.py /dev/ttyACM0   (si no funciona ttyUSB0)

Recuerda: flashea primero con MOTOR_TO_TEST=N en debug_motors_l298n.rs
"""

import serial
import serial.tools.list_ports
import sys
import threading
import time

BAUD = 115200
DEFAULT_PORT = "/dev/ttyUSB0"

# Mapa de motores para mostrar info
MOTOR_INFO = {
    1: "D9  / OC2B / Timer2 — Frontal Derecho",
    2: "D10 / OC2A / Timer2 — Frontal Izquierdo",
    3: "D5  / OC3A / Timer3 — Central Derecho",
    4: "D6  / OC4A / Timer4 — Central Izquierdo",
    5: "D7  / OC4B / Timer4 — Trasero Derecho",
    6: "D8  / OC4C / Timer4 — Trasero Izquierdo",
}


def list_ports():
    ports = serial.tools.list_ports.comports()
    if not ports:
        print("  (ningún puerto serie detectado)")
    for p in ports:
        print(f"  {p.device:20s} — {p.description}")


def read_loop(ser, stop_event):
    """Hilo que imprime todo lo que manda el Arduino."""
    while not stop_event.is_set():
        try:
            if ser.in_waiting:
                line = ser.readline().decode("utf-8", errors="replace").strip()
                if line:
                    print(f"  [Arduino] {line}")
        except Exception:
            break
        time.sleep(0.02)


def send_cmd(ser, cmd: str):
    ser.write((cmd + "\n").encode())
    time.sleep(0.15)  # espera respuesta antes del próximo prompt


def interactive_menu(ser):
    print("\n" + "="*55)
    print("  CONTROL MOTORES — debug_motors_l298n")
    print("="*55)
    print("  El motor activo depende de MOTOR_TO_TEST en el .rs")
    print()
    print("  Comandos:")
    print("    f / F   → adelante (speed=80)")
    print("    b / B   → atrás   (speed=-80)")
    print("    s / S   → stop")
    print("    q       → salir")
    print()
    print("  Referencia de motores:")
    for n, info in MOTOR_INFO.items():
        print(f"    M{n} — {info}")
    print("="*55)

    while True:
        try:
            cmd = input("\n  > ").strip().upper()
        except (EOFError, KeyboardInterrupt):
            print("\n  Saliendo...")
            break

        if cmd == "Q":
            print("  Saliendo...")
            break
        elif cmd in ("F", "B", "S"):
            names = {"F": "ADELANTE", "B": "ATRÁS", "S": "STOP"}
            print(f"  → Enviando: {cmd} ({names[cmd]})")
            send_cmd(ser, cmd)
        elif cmd == "":
            continue
        else:
            print("  Comandos válidos: F  B  S  q")


def main():
    port = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_PORT

    print(f"\nPuertos serie disponibles:")
    list_ports()
    print(f"\nConectando a {port} @ {BAUD} baud...")

    try:
        ser = serial.Serial(port, BAUD, timeout=1)
    except serial.SerialException as e:
        print(f"\nERROR: no se pudo abrir {port}")
        print(f"  {e}")
        print("\nPrueba con otro puerto, por ejemplo:")
        print("  python3 tests/test_motors_debug.py /dev/ttyACM0")
        sys.exit(1)

    # El DTR reset del Arduino tarda ~2 seg
    print("Esperando reset del Arduino (2s)...")
    time.sleep(2)
    ser.reset_input_buffer()

    stop_event = threading.Event()
    reader = threading.Thread(target=read_loop, args=(ser, stop_event), daemon=True)
    reader.start()

    # Leer el banner de inicio que manda el Arduino (espera suficiente)
    print("Leyendo banner de inicio...")
    time.sleep(1.5)

    try:
        interactive_menu(ser)
    finally:
        stop_event.set()
        ser.close()
        print("  Puerto cerrado. Fin.")


if __name__ == "__main__":
    main()
