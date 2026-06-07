#!/usr/bin/env python3
"""
capturar_bateria.py
--------------------
Captura el voltaje de los DOS bancos desde el 2º Arduino Mega de medición
(firmware `examples/medir_bateria.rs`, SOLO Mega + 2 divisores) por Serial y lo
guarda en CSV con timestamp real. Sirve para medir la AUTONOMÍA por banco.

El firmware emite, a 9600 baud, una cabecera y luego filas:
    tiempo_s,banco1_V,banco2_V
    (banco1 = lógica/A0, banco2 = motores/A1)

Uso:
    pip install pyserial
    python capturar_bateria.py [PUERTO]
"""

import serial
import serial.tools.list_ports
import csv
import time
import os
import sys
from datetime import datetime

# ── Configuración ──────────────────────────────────────────────
PUERTO      = None        # None = autodetección, o "/dev/ttyACM0" / "COM3"
BAUDRATE    = 9600        # debe coincidir con medir_bateria.rs
ARCHIVO_CSV = "descarga_bateria.csv"
VOLTAJE_MIN = 11.2        # corte BMS aprox (4 × 2.8 V)
VOLTAJE_MAX = 16.8        # carga completa (4 × 4.2 V)
# ───────────────────────────────────────────────────────────────


def detectar_puerto():
    puertos = list(serial.tools.list_ports.comports())
    for p in puertos:
        desc = (p.description or "").lower()
        if any(x in desc for x in ["arduino", "ch340", "ch343", "cp210", "ftdi", "uart", "usb serial", "acm"]):
            return p.device
    return puertos[0].device if puertos else None


def porcentaje(v):
    if v <= 0:
        return 0.0
    pct = (v - VOLTAJE_MIN) / (VOLTAJE_MAX - VOLTAJE_MIN) * 100.0
    return max(0.0, min(100.0, pct))


def barra(pct, ancho=12):
    lleno = int(pct / 100 * ancho)
    return "[" + "#" * lleno + "." * (ancho - lleno) + "]"


def main():
    puerto = (sys.argv[1] if len(sys.argv) > 1 else None) or PUERTO or detectar_puerto()
    if not puerto:
        print("X No se encontró ningún puerto Serial. Pasalo como argumento.")
        sys.exit(1)

    print(f"> Puerto      : {puerto}")
    print(f"> Guardando en: {os.path.abspath(ARCHIVO_CSV)}")
    print("> Ctrl+C para detener\n")

    try:
        ser = serial.Serial(puerto, BAUDRATE, timeout=2)
    except serial.SerialException as e:
        print(f"X No se pudo abrir {puerto}: {e}")
        sys.exit(1)

    time.sleep(2)            # el Mega se resetea al abrir el puerto
    ser.reset_input_buffer()

    nuevo = not os.path.exists(ARCHIVO_CSV)
    with open(ARCHIVO_CSV, "a", newline="", encoding="utf-8") as f:
        writer = csv.writer(f)
        if nuevo:
            writer.writerow(["timestamp", "tiempo_s", "banco1_V", "banco2_V",
                             "pct_b1", "pct_b2"])
            f.flush()

        print(f"{'TIMESTAMP':<20} {'T(s)':>5} {'BANCO1':>8} {'BANCO2':>8}  CARGA(B1) CARGA(B2)")
        print("-" * 78)

        muestras = errores = 0
        try:
            while True:
                linea = ser.readline().decode("utf-8", errors="ignore").strip()
                if not linea or linea.startswith("#") or linea.startswith("tiempo"):
                    continue
                partes = linea.split(",")
                if len(partes) < 3:
                    errores += 1
                    continue
                try:
                    t_s = int(partes[0])
                    b1 = float(partes[1])
                    b2 = float(partes[2])
                except ValueError:
                    errores += 1
                    continue
                if not (0.0 <= b1 <= 25.0 and 0.0 <= b2 <= 25.0):
                    errores += 1
                    continue

                ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                p1, p2 = porcentaje(b1), porcentaje(b2)
                writer.writerow([ts, t_s, f"{b1:.3f}", f"{b2:.3f}",
                                 f"{p1:.1f}", f"{p2:.1f}"])
                f.flush()
                muestras += 1
                print(f"{ts:<20} {t_s:>5} {b1:>7.3f}V {b2:>7.3f}V  "
                      f"{barra(p1)} {barra(p2)}")
        except KeyboardInterrupt:
            print(f"\n> Detenido. Muestras: {muestras} | errores: {errores}")
            print(f"> CSV: {os.path.abspath(ARCHIVO_CSV)}")

    ser.close()


if __name__ == "__main__":
    main()
