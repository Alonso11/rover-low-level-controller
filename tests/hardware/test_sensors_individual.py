#!/usr/bin/env python3
"""
INT-04b — Verificación individual de sensores Olympus Rover LLC
Ejecutar DESPUÉS de INT-04 (flash), ANTES de INT-05 (protocolo).

Uso:
    python3 tests/hardware/test_sensors_individual.py [puerto]
    python3 tests/hardware/test_sensors_individual.py /dev/ttyUSB0

El rover debe estar en STB (sin movimiento) durante todo el test.
"""

import sys
import serial
import time
import re
import statistics

PORT = sys.argv[1] if len(sys.argv) > 1 else "/dev/ttyUSB0"
BAUD = 115200
TLM_SAMPLES = 5
TIMEOUT_S = 5.0

# Rangos válidos para cada campo TLM en estado STB sin carga
# (min, max, descripcion, sensor_fisico)
SENSOR_RANGES = {
    "batt_mv":      (12000, 16800, "Tensión batería 4S",        "INA226 I2C 0x40"),
    "batt_ma":      (-200,    500, "Corriente total (reposo)",   "INA226 I2C 0x40"),
    "i0":           (-200,    200, "Corriente motor FL",         "ACS712 ADC A8"),
    "i1":           (-200,    200, "Corriente motor FR",         "ACS712 ADC A9"),
    "i2":           (-200,    200, "Corriente motor CL",         "ACS712 ADC A10"),
    "i3":           (-200,    200, "Corriente motor CR",         "ACS712 ADC A11"),
    "i4":           (-200,    200, "Corriente motor RL",         "ACS712 ADC A12"),
    "i5":           (-200,    200, "Corriente motor RR",         "ACS712 ADC A13"),
    "temp_c":       (15,       40, "Temperatura ambiente",       "LM335 ADC A0"),
    "batt_t0":      (15,       45, "Temperatura NTC celda 0",    "NTC ADC A1"),
    "batt_t1":      (15,       45, "Temperatura NTC celda 1",    "NTC ADC A2"),
    "batt_t2":      (15,       45, "Temperatura NTC celda 2",    "NTC ADC A3"),
    "batt_t3":      (15,       45, "Temperatura NTC celda 3",    "NTC ADC A4"),
    "batt_t4":      (15,       45, "Temperatura NTC celda 4",    "NTC ADC A5"),
    "batt_t5":      (15,       45, "Temperatura NTC celda 5",    "NTC ADC A6"),
    "dist_mm":      (1,      8000, "Distancia VL53L0X",          "VL53L0X I2C 0x29"),
}

# Campos que se verifican manualmente (encoders, EKF pose)
MANUAL_CHECKS = [
    ("enc_left",    "Encoders Hall izq (FL+CL+RL)",  "Girar ruedas izq a mano → debe cambiar"),
    ("enc_right",   "Encoders Hall der (FR+CR+RR)",  "Girar ruedas der a mano → debe cambiar"),
    ("theta_mrad",  "MPU-6050 via EKF",              "Debe ser ≠ 0 si hay vibración, o estable ≠ basura"),
]

# Campos que no se toleran como 65535 (sensor timeout I2C)
I2C_TIMEOUT_SENTINEL = {
    "dist_mm": 65535,
}

# Regex para parsear el frame TLM v2.12:
# TLM:MODE:STALL:TICKms:BATmV:BATmA:i0:i1:i2:i3:i4:i5:TEMP:bt0:bt1:bt2:bt3:bt4:bt5:DIST:EL:ER:X:Y:TH
TLM_RE = re.compile(
    r"TLM:"
    r"(\w+):"           # 1 mode
    r"([01]{6}):"       # 2 stall_mask
    r"(\d+)ms:"         # 3 tick_ms
    r"(\d+)mV:"         # 4 batt_mv
    r"(-?\d+)mA:"       # 5 batt_ma
    r"(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):"  # 6-11 i0..i5
    r"(-?\d+)C:"        # 12 temp_c
    r"(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+):(-?\d+)C:"  # 13-18 batt_t0..5
    r"(\d+)mm:"         # 19 dist_mm
    r"(-?\d+):(-?\d+):" # 20-21 enc_left enc_right
    r"(-?\d+):(-?\d+):(-?\d+)"  # 22-24 x_mm y_mm theta_mrad
)


def parse_tlm(line: str) -> dict | None:
    m = TLM_RE.search(line)
    if not m:
        return None
    g = m.groups()
    return {
        "mode":       g[0],
        "stall_mask": g[1],
        "tick_ms":    int(g[2]),
        "batt_mv":    int(g[3]),
        "batt_ma":    int(g[4]),
        "i0":         int(g[5]),
        "i1":         int(g[6]),
        "i2":         int(g[7]),
        "i3":         int(g[8]),
        "i4":         int(g[9]),
        "i5":         int(g[10]),
        "temp_c":     int(g[11]),
        "batt_t0":    int(g[12]),
        "batt_t1":    int(g[13]),
        "batt_t2":    int(g[14]),
        "batt_t3":    int(g[15]),
        "batt_t4":    int(g[16]),
        "batt_t5":    int(g[17]),
        "dist_mm":    int(g[18]),
        "enc_left":   int(g[19]),
        "enc_right":  int(g[20]),
        "x_mm":       int(g[21]),
        "y_mm":       int(g[22]),
        "theta_mrad": int(g[23]),
    }


def collect_tlm_samples(ser: serial.Serial, n: int) -> list[dict]:
    samples = []
    deadline = time.time() + TIMEOUT_S * n
    while len(samples) < n and time.time() < deadline:
        try:
            raw = ser.readline().decode("ascii", errors="replace").strip()
        except Exception:
            continue
        frame = parse_tlm(raw)
        if frame:
            samples.append(frame)
            print(f"  [TLM {len(samples)}/{n}] mode={frame['mode']} batt={frame['batt_mv']}mV dist={frame['dist_mm']}mm")
    return samples


def average_field(samples: list[dict], field: str) -> float:
    vals = [s[field] for s in samples]
    return statistics.mean(vals)


def run():
    print(f"\n=== INT-04b — Verificación individual de sensores ===")
    print(f"Puerto: {PORT}  Baud: {BAUD}  Muestras TLM: {TLM_SAMPLES}\n")

    try:
        ser = serial.Serial(PORT, BAUD, timeout=2.0)
    except serial.SerialException as e:
        print(f"ERROR: No se puede abrir {PORT}: {e}")
        sys.exit(1)

    time.sleep(2.0)  # esperar boot del Mega

    # Poner en STB (seguro: sin movimiento)
    print(">> Enviando STB para asegurar reposo...")
    ser.write(b"STB\n")
    time.sleep(0.5)

    # Vaciar buffer
    ser.reset_input_buffer()

    # Recolectar frames TLM
    print(f">> Recolectando {TLM_SAMPLES} frames TLM...\n")
    samples = collect_tlm_samples(ser, TLM_SAMPLES)

    if len(samples) < TLM_SAMPLES:
        print(f"\nERROR: Solo se recibieron {len(samples)}/{TLM_SAMPLES} frames TLM.")
        print("Verifica que el firmware esté corriendo y el banner se haya mostrado.")
        ser.close()
        sys.exit(1)

    ser.close()

    # Verificar rangos automáticamente
    print(f"\n{'='*65}")
    print(f"{'Sensor':<20} {'Campo':<12} {'Promedio':>10}  {'Rango':<18} {'Result'}")
    print(f"{'='*65}")

    passed = 0
    failed = 0
    failures = []

    for field, (lo, hi, desc, hw) in SENSOR_RANGES.items():
        avg = average_field(samples, field)

        # Verificar sentinel de I2C timeout
        sentinel = I2C_TIMEOUT_SENTINEL.get(field)
        if sentinel is not None and any(s[field] == sentinel for s in samples):
            result = "FAIL (I2C timeout)"
            failed += 1
            failures.append((field, hw, f"Valor {sentinel} indica timeout I2C — sensor no responde"))
        elif lo <= avg <= hi:
            result = "PASS"
            passed += 1
        else:
            result = f"FAIL ({avg:.0f} fuera de [{lo},{hi}])"
            failed += 1
            failures.append((field, hw, f"Promedio {avg:.1f} fuera de [{lo},{hi}]"))

        print(f"{desc:<20} {field:<12} {avg:>10.1f}  [{lo:>6},{hi:<6}]  {result}")

    # Verificaciones manuales (encoders, EKF)
    print(f"\n{'='*65}")
    print("VERIFICACIONES MANUALES (requieren acción física):")
    print(f"{'='*65}")
    for field, desc, instruccion in MANUAL_CHECKS:
        vals = [s[field] for s in samples]
        print(f"  {desc} ({field}): valores={vals}")
        print(f"    → {instruccion}")

    # Resumen
    total_auto = len(SENSOR_RANGES)
    print(f"\n{'='*65}")
    print(f"Resultado automático: {passed}/{total_auto} sensores en rango")

    if failures:
        print("\nFALLOS DETECTADOS:")
        for field, hw, msg in failures:
            print(f"  ✗ {field} [{hw}]: {msg}")
        print("\nNo proceder a INT-05 hasta resolver los fallos.")
        sys.exit(1)
    else:
        print("\nTodos los sensores en rango.")
        print("Verificar manualmente encoders y EKF (ver tabla arriba).")
        print("Si encoders responden a movimiento manual → proceder a INT-05.")
        sys.exit(0)


if __name__ == "__main__":
    run()
