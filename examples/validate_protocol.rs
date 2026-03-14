// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::command_interface::CommandInterface;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Para esta validación, usamos el puerto USB (USART0)
    // Esto nos permite probar el protocolo desde la PC directamente.
    let serial = arduino_hal::default_serial!(dp, pins, 115200);
    let mut interface = CommandInterface::new(serial);

    interface.log("--- VALIDADOR DE PROTOCOLO ---");
    interface.log("Escribe 'F' (Forward), 'B' (Backward) o 'S' (Stop)");

    loop {
        if interface.poll_command() {
            let cmd = interface.get_command();
            
            // Verificamos qué recibimos (el primer byte)
            match cmd[0] {
                b'F' | b'f' => {
                    interface.log("ORDEN RECIBIDA: Avanzar Motores");
                },
                b'B' | b'b' => {
                    interface.log("ORDEN RECIBIDA: Retroceder Motores");
                },
                b'S' | b's' => {
                    interface.log("ORDEN RECIBIDA: Detener TODO");
                },
                _ => {
                    interface.log("ERROR: Caracter no reconocido");
                }
            }
        }
    }
}
