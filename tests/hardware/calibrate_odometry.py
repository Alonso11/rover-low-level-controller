#!/usr/bin/env python3
"""
calibrate_odometry.py — Calibración guiada de las constantes de odometría del LLC.

Mide TICKS_PER_REV, WHEEL_RADIUS_MM y WHEEL_BASE_MM directamente sobre el
hardware físico y genera el bloque de constantes listo para pegar en config.rs.

Prerrequisitos:
  - Arduino Mega flasheado con el firmware LLC.
  - Rover sobre soporte (ruedas elevadas) para TICKS_PER_REV y WHEEL_BASE_MM.
  - Cinta métrica y calibrador digital disponibles.
  - pyserial instalado: pip install pyserial

Uso:
  python3 calibrate_odometry.py /dev/ttyUSB0

Salida:
  Imprime el bloque de constantes y escribe calibration_result.txt con
  los valores medidos y el error relativo de validación (RF-004-R1).

Referencia: verificacion.tex §subsec:vv_odometria
"""

import sys
import time
import statistics
import serial


# ─── Parámetros ──────────────────────────────────────────────────────────────

BAUD        = 115200
TIMEOUT_S   = 3.0
REPS        = 5       # repeticiones por medición (promedio)
PING_TRIES  = 5


# ─── UART helpers ─────────────────────────────────────────────────────────────

def connect(port: str) -> serial.Serial:
    ser = serial.Serial(port, BAUD, timeout=TIMEOUT_S)
    time.sleep(2.0)  # esperar reset del Arduino tras abrir puerto
    ser.reset_input_buffer()
    return ser


def send(ser: serial.Serial, cmd: str) -> str:
    ser.write((cmd + "\n").encode())
    ser.flush()
    line = ser.readline().decode(errors="replace").strip()
    return line


def ping(ser: serial.Serial) -> bool:
    for _ in range(PING_TRIES):
        if send(ser, "PING") == "PONG":
            return True
        time.sleep(0.2)
    return False


def read_tlm(ser: serial.Serial, timeout_s: float = 3.0) -> dict | None:
    """Lee el próximo frame TLM y retorna un dict con enc_left y enc_right."""
    deadline = time.monotonic() + timeout_s
    while time.monotonic() < deadline:
        line = ser.readline().decode(errors="replace").strip()
        if not line.startswith("TLM:"):
            continue
        # TLM:NORMAL:000000:<ts>ms:<mv>mV:<ma>mA:c0:c1:c2:c3:c4:c5:<t>C:...:<dist>mm:<EL>:<ER>:<x>:<y>:<theta>
        parts = line.split(":")
        try:
            # Índices según format_tlm en state_machine/mod.rs:
            # 0=TLM 1=safety 2=stall 3=ts 4=mv 5=ma 6-11=currents 12=temp 13-18=batt_temps 19=dist 20=enc_left 21=enc_right 22=x 23=y 24=theta
            enc_left  = int(parts[20])
            enc_right = int(parts[21])
            return {"enc_left": enc_left, "enc_right": enc_right}
        except (IndexError, ValueError):
            continue
    return None


# ─── Paso 1: TICKS_PER_REV ───────────────────────────────────────────────────

def measure_ticks_per_rev(ser: serial.Serial) -> int:
    print("\n" + "="*60)
    print("PASO 1 — Medición de TICKS_PER_REV")
    print("="*60)
    print("El rover debe estar sobre un soporte con las ruedas elevadas.")
    print("Se medirá el número de pulsos por vuelta completa del encoder.")
    print()
    print("Instrucciones:")
    print("  1. Marque un punto de referencia en la rueda delantera izquierda.")
    print("  2. Cuando el script lo indique, gire la rueda UNA vuelta completa")
    print("     lentamente (sentido horario visto desde el exterior).")
    print("  3. Repita", REPS, "veces.")
    input("\nPresione ENTER cuando esté listo...")

    samples = []
    for i in range(REPS):
        # Leer enc_left antes
        send(ser, "STB")
        time.sleep(0.1)
        t0 = read_tlm(ser)
        if t0 is None:
            print(f"  [!] No se recibió TLM en intento {i+1}. Reintentando...")
            continue

        input(f"  [{i+1}/{REPS}] Gire UNA vuelta completa y presione ENTER al terminar...")

        t1 = read_tlm(ser)
        if t1 is None:
            print("  [!] No se recibió TLM tras la vuelta. Se omite.")
            continue

        delta = abs(t1["enc_left"] - t0["enc_left"])
        samples.append(delta)
        print(f"         Ticks medidos: {delta}")

    if not samples:
        print("[ERROR] No se obtuvieron muestras válidas.")
        sys.exit(1)

    ticks_per_rev = round(statistics.mean(samples))
    stddev = statistics.stdev(samples) if len(samples) > 1 else 0.0
    print(f"\n  Media   : {statistics.mean(samples):.1f} ticks")
    print(f"  Desv.est: {stddev:.1f} ticks")
    print(f"  → TICKS_PER_REV = {ticks_per_rev}")
    return ticks_per_rev


# ─── Paso 2: WHEEL_RADIUS_MM ─────────────────────────────────────────────────

def measure_wheel_radius() -> int:
    print("\n" + "="*60)
    print("PASO 2 — Medición de WHEEL_RADIUS_MM")
    print("="*60)
    print("Instrucciones:")
    print("  1. Coloque la rueda sobre una superficie plana.")
    print("  2. Mida el DIÁMETRO exterior con un calibrador digital (en mm).")
    print("  3. Repita en las 6 ruedas y registre todos los valores.")

    diameters = []
    for i in range(6):
        label = ["FR", "FL", "CR", "CL", "RR", "RL"][i]
        raw = input(f"  Diámetro rueda {label} (mm): ").strip()
        try:
            diameters.append(float(raw))
        except ValueError:
            print("  [!] Valor inválido, se omite.")

    if not diameters:
        print("[ERROR] No se ingresaron diámetros.")
        sys.exit(1)

    median_diam = statistics.median(diameters)
    radius = median_diam / 2.0
    radius_int = round(radius)
    print(f"\n  Mediana diámetros : {median_diam:.1f} mm")
    print(f"  Radio (mediana/2) : {radius:.1f} mm")
    print(f"  → WHEEL_RADIUS_MM = {radius_int}")
    return radius_int


# ─── Paso 3: WHEEL_BASE_MM ───────────────────────────────────────────────────

def measure_wheel_base() -> int:
    print("\n" + "="*60)
    print("PASO 3 — Medición de WHEEL_BASE_MM")
    print("="*60)
    print("Instrucciones:")
    print("  1. Coloque el rover en terreno plano.")
    print("  2. Mida la separación entre los centros de contacto de las")
    print("     ruedas izquierda y derecha (track width) con cinta métrica.")
    print("  3. Mida en el eje delantero, central y trasero.")

    measurements = []
    for label in ["Eje delantero (FR-FL)", "Eje central (CR-CL)", "Eje trasero (RR-RL)"]:
        raw = input(f"  {label} (mm): ").strip()
        try:
            measurements.append(float(raw))
        except ValueError:
            print("  [!] Valor inválido, se omite.")

    if not measurements:
        print("[ERROR] No se ingresaron mediciones.")
        sys.exit(1)

    mean_base = statistics.mean(measurements)
    base_int  = round(mean_base)
    print(f"\n  Media track width: {mean_base:.1f} mm")
    print(f"  → WHEEL_BASE_MM = {base_int}")
    return base_int


# ─── Paso 4: Validación de RF-004-R1 ─────────────────────────────────────────

def validate_rf004(ser: serial.Serial, ticks: int, radius: int) -> None:
    print("\n" + "="*60)
    print("PASO 4 — Validación RF-004-R1 (error < 5 %)")
    print("="*60)
    print("Instrucciones:")
    print("  1. Marque una línea de inicio y una de fin separadas por una")
    print("     distancia CONOCIDA (recomendado: 2000 mm = 2 m).")
    print("  2. Coloque el rover con la rueda delantera sobre la línea de inicio.")
    print("  3. Envíe EXP:40:40 desde el GCS o manualmente para avanzar en línea recta.")
    print("  4. Detenga el rover cuando la rueda delantera cruce la línea de fin.")
    print("  5. Lea los valores de enc_left y enc_right del TLM.")

    gt_raw = input("\n  Distancia real medida con cinta (mm): ").strip()
    try:
        ground_truth_mm = float(gt_raw)
    except ValueError:
        print("  [!] Valor inválido. Salteando validación.")
        return

    enc_start = read_tlm(ser, timeout_s=5.0)
    if enc_start is None:
        print("  [!] No se recibió TLM inicial. Salteando validación.")
        return
    input("  Realice el recorrido y presione ENTER al llegar al fin...")
    enc_end = read_tlm(ser, timeout_s=5.0)
    if enc_end is None:
        print("  [!] No se recibió TLM final. Salteando validación.")
        return

    # Calcular distancia estimada usando los valores de calibración
    import math
    ticks_l = abs(enc_end["enc_left"]  - enc_start["enc_left"])
    ticks_r = abs(enc_end["enc_right"] - enc_start["enc_right"])
    # 3 encoders por lado; ENC_TO_METER = 2*pi*r / (3*ticks_per_rev)
    enc_to_m = (2 * math.pi * (radius / 1000.0)) / (3 * ticks)
    dist_l_mm = ticks_l * enc_to_m * 1000.0
    dist_r_mm = ticks_r * enc_to_m * 1000.0
    estimated_mm = (dist_l_mm + dist_r_mm) / 2.0

    error_pct = abs(estimated_mm - ground_truth_mm) / ground_truth_mm * 100.0
    status = "PASS ✓" if error_pct < 5.0 else "FAIL ✗"

    print(f"\n  Distancia estimada : {estimated_mm:.1f} mm")
    print(f"  Ground truth       : {ground_truth_mm:.1f} mm")
    print(f"  Error relativo     : {error_pct:.2f} %")
    print(f"  RF-004-R1          : {status}  (umbral: < 5 %)")


# ─── Salida: bloque config.rs ─────────────────────────────────────────────────

def print_config_block(ticks: int, radius: int, base: int) -> None:
    print("\n" + "="*60)
    print("RESULTADO — Copiar en src/config.rs")
    print("="*60)
    block = f"""
pub const TICKS_PER_REV:  u32 = {ticks};   // medido: {REPS} muestras
pub const WHEEL_RADIUS_MM: u32 = {radius};  // mediano de 6 ruedas
pub const WHEEL_BASE_MM:  u32 = {base};    // media de 3 ejes
"""
    print(block)

    with open("calibration_result.txt", "w") as f:
        f.write(f"TICKS_PER_REV  = {ticks}\n")
        f.write(f"WHEEL_RADIUS_MM = {radius}\n")
        f.write(f"WHEEL_BASE_MM  = {base}\n")
    print("  Guardado en: calibration_result.txt")


# ─── Main ─────────────────────────────────────────────────────────────────────

def main() -> None:
    if len(sys.argv) < 2:
        print(f"Uso: python3 {sys.argv[0]} <puerto>  (ej: /dev/ttyUSB0)")
        sys.exit(1)

    port = sys.argv[1]
    print(f"\nConectando a {port} @ {BAUD} baud...")
    ser = connect(port)

    if not ping(ser):
        print("[ERROR] El Arduino no responde a PING. ¿Firmware flasheado?")
        ser.close()
        sys.exit(1)
    print("Conexión OK — LLC responde PONG")

    ticks  = measure_ticks_per_rev(ser)
    radius = measure_wheel_radius()
    base   = measure_wheel_base()

    print_config_block(ticks, radius, base)
    validate_rf004(ser, ticks, radius)

    ser.close()
    print("\nCalibración completada.")


if __name__ == "__main__":
    main()
