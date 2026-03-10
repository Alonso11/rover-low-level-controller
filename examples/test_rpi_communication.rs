#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Inicializamos el Serial 1 (TX1=D18, RX1=D19) a 115200 baudios
    let mut serial = arduino_hal::Usart::new(
        dp.USART1,
        pins.d19.into_pull_up_input(),
        pins.d18.into_output(),
        arduino_hal::hal::usart::BaudrateExt::into_baudrate(115200),
    );

    ufmt::uwriteln!(&mut serial, "Conexion con RPi5 establecida por GPIO UART!\r").unwrap();

    loop {
        // Leemos de la RPi5
        let byte = nb::block!(serial.read()).unwrap();
        
        // Respondemos a la RPi5
        ufmt::uwrite!(&mut serial, "RPi5 envio: {}\r\n", byte).unwrap();
    }
}
