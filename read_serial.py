import serial
import sys

port = '/dev/ttyACM0'
baudrate = 115200

try:
    with serial.Serial(port, baudrate, timeout=1) as ser:
        print(f"--- Escuchando en {port} a {baudrate} baudios ---")
        print("Presiona Ctrl+C para detener.")
        while True:
            line = ser.readline().decode('utf-8', errors='ignore').strip()
            if line:
                print(line)
except serial.SerialException as e:
    print(f"Error al abrir el puerto: {e}")
except KeyboardInterrupt:
    print("\nDetenido por el usuario.")
