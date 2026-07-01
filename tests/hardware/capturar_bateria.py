#!/usr/bin/env python3
"""
capturar_bateria.py
--------------------
Captura voltaje de DOS bancos desde 2o Arduino Mega y guarda CSV.
Soporta dos firmwares:
  - medir_bateria (por defecto): emite tiempo_s,V1,V2  (voltaje calculado)
  - medir_bateria_raw (--raw):    emite tiempo_s,raw1,raw2 (ADC 0-1023)

El factor del divisor se pasa con --factor (defecto 3.7).
Calibracion: medir R2 (wiper-GND) con multimetro, factor = (10000+R2)/R2.

Uso:
    pip install pyserial
    python capturar_bateria.py                          # firmware original
    python capturar_bateria.py --raw --factor 3.7       # raw + factor
    python capturar_bateria.py /dev/ttyACM0 --raw

Factores separados por banco:
    python capturar_bateria.py --raw --factor-b1 3.17 --factor-b2 4.69
"""

import argparse
import sys
import serial
import serial.tools.list_ports
import csv
import time
import os
from datetime import datetime

# ── Defaults ──────────────────────────────────────────────
BAUDRATE    = 9600
ARCHIVO_CSV = "descarga_bateria.csv"
VOLTAJE_MIN = 11.2
VOLTAJE_MAX = 16.8
# ──────────────────────────────────────────────────────────


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
    parser = argparse.ArgumentParser(description="Logger de voltaje de baterias desde 2o Mega")
    parser.add_argument("puerto", nargs="?", help="Puerto serie (auto-detectado si se omite)")
    parser.add_argument("--raw", action="store_true", help="El firmware emite ADC crudo (0-1023)")
    parser.add_argument("--factor", type=float, default=None,
                        help="Factor unico para ambos bancos")
    parser.add_argument("--factor-b1", type=float, default=None,
                        help="Factor para banco 1 (A0)")
    parser.add_argument("--factor-b2", type=float, default=None,
                        help="Factor para banco 2 (A1)")
    args = parser.parse_args()

    puerto = args.puerto or detectar_puerto()
    if not puerto:
        print("X No se encontro ningun puerto Serial. Pasalo como argumento.")
        sys.exit(1)

    f1 = args.factor_b1 or args.factor or 3.7
    f2 = args.factor_b2 or args.factor or 3.7
    modo = "RAW" if args.raw else "VOLTS"
    print(f"> Puerto      : {puerto}")
    print(f"> Modo        : {modo}  (factor B1={f1}, B2={f2})")
    print(f"> Guardando en: {os.path.abspath(ARCHIVO_CSV)}")
    print("> Ctrl+C para detener\n")

    try:
        ser = serial.Serial(puerto, BAUDRATE, timeout=2)
    except serial.SerialException as e:
        print(f"X No se pudo abrir {puerto}: {e}")
        sys.exit(1)

    time.sleep(2)
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
                    v1 = float(partes[1])
                    v2 = float(partes[2])
                except ValueError:
                    errores += 1
                    continue

                if args.raw:
                    if not (0 <= v1 <= 1023 and 0 <= v2 <= 1023):
                        errores += 1
                        continue
                    v1 = v1 * 5.0 / 1023 * f1
                    v2 = v2 * 5.0 / 1023 * f2
                else:
                    if not (0.0 <= v1 <= 25.0 and 0.0 <= v2 <= 25.0):
                        errores += 1
                        continue

                ts = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
                p1, p2 = porcentaje(v1), porcentaje(v2)
                writer.writerow([ts, t_s, f"{v1:.3f}", f"{v2:.3f}",
                                 f"{p1:.1f}", f"{p2:.1f}"])
                f.flush()
                muestras += 1
                print(f"{ts:<20} {t_s:>5} {v1:>7.3f}V {v2:>7.3f}V  "
                      f"{barra(p1)} {barra(p2)}")
        except KeyboardInterrupt:
            print(f"\n> Detenido. Muestras: {muestras} | errores: {errores}")
            print(f"> CSV: {os.path.abspath(ARCHIVO_CSV)}")

    ser.close()


if __name__ == "__main__":
    main()
