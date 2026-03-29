// Version: v2.7
//! # Firmware Principal — Rover Olympus / Arduino Mega 2560
//!
//! ## Loop principal (20 ms / ciclo):
//!   1. `msm.tick()`            — watchdog: sin PING en 100 ciclos (~2 s) → FAULT
//!   2. HC-SR04 + VL53L0X (cada 5 ciclos) — emergencia < 20/15 cm → FAULT
//!   3. Stall detection         — encoders via ISR → msm.update_safety(mask)
//!   4. `iface.poll_command()`  — trama ASCII desde USART0 (USB/RPi5)
//!   5. `msm.process(cmd)`      — transición de estado + calcula DriveOutput
//!   6. `sync_drive!()`         — aplica DriveOutput a los 6 motores
//!   7. `iface.send_response()` — respuesta ASCII
//!   8. Sensores analógicos (cada 25 ciclos ~500 ms) — corriente + temperatura
//!   9. Telemetría (~1 s)       — TLM:<SAFETY>:<MASK>
//!
//! ## Asignación de pines:
//!   - USART0 (USB) @ 115200 → RPi5 / PC
//!   - Timer2 D9(FR) D10(FL) | Timer3 D5(CR) | Timer4 D6(CL) D7(RR) D8(RL)
//!   - Dirección motores: D22–D37
//!   - HC-SR04: D38(Trigger) D39(Echo)
//!   - VL53L0X: D42(SDA/PL7) D43(SCL/PL6) - soft I2C, avoids TWI conflict
//!   - Encoders: D21(INT0/FR) D20(INT1/FL) D19(INT2/CR) D18(INT3/CL)
//!               D2(INT4/RR) D3(INT5/RL)
//!   - ACS712-30A: A0(FR) A1(FL) A2(CR) A3(CL) A4(RR) A5(RL)
//!   - LM335:      A6
//!
//! ## Diseño de encoders:
//!   Los 6 HallEncoders son `static` para ser accesibles desde las ISRs.
//!   Cada ISR llama `pulse()` en rising edge. El loop principal lee los
//!   contadores y detecta stall si no cambian durante STALL_THRESHOLD ciclos
//!   mientras la velocidad supera STALL_SPEED_MIN.
//!   Ver consideration_implementation.md §6 para la decisión de diseño.

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use arduino_hal::simple_pwm::{IntoPwmPin, Prescaler, Timer2Pwm, Timer3Pwm, Timer4Pwm};
use rover_low_level_controller::command_interface::{CommandInterface, RxRingBuffer};
use rover_low_level_controller::motor_control::l298n::{L298NMotor, SixWheelRover};
use rover_low_level_controller::sensors::hc_sr04::HCSR04;
use rover_low_level_controller::sensors::encoder::{HallEncoder, Encoder};
use rover_low_level_controller::sensors::{ACS712, LM335, NTCThermistor, VL53L0X};
use arduino_hal::prelude::*;
use rover_low_level_controller::state_machine::{
    format_response, parse_command, Command, MasterStateMachine, Response, RoverState,
    SafetyState, SensorFrame,
};

// ─── Constantes ──────────────────────────────────────────────────────────────

const TLM_PERIOD: u8    = 50;  // ciclos entre telemetría (~1 s a 20 ms/ciclo)
const LOOP_MS: u32      = 20;
const RESP_BUF: usize   = 128; // TLM extendido: TLM:NORMAL:000000:±30000×6:±100C:±100×6C\n ≈ 110 bytes

/// Cada cuántos ciclos leer el HC-SR04 (~100 ms).
/// El driver es bloqueante; ver consideration_implementation.md §5.
const HC_READ_PERIOD: u8  = 5;

/// Distancia de emergencia HC-SR04 en mm (20 cm → FAULT inmediato).
const HC_EMERGENCY_MM: u16 = 200;

/// Distancia de emergencia VL53L0X en mm (15 cm → FAULT inmediato).
/// Umbral más ajustado que HC-SR04 gracias a la mayor precisión del ToF láser.
const TOF_EMERGENCY_MM: u16 = 150;

/// Ciclos sin movimiento de encoder para declarar stall (~1 s a 20 ms/ciclo).
/// Coincide con el umbral de DriveChannel::check_stall en controller/mod.rs.
const STALL_THRESHOLD: u16 = 50;

/// Velocidad mínima absoluta (%) para activar la detección de stall.
/// Por debajo de este valor se asume que el motor está intencionalmente parado.
const STALL_SPEED_MIN: i16 = 20;

/// Cada cuántos ciclos leer los sensores analógicos (~500 ms).
const SEN_READ_PERIOD: u8 = 25;

/// Umbrales base para driver L298N (2 A continuo, 3 A pico).
const OC_WARN_L298N:  i32 = 1_200; // 60 % de 2A
const OC_LIMIT_L298N: i32 = 1_600; // 80 % de 2A
const OC_FAULT_L298N: i32 = 2_000; // 100 % de 2A

/// Umbrales base para driver BTS7960 (ajustar según motor real tras calibración).
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
const OC_WARN_BTS:  i32 = 8_000;  // ~20 % del pico — indicativo
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
const OC_LIMIT_BTS: i32 = 12_000; // ~28 % del pico
#[cfg(any(feature = "mixed-drivers", feature = "all-bts7960"))]
const OC_FAULT_BTS: i32 = 15_000; // ~35 % del pico

/// Umbrales de sobrecorriente por motor [FR, FL, CR, CL, RR, RL].
/// Seleccionados en tiempo de compilación según el feature activo:
///   (default)     → all-l298n:     todos OC_*_L298N
///   mixed-drivers → FR/FL=L298N, CR/CL/RR/RL=BTS7960
///   all-bts7960   → todos OC_*_BTS
#[cfg(feature = "mixed-drivers")]
const OC_WARN:  [i32; 6] = [OC_WARN_L298N,  OC_WARN_L298N,  OC_WARN_BTS,  OC_WARN_BTS,  OC_WARN_BTS,  OC_WARN_BTS];
#[cfg(feature = "mixed-drivers")]
const OC_LIMIT: [i32; 6] = [OC_LIMIT_L298N, OC_LIMIT_L298N, OC_LIMIT_BTS, OC_LIMIT_BTS, OC_LIMIT_BTS, OC_LIMIT_BTS];
#[cfg(feature = "mixed-drivers")]
const OC_FAULT: [i32; 6] = [OC_FAULT_L298N, OC_FAULT_L298N, OC_FAULT_BTS, OC_FAULT_BTS, OC_FAULT_BTS, OC_FAULT_BTS];

#[cfg(feature = "all-bts7960")]
const OC_WARN:  [i32; 6] = [OC_WARN_BTS;  6];
#[cfg(feature = "all-bts7960")]
const OC_LIMIT: [i32; 6] = [OC_LIMIT_BTS; 6];
#[cfg(feature = "all-bts7960")]
const OC_FAULT: [i32; 6] = [OC_FAULT_BTS; 6];

#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
const OC_WARN:  [i32; 6] = [OC_WARN_L298N;  6];
#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
const OC_LIMIT: [i32; 6] = [OC_LIMIT_L298N; 6];
#[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
const OC_FAULT: [i32; 6] = [OC_FAULT_L298N; 6];

/// Velocidad máxima (%) aplicada a todos los motores cuando safety == Limit.
const LIMIT_SPEED_CAP: i16 = 60;

/// Umbrales de temperatura de batería 18650 en °C.
/// Thermal runaway inicia ~80–90 °C; márgenes conservadores.
const BATT_WARN_C:  i32 = 45; // operación prolongada a alta carga
const BATT_LIMIT_C: i32 = 55; // reducir carga
const BATT_FAULT_C: i32 = 65; // detener rover — peligro inmediato

/// Número de muestras ADC a promediar por canal.
/// El ATmega2560 tiene un solo ADC multiplexado: cada muestra toma ~104 µs.
/// 8 muestras × 7 canales = ~5.8 ms bloqueantes cada SEN_READ_PERIOD ciclos.
const SEN_SAMPLES: u8 = 8;

/// Muestras ADC para el chequeo rápido de sobrecorriente (solo detección de Fault).
/// 2 muestras × 6 canales × ~104 µs = ~1.25 ms bloqueantes cada SEN_FAST_PERIOD ciclos.
const SEN_FAST_SAMPLES: u8 = 2;

/// Cada cuántos ciclos ejecutar el chequeo rápido (~60 ms a 20 ms/ciclo).
const SEN_FAST_PERIOD: u8 = 3;

// ─── Encoders estáticos (accesibles desde ISRs) ───────────────────────────────
//
// Orden del stall_mask (bit N = motor N):
//   bit 0 = Front Right  (INT0 / D21)
//   bit 1 = Front Left   (INT1 / D20)
//   bit 2 = Center Right (INT2 / D19) ← libre gracias a USART3 para RPi
//   bit 3 = Center Left  (INT3 / D18) ← libre gracias a USART3 para RPi
//   bit 4 = Rear Right   (INT4 / D2)
//   bit 5 = Rear Left    (INT5 / D3)

static ENCODER_FR: HallEncoder = HallEncoder::new(); // Front Right  — INT0, D21
static ENCODER_FL: HallEncoder = HallEncoder::new(); // Front Left   — INT1, D20
static ENCODER_CR: HallEncoder = HallEncoder::new(); // Center Right — INT2, D19
static ENCODER_CL: HallEncoder = HallEncoder::new(); // Center Left  — INT3, D18
static ENCODER_RR: HallEncoder = HallEncoder::new(); // Rear Right   — INT4, D2
static ENCODER_RL: HallEncoder = HallEncoder::new(); // Rear Left    — INT5, D3

// ─── ISRs — rising edge en Fase A de cada encoder ────────────────────────────

#[avr_device::interrupt(atmega2560)]
fn INT0() { ENCODER_FR.pulse(); }

#[avr_device::interrupt(atmega2560)]
fn INT1() { ENCODER_FL.pulse(); }

#[avr_device::interrupt(atmega2560)]
fn INT2() { ENCODER_CR.pulse(); }

#[avr_device::interrupt(atmega2560)]
fn INT3() { ENCODER_CL.pulse(); }

#[avr_device::interrupt(atmega2560)]
fn INT4() { ENCODER_RR.pulse(); }

#[avr_device::interrupt(atmega2560)]
fn INT5() { ENCODER_RL.pulse(); }

// ─── Ring buffer USART RX (interrupt-driven) ─────────────────────────────────
//
// El FIFO hardware USART del ATmega2560 tiene solo 3 bytes. Con delay_ms(20)
// en el loop, un comando de 5+ bytes llega completo en ~434 µs y los últimos
// bytes se pierden por overflow (DOR). Ver docs/debug_usart_overflow.md.
//
// Solución: la ISR USART_RX copia cada byte recibido en este ring buffer de
// 64 bytes. poll_from_ring() lo drena en cada iteración del loop sin importar
// si el CPU estuvo bloqueado los últimos 20 ms.
//
// MODO TEST (actual): USART0 (USB) — ISR: USART0_RX, registro: UDR0
// MODO PRODUCCIÓN:    USART3 (RPi) — cambiar ISR a USART3_RX y leer UDR3

static RX_BUF: RxRingBuffer = RxRingBuffer::new();

/// ISR USART0 RX Complete — copia byte recibido al ring buffer.
/// En producción (USART3) renombrar a USART3_RX y leer UDR3.
#[avr_device::interrupt(atmega2560)]
fn USART0_RX() {
    // Safety: acceso al registro hardware en contexto de interrupción.
    // Las interrupciones globales están deshabilitadas implícitamente durante ISR.
    let byte = unsafe {
        (*avr_device::atmega2560::USART0::ptr()).udr0().read().bits()
    };
    unsafe { RX_BUF.push(byte); }
}

// ─── Macro auxiliar ──────────────────────────────────────────────────────────

/// Promedia N lecturas de un pin analógico para reducir ruido ADC.
/// El ATmega2560 tiene un solo ADC multiplexado; cada lectura es ~104 µs.
macro_rules! adc_avg {
    ($pin:expr, $adc:expr, $n:expr) => {{
        let mut sum = 0u32;
        for _ in 0..$n {
            sum += $pin.analog_read(&mut $adc) as u32;
        }
        (sum / $n as u32) as u16
    }};
}

/// Aplica msm.drive al rover; en FAULT/STANDBY para todos los motores.
/// En estado Limit recorta la velocidad a LIMIT_SPEED_CAP %.
macro_rules! sync_drive {
    ($rover:expr, $msm:expr) => {
        match $msm.state {
            RoverState::Fault | RoverState::Standby => $rover.stop(),
            _ => {
                let (l, r) = if $msm.safety == SafetyState::Limit {
                    ($msm.drive.left.clamp(-LIMIT_SPEED_CAP, LIMIT_SPEED_CAP),
                     $msm.drive.right.clamp(-LIMIT_SPEED_CAP, LIMIT_SPEED_CAP))
                } else {
                    ($msm.drive.left, $msm.drive.right)
                };
                $rover.set_speeds(l, r);
            }
        }
    };
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // ── USART0: USB — modo test desde PC ─────────────────────────────────────
    // En producción usar USART3 (D14/D15): ver comentario en bloque ISR arriba.
    let serial_rpi = arduino_hal::default_serial!(dp, pins, 115200);
    let mut iface = CommandInterface::new(serial_rpi);

    // Habilitar interrupción USART0 RX Complete (RXCIE0 = bit 7 de UCSR0B).
    // La ISR USART0_RX copiará cada byte al RX_BUF antes de que el FIFO
    // hardware (3 bytes) se desborde durante delay_ms(20).
    // En producción (USART3): modificar UCSR3B con bit RXCIE3.
    //
    // IMPORTANTE: vaciar el FIFO hardware antes de habilitar la ISR.
    // El bootloader puede dejar bytes residuales en el FIFO que, de no
    // descartarse, la ISR los leería al ejecutar SEI y los almacenaría
    // en RX_BUF como basura, potencialmente formando comandos inválidos
    // (p.ej. "FLT") que pondrían la MSM en FAULT en el arranque.
    unsafe {
        let p = &(*avr_device::atmega2560::USART0::ptr());
        while p.ucsr0a().read().rxc0().bit_is_set() {
            let _ = p.udr0().read().bits(); // descartar byte residual
        }
        p.ucsr0b().modify(|_, w| w.rxcie0().set_bit());
    }

    // ── Timers PWM ───────────────────────────────────────────────────────────
    let mut timer2 = Timer2Pwm::new(dp.TC2, Prescaler::Prescale64);
    let mut timer3 = Timer3Pwm::new(dp.TC3, Prescaler::Prescale64);
    let mut timer4 = Timer4Pwm::new(dp.TC4, Prescaler::Prescale64);

    // ── 6 Motores — layout verificado en control_6_motors_l298n.rs v3.0 ─────
    let fr = L298NMotor::new(pins.d9.into_output().into_pwm(&mut timer2),  pins.d23.into_output(), pins.d25.into_output(), false);
    let fl = L298NMotor::new(pins.d10.into_output().into_pwm(&mut timer2), pins.d22.into_output(), pins.d24.into_output(), false);
    let cr = L298NMotor::new(pins.d5.into_output().into_pwm(&mut timer3),  pins.d28.into_output(), pins.d29.into_output(), false);
    let cl = L298NMotor::new(pins.d6.into_output().into_pwm(&mut timer4),  pins.d30.into_output(), pins.d31.into_output(), false);
    let rr = L298NMotor::new(pins.d7.into_output().into_pwm(&mut timer4),  pins.d34.into_output(), pins.d35.into_output(), false);
    let rl = L298NMotor::new(pins.d8.into_output().into_pwm(&mut timer4),  pins.d36.into_output(), pins.d37.into_output(), false);
    let mut rover = SixWheelRover::new(fr, fl, cr, cl, rr, rl);

    // ── HC-SR04 — D38(Trigger), D39(Echo) ───────────────────────────────────
    let mut hcsr04 = HCSR04::new(
        pins.d38.into_output(),
        pins.d39.into_floating_input().forget_imode(),
    );

    // ── VL53L0X — D42(SDA/PL7), D43(SCL/PL6) vía soft I2C ──────────────────
    // Los pines son controlados directamente por soft_i2c (registros PORTL/DDRL),
    // no se consumen como recursos arduino-hal. init() puede fallar si el sensor
    // no responde; el driver queda en ready=false y read_mm() no se llamará.
    let mut tof = VL53L0X::new();
    if tof.init() {
        tof.start_continuous();
    }

    // ── ADC + sensores analógicos ─────────────────────────────────────────────
    // ACS712 (corriente):         A0=FR A1=FL A2=CR A3=CL A4=RR A5=RL
    // LM335  (temp. ambiente):    A6
    // NTC    (temp. baterías):    A7=B1a A8=B1b A9=B2a A10=B2b A11=B3a A12=B3b
    let mut adc     = arduino_hal::Adc::new(dp.ADC, Default::default());
    let acs_fr_pin  = pins.a0.into_analog_input(&mut adc);
    let acs_fl_pin  = pins.a1.into_analog_input(&mut adc);
    let acs_cr_pin  = pins.a2.into_analog_input(&mut adc);
    let acs_cl_pin  = pins.a3.into_analog_input(&mut adc);
    let acs_rr_pin  = pins.a4.into_analog_input(&mut adc);
    let acs_rl_pin  = pins.a5.into_analog_input(&mut adc);
    let lm335_pin   = pins.a6.into_analog_input(&mut adc);
    let ntc_b1a_pin = pins.a7.into_analog_input(&mut adc);   // Banco 1 — sensor A
    let ntc_b1b_pin = pins.a8.into_analog_input(&mut adc);   // Banco 1 — sensor B
    let ntc_b2a_pin = pins.a9.into_analog_input(&mut adc);   // Banco 2 — sensor A
    let ntc_b2b_pin = pins.a10.into_analog_input(&mut adc);  // Banco 2 — sensor B
    let ntc_b3a_pin = pins.a11.into_analog_input(&mut adc);  // Banco 3 — sensor A
    let ntc_b3b_pin = pins.a12.into_analog_input(&mut adc);  // Banco 3 — sensor B

    // Instancias ACS712 por motor [FR, FL, CR, CL, RR, RL].
    // La variante (05A/30A) se elige en compilación según el feature activo.
    // Para calibrar el zero_mv de un motor concreto usar .calibrate_zero(mv).
    #[cfg(feature = "mixed-drivers")]
    let acs: [ACS712; 6] = [
        ACS712::new_05a(), ACS712::new_05a(), // FR, FL → L298N
        ACS712::new_30a(), ACS712::new_30a(), // CR, CL → BTS7960
        ACS712::new_30a(), ACS712::new_30a(), // RR, RL → BTS7960
    ];
    #[cfg(feature = "all-bts7960")]
    let acs: [ACS712; 6] = [ACS712::new_30a(); 6];
    #[cfg(not(any(feature = "mixed-drivers", feature = "all-bts7960")))]
    let acs: [ACS712; 6] = [ACS712::new_05a(); 6]; // all-l298n (default)

    let lm335 = LM335::new(); // offset 0 K — ajustar con with_offset si es necesario

    // Instancias NTC por sensor de batería [B1a, B1b, B2a, B2b, B3a, B3b].
    // offset 0 — calibrar con .calibrate(offset_c) tras verificación en hardware.
    let ntc_batt: [NTCThermistor; 6] = [NTCThermistor::new(); 6];

    // ── Interrupciones externas INT0–INT5 (rising edge) ──────────────────────
    // EICRA: controla INT0–INT3 (ISCn1=1, ISCn0=1 → rising edge)
    // EICRB: controla INT4–INT5 (ISCn1=1, ISCn0=1 → rising edge)
    // EIMSK: habilita INT0–INT5 (bits 0–5)
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0xFF) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x0F) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });
    unsafe { avr_device::interrupt::enable() };

    // ── Estado del loop ──────────────────────────────────────────────────────
    let mut msm              = MasterStateMachine::new();
    let mut resp_buf         = [0u8; RESP_BUF];
    let mut tlm_counter:      u8 = 0;
    let mut hc_counter:       u8 = 0;
    let mut sen_counter:      u8 = 0;
    let mut sen_fast_counter: u8 = 0;
    let mut elapsed_ms:      u32 = 0; // timestamp relativo desde arranque (overflow ~49 días)
    let mut sensor_frame = SensorFrame::ZERO; // última lectura de ACS712 + LM335

    // Estado de stall por encoder (parallel al stall_mask de la MSM)
    let mut last_counts  = [0i32; 6];
    let mut stall_timers = [0u16; 6];

    iface.log("=== ROVER OLYMPUS v2.7 — MSM + HC-SR04 + VL53L0X + ENCODERS + ACS712 + LM335 + NTC ===");

    // ── Bucle principal ───────────────────────────────────────────────────────
    loop {
        // 1. Watchdog de comunicación
        if let Some(wdog_resp) = msm.tick() {
            sync_drive!(rover, msm);
            iface.send_response(format_response(wdog_resp, &mut resp_buf));
        }

        // 2. HC-SR04 + VL53L0X — lectura cada HC_READ_PERIOD ciclos (~100 ms)
        //    HC-SR04 es bloqueante (~30 ms max); VL53L0X no bloquea (modo continuo).
        hc_counter = hc_counter.wrapping_add(1);
        if hc_counter >= HC_READ_PERIOD {
            hc_counter = 0;
            if let Some(mm) = hcsr04.measure_mm() {
                if mm < HC_EMERGENCY_MM {
                    let resp = msm.process(Command::Fault);
                    sync_drive!(rover, msm);
                    iface.send_response(format_response(resp, &mut resp_buf));
                }
            }
            // VL53L0X: lectura no bloqueante — solo hay dato si el sensor está listo
            if tof.ready {
                if let Some(mm) = tof.read_mm() {
                    sensor_frame.dist_mm = mm;
                    if mm < TOF_EMERGENCY_MM {
                        let resp = msm.process(Command::Fault);
                        sync_drive!(rover, msm);
                        iface.send_response(format_response(resp, &mut resp_buf));
                    }
                }
            }
        }

        // 3. Stall detection via encoders
        // Lee los 6 contadores y compara con el ciclo anterior.
        // Si la velocidad supera STALL_SPEED_MIN y el encoder no avanzó,
        // incrementa el timer. Al superar STALL_THRESHOLD → bit en stall_mask.
        {
            let counts = [
                ENCODER_FR.get_counts(), ENCODER_FL.get_counts(),
                ENCODER_CR.get_counts(), ENCODER_CL.get_counts(),
                ENCODER_RR.get_counts(), ENCODER_RL.get_counts(),
            ];
            // Velocidad por motor: [FR, FL, CR, CL, RR, RL]
            // FR/CR/RR → lado derecho, FL/CL/RL → lado izquierdo
            let speeds = [
                msm.drive.right, msm.drive.left,   // FR, FL
                msm.drive.right, msm.drive.left,   // CR, CL
                msm.drive.right, msm.drive.left,   // RR, RL
            ];
            let mut stall_mask: u8 = 0;
            for i in 0..6usize {
                if speeds[i].abs() > STALL_SPEED_MIN && counts[i] == last_counts[i] {
                    stall_timers[i] = stall_timers[i].saturating_add(1);
                } else {
                    stall_timers[i] = 0;
                }
                last_counts[i] = counts[i];
                if stall_timers[i] > STALL_THRESHOLD {
                    stall_mask |= 1 << i;
                }
            }
            if stall_mask != 0 {
                msm.update_safety(stall_mask);
                sync_drive!(rover, msm);
                iface.send_response(format_response(msm.telemetry(stall_mask, sensor_frame), &mut resp_buf));
            }
        }

        // 4. Comando entrante desde RPi5 (interrupt-driven via RX_BUF)
        if iface.poll_from_ring(&RX_BUF) {
            // Copiar a buffer local para liberar el borrow de iface
            let mut cmd_buf = [0u8; 32];
            let cmd_len = {
                let raw = iface.get_command();
                let len = raw.len().min(32);
                cmd_buf[..len].copy_from_slice(&raw[..len]);
                len
            };
            let cmd_bytes = &cmd_buf[..cmd_len];
            // DEBUG: echo hex de los bytes recibidos
            iface.log("DBG:");
            for &b in cmd_bytes {
                let hi = b >> 4;
                let lo = b & 0xF;
                let h = if hi < 10 { b'0' + hi } else { b'a' + hi - 10 };
                let l = if lo < 10 { b'0' + lo } else { b'a' + lo - 10 };
                resp_buf[0] = h; resp_buf[1] = l; resp_buf[2] = b' ';
                iface.send_response(&resp_buf[..3]);
            }
            iface.send_response(b"\n");
            let response = match parse_command(cmd_bytes) {
                Some(cmd) => msm.process(cmd),
                None      => Response::ErrUnknown,
            };
            sync_drive!(rover, msm);
            iface.send_response(format_response(response, &mut resp_buf));
        }

        // 5a. Chequeo rápido de sobrecorriente — cada SEN_FAST_PERIOD ciclos (~60 ms).
        //     Solo 2 muestras por canal (~1.25 ms bloqueantes): detecta Fault únicamente.
        //     No actualiza sensor_frame (usa las últimas lecturas lentas para TLM).
        sen_fast_counter = sen_fast_counter.wrapping_add(1);
        if sen_fast_counter >= SEN_FAST_PERIOD && msm.state != RoverState::Fault {
            sen_fast_counter = 0;
            let fast_raw = [
                adc_avg!(acs_fr_pin, adc, SEN_FAST_SAMPLES),
                adc_avg!(acs_fl_pin, adc, SEN_FAST_SAMPLES),
                adc_avg!(acs_cr_pin, adc, SEN_FAST_SAMPLES),
                adc_avg!(acs_cl_pin, adc, SEN_FAST_SAMPLES),
                adc_avg!(acs_rr_pin, adc, SEN_FAST_SAMPLES),
                adc_avg!(acs_rl_pin, adc, SEN_FAST_SAMPLES),
            ];
            let mut fault_mask: u8 = 0;
            for i in 0..6usize {
                if acs[i].read_ma(fast_raw[i]).abs() > OC_FAULT[i] {
                    fault_mask |= 1 << i;
                }
            }
            if fault_mask != 0 {
                msm.update_overcurrent(SafetyState::FaultStall);
                sync_drive!(rover, msm);
                iface.send_response(format_response(
                    msm.telemetry(fault_mask, sensor_frame), &mut resp_buf));
            }
        }

        // 5b. Sensores analógicos — corriente y temperatura, cada SEN_READ_PERIOD ciclos (~500 ms).
        //     8 muestras: lectura precisa para Warn/Limit + actualiza sensor_frame para TLM.
        sen_counter = sen_counter.wrapping_add(1);
        if sen_counter >= SEN_READ_PERIOD {
            sen_counter = 0;

            let raw_i = [
                adc_avg!(acs_fr_pin, adc, SEN_SAMPLES),
                adc_avg!(acs_fl_pin, adc, SEN_SAMPLES),
                adc_avg!(acs_cr_pin, adc, SEN_SAMPLES),
                adc_avg!(acs_cl_pin, adc, SEN_SAMPLES),
                adc_avg!(acs_rr_pin, adc, SEN_SAMPLES),
                adc_avg!(acs_rl_pin, adc, SEN_SAMPLES),
            ];

            // Clasificar el peor nivel de corriente entre los 6 motores.
            // Cada motor usa su propia instancia ACS712 y sus umbrales OC_*[i].
            let mut worst = SafetyState::Normal;
            for i in 0..6usize {
                let current_ma = acs[i].read_ma(raw_i[i]);
                sensor_frame.currents[i] = current_ma;
                let abs_ma = current_ma.abs();
                let level = if abs_ma > OC_FAULT[i] {
                    SafetyState::FaultStall
                } else if abs_ma > OC_LIMIT[i] {
                    SafetyState::Limit
                } else if abs_ma > OC_WARN[i] {
                    SafetyState::Warn
                } else {
                    SafetyState::Normal
                };
                if level > worst { worst = level; }
            }

            let raw_t = adc_avg!(lm335_pin, adc, SEN_SAMPLES);
            sensor_frame.temp_c = lm335.read_celsius(raw_t);

            // Leer 6 sensores NTC de batería y clasificar el peor nivel térmico.
            let raw_batt = [
                adc_avg!(ntc_b1a_pin, adc, SEN_SAMPLES),
                adc_avg!(ntc_b1b_pin, adc, SEN_SAMPLES),
                adc_avg!(ntc_b2a_pin, adc, SEN_SAMPLES),
                adc_avg!(ntc_b2b_pin, adc, SEN_SAMPLES),
                adc_avg!(ntc_b3a_pin, adc, SEN_SAMPLES),
                adc_avg!(ntc_b3b_pin, adc, SEN_SAMPLES),
            ];
            for i in 0..6usize {
                let t = ntc_batt[i].read_celsius(raw_batt[i]);
                sensor_frame.batt_temps[i] = t;
                let level = if t > BATT_FAULT_C {
                    SafetyState::FaultStall
                } else if t > BATT_LIMIT_C {
                    SafetyState::Limit
                } else if t > BATT_WARN_C {
                    SafetyState::Warn
                } else {
                    SafetyState::Normal
                };
                if level > worst { worst = level; }
            }

            let prev_safety = msm.safety;
            let faulted = msm.update_overcurrent(worst);

            // sync_drive si: hay fault, safety cambió (cap aplicado/eliminado), o sigue en Warn/Limit
            if faulted || msm.safety != prev_safety || msm.safety != SafetyState::Normal {
                sync_drive!(rover, msm);
            }
            // TLM inmediato si el estado no es Normal
            if faulted || msm.safety != SafetyState::Normal {
                iface.send_response(format_response(
                    msm.telemetry(0, sensor_frame), &mut resp_buf));
            }
        }

        // 6. Telemetría periódica (~1 s) — stall_mask siempre 0 aquí porque
        //    si hubiera stall ya se reportó en el bloque de encoders arriba.
        tlm_counter = tlm_counter.wrapping_add(1);
        if tlm_counter >= TLM_PERIOD {
            tlm_counter = 0;
            iface.send_response(format_response(msm.telemetry(0, sensor_frame), &mut resp_buf));
        }

        // Actualizar timestamp relativo antes del delay para que el próximo
        // ciclo (y el TLM periódico) reflejen el tiempo acumulado real.
        elapsed_ms = elapsed_ms.wrapping_add(LOOP_MS);
        sensor_frame.tick_ms = elapsed_ms;

        // delay_ms restaurado: la ISR USART0_RX garantiza que ningún byte
        // se pierde durante el bloqueo. Ver docs/debug_usart_overflow.md.
        arduino_hal::delay_ms(LOOP_MS);
    }
}
