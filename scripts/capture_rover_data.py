#!/usr/bin/env python3
# capture_rover_data.py — Data logger para calibración de odometría
#
# Uso: python3 capture_rover_data.py [puerto_serie]
# Requisitos: pip install pyserial

import serial
import serial.tools.list_ports
import csv
import time
import sys
from datetime import datetime

# Configuración por defecto
BAUD_RATE = 115200

def find_arduino_port():
    ports = serial.tools.list_ports.comports()
    for port in ports:
        # Busca descriptores comunes de Arduino o conversores USB-Serial
        if any(key in port.description for key in ["Arduino", "USB Serial", "CH340", "FT232"]):
            return port.device
    return None

def main():
    # Selección de puerto
    port = sys.argv[1] if len(sys.argv) > 1 else find_arduino_port()
    
    if not port:
        print("Error: No se encontró ningún dispositivo USB Serial.")
        print("Puertos disponibles:")
        for p in serial.tools.list_ports.comports():
            print(f"  - {p.device}: {p.description}")
        return

    filename = f"rover_capture_{datetime.now().strftime('%Y%m%d_%H%M%S')}.csv"
    print(f"Conectando a {port} a {BAUD_RATE} baudios...")
    print(f"Guardando datos en {filename}...")
    
    try:
        with serial.Serial(port, BAUD_RATE, timeout=1) as ser, \
             open(filename, mode='w', newline='') as csv_file:
            
            writer = csv.writer(csv_file)
            header_written = False
            
            print("Capturando... Presiona Ctrl+C para finalizar.")
            
            while True:
                line = ser.readline().decode('ascii', errors='replace').strip()
                if not line:
                    continue
                
                # Procesar tramas de telemetría (TLM)
                if line.startswith("TLM:"):
                    parts = line.split(":")
                    if not header_written:
                        header = ["local_ts", "safety", "stall_mask", "tick", "batt_mv", "batt_ma"] + \
                                 [f"curr_{i}" for i in range(6)] + ["temp_c"] + \
                                 [f"batt_t_{i}" for i in range(6)] + ["dist_mm", "enc_l", "enc_r", "x_mm", "y_mm", "theta_mrad"]
                        writer.writerow(header)
                        header_written = True
                    
                    # Limpiar sufijos 'ms', 'mV', etc.
                    clean_data = []
                    for p in parts[1:]:
                        clean_p = p.replace('ms', '').replace('mV', '').replace('mA', '').replace('C', '').replace('mm', '')
                        clean_data.append(clean_p)
                    
                    writer.writerow([time.time()] + clean_data)

                # Procesar tramas de alta frecuencia (RAW)
                elif line.startswith("RAW:"):
                    parts = line.split(":")
                    if not header_written:
                        header = ["local_ts", "tick", "ax", "ay", "az", "gx", "gy", "gz", "enc_l", "enc_r"]
                        writer.writerow(header)
                        header_written = True
                    
                    writer.writerow([time.time()] + parts[1:])

    except KeyboardInterrupt:
        print(f"\nCaptura finalizada. Datos guardados en {filename}")
    except Exception as e:
        print(f"\nError durante la captura: {e}")

if __name__ == "__main__":
    main()
