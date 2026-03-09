# Rover Low-Level Controller (Arduino Mega 2560)

Este es el firmware de bajo nivel para el rover, implementado en **Rust** para garantizar seguridad de memoria y concurrencia robusta.

## 🛠 Requisitos de Sistema

- **Rust Nightly**: Necesario para el soporte de arquitectura AVR.
- **ravedude**: Herramienta para flashear y monitorizar el Arduino.
  ```bash
  cargo install ravedude
  ```
- **avr-gcc**: Linker necesario para la compilación.

## 🚀 Compilación y Carga

Para compilar y subir el código al Arduino Mega (conectado vía USB):

```bash
RUSTFLAGS="-C target-cpu=atmega2560" cargo run -Z build-std=core -Z json-target-spec --target avr-atmega2560.json
```

## 🔌 Interfaz de Comunicación (SBC <-> Arduino)

La comunicación se realiza vía Serial a **115,200 baudios**.

### Protocolo de Comandos:
- `M`: **Move** - Mueve el rover adelante.
- `S`: **Stop** - Detiene todos los motores.
- `D`: **Distance** - Lee el sensor ultrasónico (Trig: D4, Echo: D5).

## 🧩 Arquitectura SOLID

El proyecto sigue una estructura modular diseñada para ser expandible:
- `src/motor_control`: Abstracción de motores y drivers.
- `src/sensors`: Gestión de sensores de distancia y estado.
- `src/command_interface`: Puente de comandos con la Raspberry Pi 5.

## ⚠️ Notas de Compatibilidad (AVR-Rust)

Debido a las limitaciones del backend de AVR en Rust:
1. Se requiere el flag `-Z build-std=core` para recompilar la librería core para 8-bit.
2. Los tipos de PWM son altamente dependientes del Timer utilizado (en este caso **Timer 3** para pines D2 y D3).
3. Se recomienda usar tipos concretos en lugar de Traits genéricos excesivamente complejos para evitar el "bloat" del binario.
