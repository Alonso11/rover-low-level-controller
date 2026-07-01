# Documentación Encoders de Efecto Hall

## 1. Descripción General

Los encoders de efecto Hall son sensores magnéticos que detectan el paso de imanes
montados en el eje del motor. Cada pulso equivale a una fracción de vuelta del motor,
lo que permite contar pulsos para medir desplazamiento y detectar bloqueos (stall).

En el rover se usan 6 encoders, uno por motor, conectados a los pines de interrupción
externa (INT0–INT5) del ATmega2560.

## 2. Principio de Funcionamiento

El sensor genera una señal digital (flanco de subida) cada vez que un imán pasa frente
al elemento Hall. La ISR correspondiente incrementa un contador en el objeto `HallEncoder`.

Solo se usa la Fase A (sin Fase B): suficiente para detección de stall. No se determina
la dirección de giro por hardware; la dirección se conoce por los pines IN del L298N.

## 3. Conexión al Arduino Mega

| Motor         | Pin (Fase A) | Interrupción | Registro   | Stall bit |
| :------------ | :----------- | :----------- | :--------- | :-------- |
| Front Right   | **D21**      | INT0         | PD0        | bit 0     |
| Front Left    | **D20**      | INT1         | PD1        | bit 1     |
| Center Right  | **D19**      | INT2         | PD2        | bit 2     |
| Center Left   | **D18**      | INT3         | PD3        | bit 3     |
| Rear Right    | **D2**       | INT4         | PE4        | bit 4     |
| Rear Left     | **D3**       | INT5         | PE5        | bit 5     |

Wiring por encoder:

| Cable sensor | Conexión          |
| :----------- | :---------------- |
| VCC          | 5V                |
| GND          | GND               |
| OUT (Fase A) | Pin INT correspondiente (pull-up habilitado por software) |

> Los pines D18/D19 están disponibles porque la RPi5 usa USART3 (D14/D15), no USART1.

## 4. Configuración de Interrupciones

Las interrupciones se configuran en `src/main.rs` al iniciar:

- **Trigger:** flanco de subida (`ISCn1=1, ISCn0=1`)
- **EICRA/EICRB:** registros de control para INT0–INT7 del ATmega2560
- **EIMSK:** máscara que habilita cada interrupción individualmente
- **Pull-up:** activado en cada pin para evitar lecturas espurias con la línea en alto

```rust
// Ejemplo INT0 (D21, Front Right)
dp.EXINT.eicra().modify(|_, w| unsafe { w.isc0().bits(0x03) }); // rising edge
dp.EXINT.eimsk().modify(|_, w| unsafe { w.int().bits(0x3F) });  // INT0-INT5 en main.rs
unsafe { avr_device::interrupt::enable() };
```

## 5. Driver — `src/sensors/encoder.rs`

### Estructuras

**`HallEncoder`** — contador de pulsos con acceso seguro entre ISR y main loop:
- Internamente usa `Mutex<Cell<i32>>` de `avr_device::interrupt`.
- La ISR llama a `pulse()` (incremento siempre positivo).
- El main loop llama a `get_counts()` y `reset()`.

### Métodos

| Método              | Descripción                                               |
| :------------------ | :-------------------------------------------------------- |
| `new()`             | Crea el encoder con contador en 0 (const, para statics)  |
| `pulse()`           | Incrementa +1 desde la ISR                               |
| `update(forward)`   | Incrementa o decrementa según dirección (no usado actualmente) |
| `get_counts() → i32`| Lee el contador de forma segura (sección crítica)        |
| `reset()`           | Pone el contador a 0                                      |

### Seguridad en AVR

El AVR es single-core. El patrón `Mutex<Cell<T>>` garantiza exclusión mutua porque
`avr_device::interrupt::free` desactiva interrupciones durante el acceso, evitando
condiciones de carrera entre la ISR y el main loop.

## 6. Detección de Stall en `main.rs`

El main loop compara los conteos de cada encoder entre ciclos. Si un motor está
activo (PWM > 0) pero no genera pulsos durante N ciclos consecutivos, se marca
como bloqueado (stall) y se activa el estado `FAULT` en la MSM.

## 7. Ejemplo de Uso

Ver `examples/test_encoders.rs` — prueba básica con un solo encoder en D21 (INT0).

Comando de flash:
```bash
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" \
  cargo +nightly run --example test_encoders --release \
  -Zjson-target-spec -Zbuild-std=core
```

Salida esperada (girando el motor manualmente):
```
Pulsos detectados: 0
Pulsos detectados: 7
Pulsos detectados: 15
```
