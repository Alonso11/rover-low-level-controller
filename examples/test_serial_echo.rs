// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Inicializamos la consola serial a 115200 baudios
    // En el Mega, USART0 usa los pines D0 (RX) y D1 (TX), que van al USB.
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    ufmt::uwriteln!(&mut serial, "Arduino Mega listo para Yocto!\r").unwrap();

    loop {
        // Leemos un byte
        let byte = nb::block!(serial.read()).unwrap();
        
        // Respondemos con el mismo byte para confirmar recepcion
        ufmt::uwrite!(&mut serial, "Recibido: {}\r\n", byte).unwrap();
    }
}
