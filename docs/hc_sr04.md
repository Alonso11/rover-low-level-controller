# Documentación HC-SR04 - Diagnóstico y Solución de Errores

## 1. Identificación del Problema
Durante las pruebas de integración con el sensor ultrasónico HC-SR04, se detectaron fallos intermitentes en la lectura. El síntoma principal era un `ECHO timeout (no sube)`, indicando que el sensor ignoraba el pulso de disparo (Trigger) de forma aleatoria.

**Entorno de prueba:**
- Microcontrolador: ATmega2560
- Pines probados: D38/D39 y D40/D41
- Objeto de prueba: Silla a ~90-130cm

## 2. Hallazgos Técnicos
- **Ruido Eléctrico:** Los sensores HC-SR04 son altamente sensibles a la estabilidad del voltaje de 5V.
- **Latencia de Hardware:** Algunos sensores no responden al estándar de 10µs para el pulso de Trigger.
- **Inestabilidad:** Aproximadamente el 30-50% de las lecturas fallaban por hardware, devolviendo un estado nulo.

## 3. Solución Implementada (Driver v1.1)
Para mitigar estos fallos sin sacrificar la respuesta en tiempo real del Rover, se aplicaron las siguientes mejoras en `src/sensors/hc_sr04.rs`:

### A. Robustez por Software
- **Estado de Memoria:** Se añadió el campo `last_valid` a la estructura `HCSR04` para almacenar la última distancia exitosa.
- **Filtro de Errores:** Si una medición falla por timeout, el driver devuelve la última distancia conocida.
- **Detección de Desconexión:** Si se superan los **5 errores consecutivos**, el driver invalida la persistencia y retorna `None`, permitiendo al sistema saber que el sensor ha fallado realmente.

### B. Ajustes de Temporización
- **Pulso de Trigger:** Aumentado de 10µs a **20µs** para asegurar la detección en sensores lentos.
- **Pre-pulso (Hold):** Se añadió un retraso de **10µs** en estado bajo antes del disparo para limpiar la línea de posibles ruidos.

## 4. Validación Final
Se utilizó el ejemplo `test_hcsr04_filtered.rs` para validar la solución.
- **Resultado:** Salida estable y continua de distancia (ej. 528mm, 539mm) sin interrupciones por timeout.
- **Conclusión:** El sistema es ahora capaz de navegar ignorando los parpadeos aleatorios del hardware.
