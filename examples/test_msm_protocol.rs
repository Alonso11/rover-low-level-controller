// Version: v1.0
//! # Test del Protocolo MSM — vía USB desde PC
//!
//! Prueba la Máquina de Estados Maestra con el protocolo completo.
//! Conectar el Arduino al PC por USB y abrir un terminal serie a 115200.
//!
//! Comandos de prueba:
//!   PING       → PONG
//!   STB        → ACK:STB
//!   EXP:80:80  → ACK:EXP  (drive izq=80, der=80)
//!   EXP:-50:50 → ACK:EXP  (giro derecha)
//!   AVD:L      → ACK:AVD  (evasión izquierda)
//!   AVD:R      → ACK:AVD  (evasión derecha)
//!   RET        → ACK:RET  (retreat)
//!   FLT        → ACK:FLT  (fault — bloquea motores)
//!   EXP:80:80  → ERR:ESTOP (bloqueado por FAULT)
//!   RST        → ACK:STB  (limpia fault)
//!   (silencio 2s) → ERR:WDOG (watchdog expira)

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::command_interface::CommandInterface;
use rover_low_level_controller::state_machine::{
    format_response, parse_command, MasterStateMachine, Response,
};

const RESP_BUF: usize = 24;
const LOOP_MS: u32 = 20;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // USB serial (USART0) para pruebas desde PC
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut iface = CommandInterface::new(serial);
    let mut msm = MasterStateMachine::new();
    let mut resp_buf = [0u8; RESP_BUF];

    iface.log("=== MSM PROTOCOL TEST v1.0 ===");
    iface.log("Comandos: PING STB EXP:<L>:<R> AVD:L AVD:R RET FLT RST");
    iface.log("Watchdog: silencio de ~2 s dispara ERR:WDOG");

    loop {
        // 1. Tick del watchdog (~20 ms por iteracion)
        if let Some(wdog_resp) = msm.tick() {
            let bytes = format_response(wdog_resp, &mut resp_buf);
            iface.send_response(bytes);
        }

        // 2. Procesar comandos entrantes
        if iface.poll_command() {
            let raw = iface.get_command();
            let response = match parse_command(raw) {
                Some(cmd) => msm.process(cmd),
                None      => Response::ErrUnknown,
            };
            let bytes = format_response(response, &mut resp_buf);
            iface.send_response(bytes);
        }

        arduino_hal::delay_ms(LOOP_MS);
    }
}
