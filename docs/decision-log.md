# Registro de Decisiones — Rover Olympus LLC (Arduino)

Historial cronológico de decisiones de diseño, cambios de arquitectura y correcciones
relevantes del repositorio `rover-low-level-controller`. Derivado del historial de
commits desde el inicio del proyecto.

---

## Semana 1 — Fundación y primer motor (9 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-09 | Crear proyecto en Rust (`no_std`) para Arduino Mega 2560 | Rust garantiza seguridad de memoria y control determinista sin OS; `no_std` es obligatorio para microcontroladores sin sistema operativo |
| 2026-03-09 | Usar `arduino-hal` como capa de abstracción de hardware | HAL oficial para Arduino en Rust; abstrae registros AVR y simplifica GPIO/UART/PWM sin sacrificar control |
| 2026-03-09 | Excluir `Cargo.lock` del repo con `.gitignore` | El lockfile se regenera en cada build AVR; commitearlo generaba ruido innecesario en el historial |
| 2026-03-09 | Excluir `target/` del repo | Los artefactos de compilación AVR son binarios grandes (~100 MB); no tienen valor en el historial de git |
| 2026-03-09 | Implementar driver `L298N` con principios SOLID | Separar la lógica del driver de la lógica de control facilita el testing y permite agregar otros drivers (BTS7960) sin modificar código existente |
| 2026-03-09 | Implementar driver `BTS7960` para motores de alta potencia | El BTS7960 soporta 43 A pico frente a los 2 A del L298N — previsión para motores más potentes si se requiere |
| 2026-03-09 | Reestructurar proyecto como librería (`lib.rs`) en lugar de solo `main.rs` | Permite importar módulos desde examples y tests; el binario `main.rs` consume la librería como cualquier otro usuario |
| 2026-03-09 | Implementar `CommandInterface` para recibir comandos ASCII por UART | Abstrae el parsing de tramas serie; el `main.rs` solo llama `poll_command()` y `get_command()` sin gestionar bytes manualmente |
| 2026-03-09 | Implementar `SixWheelRover` como struct que agrupa los 6 motores | Encapsula el control diferencial (izquierda/derecha) de los 6 motores en una sola llamada `set_speeds(left, right)` |
| 2026-03-09 | Añadir comunicación UART con RPi5 via GPIO | Explorar comunicación directa RPi5 ↔ Arduino por GPIO antes de decidir si usar USB o UART dedicado |

---

## Semana 1 — Timers PWM y pinout (9–12 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-09 | Distribuir los 6 motores en Timer2, Timer3, Timer4 | Timer0 reservado para `delay_ms()` del HAL; Timer1 reservado para servo; Timer5 no disponible en arduino-hal para PWM |
| 2026-03-12 | Usar timers separados para cada par de motores L298N | Un solo timer compartido entre motores del mismo L298N causaba conflictos de frecuencia PWM — cada canal necesita su propio OC |
| 2026-03-12 | Añadir CI con GitHub Actions para validación automática en AVR | Detectar errores de compilación en cada push sin necesitar hardware físico presente |
| 2026-03-12 | Implementar `SixWheelRover` con ejemplo `control_6_motors_l298n.rs` | Validar el control de los 6 motores con comandos F/B/L/R/S antes de añadir más lógica |

---

## Semana 1 — Encoders Hall y pinout definitivo (14 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-14 | Usar encoders Hall con interrupciones externas INT0–INT5 | Las interrupciones capturan pulsos sin polling; polling en el loop principal perdería pulsos a alta velocidad |
| 2026-03-14 | Asignar encoders a D21/D20/D19/D18/D2/D3 (INT0–INT5) | Estos son los únicos 6 pines de interrupción externa del ATmega2560; no hay alternativa |
| 2026-03-14 | Cambiar comunicación RPi5 de USART1 (D18/D19) a USART3 (D14/D15) | USART1 usa D18(TX)/D19(RX) que son INT3/INT2 — necesarios para encoders centrales; USART3 libera estos pines |
| 2026-03-14 | Implementar `HallEncoder` como struct `static` con `AtomicI32` | Los encoders son accedidos tanto desde ISRs (escritura) como desde el loop principal (lectura); `AtomicI32` garantiza acceso seguro sin mutex en `no_std` |
| 2026-03-14 | Documentar pinout completo en `docs/the_pins_connections.md` | El ATmega2560 tiene 70 pines — la distribución es compleja y necesita referencia clara para evitar conflictos |
| 2026-03-14 | Versionar todos los archivos fuente con `// Version: vX.Y` | Facilita identificar qué versión está flasheada en el hardware físico cuando el código ha cambiado |

---

## Semana 2 — Sensores de proximidad (15 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-15 | Implementar driver `HCSR04` en el LLC (Arduino) | El HC-SR04 requiere timing preciso de microsegundos para el pulso trigger — más fiable en el microcontrolador que en Linux |
| 2026-03-15 | Conectar HC-SR04 a D38(Trigger) / D39(Echo) | Pines libres sin conflicto con timers, interrupciones ni UART; zona física del conector accesible |
| 2026-03-15 | Implementar driver `TfLuna` para LIDAR de distancia precisa | El TF-Luna da distancia a 100 Hz con ±6 cm de precisión hasta 8 m — complementa al HC-SR04 que solo alcanza 4 m |
| 2026-03-15 | Conectar TF-Luna a USART2 (D16/D17) | USART2 libre tras reasignar RPi5 a USART3; el TF-Luna usa UART 115200 nativo |
| 2026-03-15 | Documentar arquitectura de sensores por capas en `consideration_implementation.md` | Tres capas de distancia (HC-SR04 emergencia, TF-Luna táctica, cámara estratégica) con latencias distintas — necesita documentación para no mezclar responsabilidades |

---

## Semana 2 — Controlador y ErasedMotor (18 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-18 | Implementar `RoverController` con detección de stall por canal | El stall detection necesita acceso a velocidad y encoder por motor — encapsularlo en `DriveChannel` permite una API limpia desde `main.rs` |
| 2026-03-18 | Implementar `ErasedMotor` para arrays heterogéneos de motores | Los 6 motores usan tipos distintos de PWM pin (`OC2A`, `OC2B`, `OC3A`…) — `ErasedMotor` borra el tipo concreto permitiendo almacenarlos en un array |
| 2026-03-18 | Exportar módulo `controller` desde `lib.rs` | El módulo no estaba re-exportado — `main.rs` no podía importarlo; error de visibilidad |
| 2026-03-18 | Corregir conflictos de timer PWM en examples | Timer4 tiene canales OC4A/OC4B/OC4C; asignarlos a distintos motores sin conflicto requiere cuidado en el orden de inicialización |
| 2026-03-18 | Actualizar CI para cubrir todos los branches activos | Las validaciones solo corrían en `main`; los branches de feature necesitan la misma cobertura |
| 2026-03-18 | Eliminar comentario inválido en `avr-atmega2560.json` | Bitbake/cargo no aceptan comentarios en JSON estricto — causaba error de parse en el target spec |
| 2026-03-18 | Documentar comunicación UART RPi5 ↔ Arduino en `rpi5_uart_communication.md` | Referencia completa de configuración de puertos, baud rate y protocolo para el equipo |

---

## Semana 3 — Máquina de Estados Maestra / MSM (23 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-23 | Implementar MSM en módulo puro `state_machine/mod.rs` sin dependencias de `arduino-hal` | Permite compilar y testear la MSM en x86 con `cargo test` sin necesitar hardware ni emulador AVR |
| 2026-03-23 | Definir 5 estados: `Standby`, `Explore`, `Avoid`, `Retreat`, `Fault` | Mínimo viable para autonomía reactiva; mapea directamente a los modos del SyRS §7.3.8 |
| 2026-03-23 | Protocolo MSM ASCII: `PING/STB/EXP:l:r/AVD:L|R/RET/FLT/RST` | ASCII es depurable con cualquier terminal serie; sin overhead de framing binario para el ancho de banda de 115200 baud |
| 2026-03-23 | Respuestas MSM: `PONG/ACK:<STATE>/TLM:<SAFETY>:<MASK>/ERR:*` | El Arduino confirma cada transición de estado — la RPi5 puede detectar desincronización sin implementar timeout propio |
| 2026-03-23 | Watchdog de 100 ciclos (~2 s) que dispara `FAULT` si no llega comando | Si la RPi5 se cuelga o pierde la conexión, el rover se detiene solo — seguridad autónoma sin intervención manual |
| 2026-03-23 | Watchdog solo activo en `Explore`/`Avoid`/`Retreat` — no en `Standby`/`Fault` | En `Standby` el rover ya está parado — el watchdog no añade seguridad. En `Fault` ya está manejado. Evita transiciones espurias |
| 2026-03-23 | Escribir 41 tests nativos x86 para la MSM (`tests/state_machine_test.rs`) | Los tests validan todas las transiciones de estado sin flashear hardware; CI corre en segundos en lugar de minutos |
| 2026-03-23 | Implementar `format_response` con buffer estático `[u8; 24]` | `no_std` no tiene `String` ni `format!` con heap; el buffer estático evita allocaciones en AVR |
| 2026-03-23 | Documentar arquitectura de sensores por capas en `consideration_implementation.md` | HC-SR04 (emergencia <20 cm), TF-Luna (táctica <150 cm), cámara (estratégica) — cada capa con latencia y responsabilidad distintas |
| 2026-03-23 | Integrar MSM + motores + USART3 en `main.rs` v2.0 | Primer firmware completo con loop de control: watchdog → HC-SR04 → encoders → comando → motores → telemetría |
| 2026-03-23 | HC-SR04 leído cada 5 ciclos (~100 ms) en lugar de cada ciclo | El HC-SR04 es bloqueante (~30 ms por medición); leerlo cada ciclo de 20 ms lo haría dominante; cada 5 ciclos equilibra latencia y overhead |
| 2026-03-23 | Umbral de emergencia HC-SR04: 200 mm (20 cm) → `FAULT` inmediato | Margen suficiente para que los motores frenen antes del impacto a velocidades bajas de exploración |
| 2026-03-23 | Añadir `debug_motors_l298n.rs` como ejemplo de debug de pinout | Permite verificar cada motor individualmente en hardware sin correr el firmware completo |

---

## Semana 3 — Corrección de pinout y build-std (24 mar 2026)

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-24 | Corregir pinout CR/CL/RR/RL en `main.rs` y `control_6_motors_l298n.rs` | El pinout en el código no coincidía con el resultado verificado físicamente con `debug_motors_l298n.rs`; el debug es la fuente de verdad |
| 2026-03-24 | Pinout final confirmado: CR=D5/Timer3, CL=D6/Timer4, RR=D7/Timer4, RL=D8/Timer4 | Resultado del debug físico — los motores centrales y traseros estaban intercambiados en el código |
| 2026-03-24 | Dirección CR: D28/D29 (consecutivos) en lugar de D28/D30 | El layout físico del L298N Driver2 tiene IN1/IN2 consecutivos; D28/D30 saltaba un pin causando dirección errónea |
| 2026-03-24 | Eliminar `build-std = ["core"]` del `.cargo/config.toml` global | Con `build-std` en la config global, compilar para x86 (`cargo test`) incluía `core` dos veces → error de "duplicate lang item"; la solución es pasar `-Zbuild-std=core` solo en el comando AVR |
| 2026-03-24 | Sincronizar `docs/motors.md`, `docs/peripheral_timers.md` y todos los examples con el pinout corregido | Documentación y código deben coincidir — la discrepancia anterior causó tiempo perdido depurando hardware |

---

## Decisiones de arquitectura transversales

| Fecha | Decisión | Motivo |
|---|---|---|
| 2026-03-09 | Separar LLC (Arduino) y HLC (RPi5) en repositorios independientes | Ciclos de desarrollo distintos: el firmware AVR se flashea al hardware, la imagen Yocto tarda horas en compilar |
| 2026-03-14 | USART3 para RPi5 en lugar de USART0 (USB) en producción | USART0 está conectado al chip USB del Arduino — en campo no habrá cable USB; USART3 es el puerto dedicado para comunicación embebida |
| 2026-03-15 | Arquitectura de tres capas de seguridad: cámara → TF-Luna → HC-SR04 | Latencia y fiabilidad inversamente proporcionales: la cámara es lenta pero informativa; el HC-SR04 es rápido pero de un solo punto |
| 2026-03-23 | MSM vive en el Arduino (LLC), no en la RPi5 (HLC) | El Arduino tiene timing determinista a 20 ms/ciclo; si la RPi5 falla, el Arduino actúa autónomamente via watchdog |
| 2026-03-23 | Telemetría periódica `TLM:<SAFETY>:<MASK>` cada ~1 s | Permite a la RPi5 monitorear el estado de seguridad sin necesidad de polling activo |

---

## Comandos de build de referencia

```bash
# Compilar para AVR (Arduino Mega 2560)
RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly build -Zjson-target-spec -Zbuild-std=core

# Flashear al Arduino
RAVEDUDE_PORT=/dev/ttyUSB0 RUSTFLAGS="-C target-cpu=atmega2560" cargo +nightly run --release -Zjson-target-spec -Zbuild-std=core

# Tests nativos x86 (sin hardware)
cargo +nightly test --target x86_64-unknown-linux-gnu --no-default-features --test state_machine_test
```

---

## Pendiente (al 24 mar 2026)

| Tarea | Bloqueante | Prioridad |
|---|---|---|
| Flash firmware al Arduino y verificar protocolo MSM por serial | Sin acceso al hardware físico | Alta — bloquea todas las pruebas de integración |
| Integrar TF-Luna en `main.rs` (capa táctica <150 cm → `AVD`) | Flash pendiente | Media |
| Extender telemetría `TLM` con distancias de sensores | Flash pendiente | Baja |
| PR `feature/msm-main-integration` → `debug` | Flash + validación pendiente | Media |
