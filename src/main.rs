// Version: v2.3
//! # Firmware Principal — Rover Olympus / Arduino Mega 2560
//!
//! ## Loop principal (20 ms / ciclo):
//!   1. `msm.tick()`            — watchdog: sin PING en 100 ciclos (~2 s) → FAULT
//!   2. HC-SR04 (cada 5 ciclos) — emergencia < 20 cm → FAULT
//!   3. Stall detection         — encoders via ISR → msm.update_safety(mask)
//!   4. `iface.poll_command()`  — trama ASCII desde USART3 (RPi5)
//!   5. `msm.process(cmd)`      — transición de estado + calcula DriveOutput
//!   6. `sync_drive!()`         — aplica DriveOutput a los 6 motores
//!   7. `iface.send_response()` — respuesta ASCII a RPi5
//!   8. Telemetría (~1 s)       — TLM:<SAFETY>:<MASK>
//!
//! ## Asignación de pines:
//!   - USART3 D14(TX3)/D15(RX3) → RPi5 @ 115200
//!     NOTA: Se usa USART3 (no USART1) para liberar D18/D19 para encoders.
//!   - Timer2 D9(FR) D10(FL) | Timer3 D5(CR) | Timer4 D6(CL) D7(RR) D8(RL)
//!   - Dirección motores: D22–D37
//!   - HC-SR04: D38(Trigger) D39(Echo)
//!   - Encoders: D21(INT0/FR) D20(INT1/FL) D19(INT2/CR) D18(INT3/CL)
//!               D2(INT4/RR) D3(INT5/RL)
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
use arduino_hal::prelude::*;
use rover_low_level_controller::state_machine::{
    format_response, parse_command, Command, MasterStateMachine, Response, RoverState,
};

// ─── Constantes ──────────────────────────────────────────────────────────────

const TLM_PERIOD: u8    = 50;  // ciclos entre telemetría (~1 s a 20 ms/ciclo)
const LOOP_MS: u32      = 20;
const RESP_BUF: usize   = 24;

/// Cada cuántos ciclos leer el HC-SR04 (~100 ms).
/// El driver es bloqueante; ver consideration_implementation.md §5.
const HC_READ_PERIOD: u8  = 5;

/// Distancia de emergencia HC-SR04 en mm (20 cm → FAULT inmediato).
const HC_EMERGENCY_MM: u16 = 200;

/// Ciclos sin movimiento de encoder para declarar stall (~1 s a 20 ms/ciclo).
/// Coincide con el umbral de DriveChannel::check_stall en controller/mod.rs.
const STALL_THRESHOLD: u16 = 50;

/// Velocidad mínima absoluta (%) para activar la detección de stall.
/// Por debajo de este valor se asume que el motor está intencionalmente parado.
const STALL_SPEED_MIN: i16 = 20;

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

/// Aplica msm.drive al rover; en FAULT/STANDBY para todos los motores.
macro_rules! sync_drive {
    ($rover:expr, $msm:expr) => {
        match $msm.state {
            RoverState::Fault | RoverState::Standby => $rover.stop(),
            _ => $rover.set_speeds($msm.drive.left, $msm.drive.right),
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

    // ── Interrupciones externas INT0–INT5 (rising edge) ──────────────────────
    // EICRA: controla INT0–INT3 (ISCn1=1, ISCn0=1 → rising edge)
    // EICRB: controla INT4–INT5 (ISCn1=1, ISCn0=1 → rising edge)
    // EIMSK: habilita INT0–INT5 (bits 0–5)
    dp.EXINT.eicra().write(|w| unsafe { w.bits(0xFF) });
    dp.EXINT.eicrb().write(|w| unsafe { w.bits(0x0F) });
    dp.EXINT.eimsk().write(|w| unsafe { w.bits(0x3F) });
    unsafe { avr_device::interrupt::enable() };

    // ── Estado del loop ──────────────────────────────────────────────────────
    let mut msm          = MasterStateMachine::new();
    let mut resp_buf     = [0u8; RESP_BUF];
    let mut tlm_counter: u8  = 0;
    let mut hc_counter:  u8  = 0;

    // Estado de stall por encoder (parallel al stall_mask de la MSM)
    let mut last_counts  = [0i32; 6];
    let mut stall_timers = [0u16; 6];

    iface.log("=== ROVER OLYMPUS v2.2 — MSM + HC-SR04 + ENCODERS ===");

    // ── Bucle principal ───────────────────────────────────────────────────────
    loop {
        // 1. Watchdog de comunicación
        if let Some(wdog_resp) = msm.tick() {
            sync_drive!(rover, msm);
            iface.send_response(format_response(wdog_resp, &mut resp_buf));
        }

        // 2. HC-SR04 — D38(Trigger) D39(Echo), lectura cada HC_READ_PERIOD ciclos (~100 ms)
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
                iface.send_response(format_response(msm.telemetry(stall_mask), &mut resp_buf));
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

        // 5. Telemetría periódica (~1 s) — stall_mask siempre 0 aquí porque
        //    si hubiera stall ya se reportó en el bloque de encoders arriba.
        tlm_counter = tlm_counter.wrapping_add(1);
        if tlm_counter >= TLM_PERIOD {
            tlm_counter = 0;
            iface.send_response(format_response(msm.telemetry(0), &mut resp_buf));
        }

        // delay_ms restaurado: la ISR USART0_RX garantiza que ningún byte
        // se pierde durante el bloqueo. Ver docs/debug_usart_overflow.md.
        arduino_hal::delay_ms(LOOP_MS);
    }
}
