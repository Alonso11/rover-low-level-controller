# Control de 6 Motores (Rover Olympus) - Arquitectura Bare-Metal

Este documento detalla la arquitectura implementada para el control de un chasis de 6 ruedas utilizando 3 puentes-H L298N sobre un ATmega2560, empleando técnicas de **Programación Bare-Metal**.

## 1. Abstracción: Estructura `SixWheelRover`

Para gestionar la complejidad de 6 motores individuales, hemos implementado una abstracción de alto nivel en `src/motor_control/l298n.rs`.

### Arquitectura de la Estructura
La estructura `SixWheelRover` agrupa 6 instancias del trait `Motor`. Esto permite un control coordinado:
- **Control Unificado:** Sincroniza los tres motores de cada lado para asegurar una tracción constante y evitar derrapes.
- **Independencia de Implementación:** Funciona con cualquier driver que cumpla el trait `Motor`, facilitando futuros cambios de hardware.

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

## 2. Técnica: PWM por Hardware (Timers Nativos)

En lugar de generar señales de control mediante software (lo cual consumiría ciclos de CPU), utilizamos los **Timers de Hardware** del ATmega2560. Esta es una característica fundamental de la programación Bare-Metal.

### Ventajas del PWM por Hardware
- **Ejecución Asíncrona:** Una vez configurado el Timer, este genera la señal PWM de forma totalmente independiente del código principal.
- **Estabilidad de Frecuencia:** La señal es generada por los circuitos lógicos del chip, lo que elimina cualquier variación (jitter) en la velocidad de los motores.
- **Eficiencia Total:** La CPU no interviene en la generación del pulso, quedando 100% disponible para la lógica de navegación y comunicación.

### Asignación de Recursos de Hardware
Hemos distribuido los 6 canales PWM en 3 Timers independientes (Timer 2, 3 y 4):

| Componente | Recurso (Timer) | Pin PWM | Pines Dirección (IN1/IN2) |
| :--- | :--- | :--- | :--- |
| **Puente 1 (Frontal)** | Timer 2 | D10, D9 | D22 al D25 |
| **Puente 2 (Central)** | Timer 3 | D5, D2 | D26 al D29 |
| **Puente 3 (Trasero)** | Timer 4 | D6, D7 | D30 al D33 |

### Configuración de los Timers
Utilizamos un `Prescaler::Prescale64` para ajustar la frecuencia del PWM a un rango óptimo para motores DC (aprox. 1kHz), proporcionando un control de velocidad suave y reduciendo el ruido eléctrico.

```rust
let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);
```

## 3. Lógica de Control
El sistema opera en un bucle de control infinito donde la CPU solo interviene para cambiar los registros de los Timers cuando se recibe un nuevo comando serie. Mientras no hay cambios, el hardware mantiene los motores girando a la última velocidad establecida sin intervención del programador.
