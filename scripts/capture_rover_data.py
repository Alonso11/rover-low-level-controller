#!/usr/bin/env python3
# capture_rover_data.py — Data logger para calibración EKF/odometría Olympus
#
# Uso:
#   python3 capture_rover_data.py                   # auto-detecta puerto
#   python3 capture_rover_data.py /dev/ttyUSB0      # puerto explícito
#   python3 capture_rover_data.py /dev/ttyUSB0 60   # captura 60 segundos y termina
#
# Genera dos archivos CSV separados:
#   raw_YYYYMMDD_HHMMSS.csv  — tramas RAW:  tick,ax,ay,az,gx,gy,gz,enc_l,enc_r  (50 Hz)
#   tlm_YYYYMMDD_HHMMSS.csv  — tramas TLM:  safety,...,x_mm,y_mm,theta_mrad      (1 Hz)
#
# Requisitos: pip install pyserial

import argparse
import csv
import sys
import time
from datetime import datetime

import serial
import serial.tools.list_ports

BAUD_RATE = 115200

RAW_HEADER = ["local_ts", "tick_ms", "ax_raw", "ay_raw", "az_raw",
              "gx_raw", "gy_raw", "gz_raw", "enc_l", "enc_r"]

TLM_HEADER = (
    ["local_ts", "safety", "stall_mask", "tick_ms", "batt_mv", "batt_ma"]
    + [f"curr_{i}" for i in range(6)]
    + ["temp_c"]
    + [f"batt_t_{i}" for i in range(6)]
    + ["dist_mm", "enc_l", "enc_r", "x_mm", "y_mm", "theta_mrad", "dist_far_mm"]
)


def _strip_units(s: str) -> str:
    for suf in ("ms", "mV", "mA", "mm", "C"):
        s = s.replace(suf, "")
    return s


def find_arduino_port() -> str | None:
    for p in serial.tools.list_ports.comports():
        if any(k in p.description for k in ("Arduino", "USB Serial", "CH340", "FT232", "ttyUSB")):
            return p.device
    return None


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Captura RAW (50 Hz) y TLM (1 Hz) del Arduino Mega para calibración EKF"
    )
    parser.add_argument("port", nargs="?", help="Puerto serie (auto-detectado si se omite)")
    parser.add_argument("duration", nargs="?", type=float, default=0,
                        help="Duración en segundos (0 = continuo hasta Ctrl+C)")
    args = parser.parse_args()

    port = args.port or find_arduino_port()
    if not port:
        print("Error: no se encontró ningún dispositivo USB Serial.")
        for p in serial.tools.list_ports.comports():
            print(f"  {p.device}: {p.description}")
        sys.exit(1)

    stamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    raw_file = f"raw_{stamp}.csv"
    tlm_file = f"tlm_{stamp}.csv"

    print(f"Puerto  : {port}  @  {BAUD_RATE} baud")
    print(f"RAW CSV : {raw_file}  (50 Hz — IMU + encoders)")
    print(f"TLM CSV : {tlm_file}  (1 Hz  — telemetría completa)")
    if args.duration:
        print(f"Duración: {args.duration} s")
    print("Ctrl+C para terminar.\n")

    raw_count = tlm_count = 0
    t_start = time.monotonic()

    try:
        with (serial.Serial(port, BAUD_RATE, timeout=1) as ser,
              open(raw_file, "w", newline="") as rf,
              open(tlm_file, "w", newline="") as tf):

            raw_w = csv.writer(rf)
            tlm_w = csv.writer(tf)
            raw_w.writerow(RAW_HEADER)
            tlm_w.writerow(TLM_HEADER)

            while True:
                if args.duration and (time.monotonic() - t_start) >= args.duration:
                    break

                line = ser.readline().decode("ascii", errors="replace").strip()
                if not line:
                    continue

                ts = time.time()

                if line.startswith("RAW:"):
                    parts = line.split(":")[1:]   # skip "RAW"
                    raw_w.writerow([ts] + parts)
                    raw_count += 1
                    if raw_count % 250 == 0:       # log cada 5 s (250 × 20 ms)
                        print(f"  RAW={raw_count:6d}  TLM={tlm_count:4d}  "
                              f"t={time.monotonic()-t_start:.0f}s")

                elif line.startswith("TLM:"):
                    parts = [_strip_units(p) for p in line.split(":")[1:]]
                    tlm_w.writerow([ts] + parts)
                    tlm_count += 1

    except KeyboardInterrupt:
        pass

    elapsed = time.monotonic() - t_start
    print(f"\nCaptura finalizada — {elapsed:.1f} s")
    print(f"  RAW: {raw_count} muestras → {raw_file}")
    print(f"  TLM: {tlm_count} muestras → {tlm_file}")


if __name__ == "__main__":
    main()
