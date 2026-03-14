<!-- Version: v1.0 -->
# Lógica de Protección y Máquina de Estados (Rover Olympus)

Este documento describe la arquitectura de seguridad y la lógica de control de bajo nivel para proteger los drivers L298N y los motores DC mediante sensores virtuales (basados en modelos matemáticos y encoders).

## 1. Modelo de Sensor Virtual de Corriente

Dado que el hardware no cuenta con sensores de corriente físicos (ACS712), se implementa un estimador basado en el modelo del motor DC:

### Fórmula de Estimación ($I_{est}$)
$$I_{est} = \frac{(V_{bat} \cdot \text{DutyCycle}) - (K_e \cdot \omega)}{R_{motor}}$$

*   **$V_{bat}$**: Voltaje actual de la batería (Leído por pin analógico o asumiendo nominal).
*   **$K_e$**: Constante de fuerza contraelectromotriz (V/(rad/s)).
*   **$\omega$**: Velocidad angular real leída de los encoders.
*   **$R_{motor}$**: Resistencia interna de las bobinas.

---

## 2. Fusible Térmico Virtual (Algoritmo $I^2t$)

Para proteger los componentes del calor acumulado por sobrecorrientes, se utiliza un acumulador de energía térmica ($E_{term}$):

### Lógica de Acumulación
En cada ciclo de control ($\Delta t = 100ms$):
1.  Calcular $I_{est}$.
2.  Si $I_{est} > I_{nominal}$: 
    $$E_{term} = E_{term} + (I_{est}^2 - I_{nominal}^2) \cdot \Delta t$$
3.  Si $I_{est} \leq I_{nominal}$: 
    $$E_{term} = E_{term} - \text{Constante\_Enfriamiento} \cdot \Delta t$$

*Nota: $E_{term}$ nunca puede ser menor que 0.*

---

## 3. Niveles de Acción (Máquina de Estados de Seguridad)

El sistema opera en diferentes estados según el valor de $E_{term}$ y la detección de fallos:

| Estado | Condición | Acción del Software | Recuperación |
| :--- | :--- | :--- | :--- |
| **NORMAL** | $E_{term} < 70\%$ | Funcionamiento al 100%. | N/A |
| **WARN** | $70\% \le E_{term} < 90\%$ | Envía log `HIGH_LOAD` a RPi. | Baja $E_{term}$ |
| **LIMIT** | $E_{term} \ge 90\%$ | Reduce PWM máximo al 40%. | $E_{term} < 60\%$ |
| **FAULT_STALL** | PWM > 30% & RPM < 5 | Corte inmediato (STOP). | Comando `RESET` |
| **FAULT_OVERHEAT**| $E_{term} \ge 100\%$ | Corte inmediato (STOP). | Enfriamiento + `RESET` |

---

## 4. Detección de Bloqueo (Stall Detection)

La detección de bloqueo es la protección más crítica para evitar que el driver L298N se queme instantáneamente:

### Algoritmo de Decisión
```rust
if (motor.pwm_duty > MIN_PWM_THRESHOLD) && (encoder.rpm < STALL_RPM_LIMIT) {
    stall_timer += dt;
    if (stall_timer > MAX_STALL_TIME) {
        trigger_fault(STALL_ERROR);
    }
} else {
    stall_timer = 0;
}
```

## 5. Parámetros de Calibración Sugeridos (Motor 12V 100RPM)
*   **$R_{motor}$**: ~2.5 $\Omega$
*   **$I_{nominal}$**: 0.8 A
*   **$I_{max\_L298N}$**: 2.0 A
*   **$K_e$**: Por determinar experimentalmente.
