// Version: v1.0
#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Inicializamos el Serial 3 (TX3=D14/PJ1, RX3=D15/PJ0) a 115200 baudios
    // USART1 (D18/D19) queda libre para encoders INT2/INT3 de los motores centrales.
    let mut serial = arduino_hal::Usart::new(
        dp.USART3,
        pins.d15.into_pull_up_input(),
        pins.d14.into_output(),
        arduino_hal::hal::usart::BaudrateExt::into_baudrate(115200),
    );

    ufmt::uwriteln!(&mut serial, "Conexion con RPi5 establecida por GPIO UART (USART3)!\r").unwrap();

    loop {
        // Leemos de la RPi5
        let byte = nb::block!(serial.read()).unwrap();
        
        // Respondemos a la RPi5
        ufmt::uwrite!(&mut serial, "RPi5 envio: {}\r\n", byte).unwrap();
    }
}
