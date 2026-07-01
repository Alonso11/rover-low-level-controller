#!/usr/bin/env python3
"""
Envía un comando especial al firmware Olympus LLC para que reporte
los dispositivos I2C detectados en el bus (0x29, 0x40, 0x68).

Uso:
    python3 tests/hardware/i2c_scan.py [puerto]

El firmware debe estar en modo TEST (USART0) y tener el comando
I2C_SCAN implementado (ver src/diagnostics.rs).

Si el firmware no lo implementa, usar directamente i2cdetect
desde una Raspberry Pi con acceso físico al bus:
    i2cdetect -y 1
Dispositivos esperados:
    0x29 — VL53L0X (sensor de distancia)
    0x40 — INA226  (monitor de potencia)
    0x68 — MPU-6050 (IMU acelerómetro + giroscopio)
"""

import sys
import serial
import time

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyUSB0"
BAUD = 115200

EXPECTED = {
    0x29: "VL53L0X  (distancia)",
    0x40: "INA226   (potencia EPS)",
    0x68: "MPU-6050 (IMU EKF)",
}

def run():
    print(f"\n=== Escaneo I2C via firmware Olympus LLC ({PORT}) ===\n")
    try:
        ser = serial.Serial(PORT, BAUD, timeout=3.0)
    except serial.SerialException as e:
        print(f"ERROR: {e}")
        print("\nAlternativa si el firmware no soporta I2C_SCAN:")
        print("  ssh root@<RPi5-IP> i2cdetect -y 1")
        sys.exit(1)

    time.sleep(1.5)
    ser.reset_input_buffer()

    ser.write(b"I2C_SCAN\n")
    time.sleep(1.0)

    lines = []
    while ser.in_waiting:
        line = ser.readline().decode("ascii", errors="replace").strip()
        if line:
            lines.append(line)
    ser.close()

    if not lines:
        print("No se recibió respuesta al comando I2C_SCAN.")
        print("El firmware puede no tener este comando implementado.")
        print("\nUsar alternativamente desde RPi5:")
        print("  i2cdetect -y 1")
        print("\nDispositivos esperados:")
        for addr, desc in EXPECTED.items():
            print(f"  0x{addr:02X} — {desc}")
        sys.exit(1)

    # Parsear respuesta tipo "I2C:0x29,0x40,0x68" o similar
    found = []
    for line in lines:
        if "I2C:" in line:
            parts = line.split("I2C:")[1].split(",")
            for p in parts:
                try:
                    found.append(int(p.strip(), 16))
                except ValueError:
                    pass

    print(f"{'Dirección':<12} {'Dispositivo':<25} {'Estado'}")
    print("-" * 50)
    all_ok = True
    for addr, desc in EXPECTED.items():
        status = "DETECTADO" if addr in found else "NO DETECTADO"
        if addr not in found:
            all_ok = False
        print(f"  0x{addr:02X}      {desc:<25} {status}")

    extra = [a for a in found if a not in EXPECTED]
    for addr in extra:
        print(f"  0x{addr:02X}      (desconocido)             DETECTADO")

    print()
    if all_ok:
        print("Resultado: todos los dispositivos I2C detectados.")
    else:
        print("ERROR: uno o más dispositivos I2C no responden.")
        print("Verificar cableado SDA/SCL y pull-ups 4.7 kΩ a 3.3V.")
        sys.exit(1)

if __name__ == "__main__":
    run()
