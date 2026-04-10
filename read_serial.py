#!/usr/bin/env python3
import serial
import sys


def main():
    port = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyACM0"
    baudrate = 115200

    try:
        with serial.Serial(port, baudrate, timeout=1) as ser:
            print(f"--- Escuchando en {port} a {baudrate} baudios ---")
            print("Presiona Ctrl+C para detener.")
            while True:
                line = ser.readline().decode("utf-8", errors="ignore").strip()
                if line:
                    print(line)
    except serial.SerialException as e:
        print(f"Error al abrir el puerto: {e}")
        sys.exit(1)
    except KeyboardInterrupt:
        print("\nDetenido por el usuario.")


if __name__ == "__main__":
    main()
