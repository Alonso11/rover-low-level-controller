<!-- Version: v1.0 -->
# Arquitectura de Control y Sensores (Rover Olympus)

Este documento detalla la arquitectura implementada para el control de un chasis de 6 ruedas y la lectura de encoders utilizando un ATmega2560.

## 1. Abstracción de Motores: `SixWheelRover`

Para gestionar 6 motores individuales, utilizamos la estructura `SixWheelRover` que agrupa instancias del trait `Motor`. Esto permite un control diferencial (tanque) coordinado.

```rust
pub struct SixWheelRover<M1, M2, M3, M4, M5, M6> {
    pub frontal_right: M1,
    pub frontal_left: M2,
    pub center_right: M3,
    pub center_left: M4,
    pub rear_right: M5,
    pub rear_left: M6,
}
```

## 2. Generación de PWM por Hardware

Utilizamos los Timers internos para generar señales PWM sin carga para la CPU. Para habilitar el uso de encoders, los pines se han distribuido de la siguiente manera:

| Componente | Timer | Pines PWM | Pines Dirección |
| :--- | :--- | :--- | :--- |
| **Puente 1 (Frontal)** | Timer 2 | D10, D9 | D22 al D25 |
| **Puente 2 (Central)** | Timer 1 | D11, D12 | D26 al D29 |
| **Puente 3 (Trasero)** | Timer 5 | D46, D45 | D30 al D33 |

*Nota: Esta configuración libera los pines de interrupción externa (D2, D3, D18-D21) para los encoders.*

## 3. Encoders de Efecto Hall

Los encoders miden la rotación de los motores detectando el paso de imanes. Para no perder pulsos, se utilizan **Interrupciones Externas (External Interrupts)**.

### Módulo `sensors::encoder`
- **Trait `Encoder`**: Define una interfaz para leer y resetear contadores.
- **Estructura `HallEncoder`**: Implementación que utiliza un `Mutex` y `Cell` para un acceso seguro desde las ISR (Interrupt Service Routines).

### Distribución de Interrupciones
| Motor | Pin Arduino | Vector ISR |
| :--- | :--- | :--- |
| Frontal Derecho | D21 | INT0 |
| Frontal Izquierdo | D20 | INT1 |
| Central Derecho | D19 | INT2 |
| Central Izquierdo | D18 | INT3 |
| Trasero Derecho | D2 | INT4 |
| Trasero Izquierdo | D3 | INT5 |

## 4. Requisitos de Software
Para el manejo de interrupciones en Rust bare-metal:
1.  Habilitar feature: `#![feature(abi_avr_interrupt)]`.
2.  Dependencia: `avr-device` para los macros de interrupción.
3.  Uso de `unsafe` para habilitar interrupciones globales: `avr_device::interrupt::enable()`.
