<!-- Version: v1.0 -->
# Rover Low-Level Controller (Rust)

A high-performance, modular firmware for a multi-terrain rover, implemented in **embedded Rust** for the **ATmega2560** (Arduino Mega). This project serves as the hardware abstraction layer (HAL), providing low-level execution for a **Raspberry Pi 5** (running Yocto Linux) through a dedicated GPIO UART communication bridge.

## 📂 Estructura del Proyecto
*   `src/lib.rs`: Punto de entrada de la librería compartida.
*   `src/motor_control/`: Drivers para L298N, BTS7960 y Servos.
*   `src/command_interface/`: Gestión de buffer y protocolo serial (USART).
*   `examples/`: Programas de prueba funcionales listos para hardware.
*   `tests/`: Validaciones de lógica ejecutables en PC.

## Compilación Segura (Dry Run)

Para verificar que el código es correcto y compila sin errores antes de flashear el hardware, utiliza el siguiente comando. Este comando recompila la librería estándar (`core`) para asegurar compatibilidad total con el ATmega2560.

**Compilar Todo el Proyecto:**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --target avr-none -Z build-std=core
```

**Compilar Ejemplo de 6 Motores :**
```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build --example control_6_motors_l298n --target avr-none -Z build-std=core
```

## Comandos de Validación (Flasheo)

**Validar Protocolo por USB (PC):**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example validate_protocol --target avr-none -Z build-std=core
```

**Ejecutar Control Real por GPIO UART:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example control_motor_rpi --target avr-none -Z build-std=core
```

**Probar Ejemplo L298N:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_l298n --target avr-none -Z build-std=core
```

**Probar Ejemplo Servo:**
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --example test_servo --target avr-none -Z build-std=core
```


