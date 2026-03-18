// Version: v1.0
//! # Ejemplo: Prueba de Encoders de Efecto Hall
//!
//! Este programa configura el pin D21 (INT0) para detectar pulsos de un encoder.
//! Cada vez que el sensor detecta un cambio (imán pasando), se incrementa un contador.
//!
//! Conexión:
//! * Sensor OUT -> Pin D21 (Arduino Mega)
//! * Sensor VCC -> 5V
//! * Sensor GND -> GND

#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use panic_halt as _;
use rover_low_level_controller::sensors::HallEncoder;
use rover_low_level_controller::sensors::Encoder;
use ufmt::uwriteln;

/// Instancia global del encoder para que sea accesible desde la ISR.
/// Usamos un HallEncoder que maneja Mutex internos.
static ENCODER_FR: HallEncoder = HallEncoder::new();

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    
    // Configurar Serial para ver los resultados
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    // --- CONFIGURACIÓN DE INTERRUPCIONES EXTERNAS (HARDWARE) ---
    // Pin D21 es INT0 en el ATmega2560 (Puerto D, Pin 0)
    // Configuramos el pin como entrada con pull-up para evitar ruido
    let _ = pins.d21.into_pull_up_input();

    // Configuramos EICRA (External Interrupt Control Register A)
    // 11: Rising Edge. El método bits() es unsafe.
    dp.EXINT.eicra().modify(|_, w| unsafe { w.isc0().bits(0x03) }); 

    // Habilitamos INT0 en EIMSK (External Interrupt Mask Register)
    // El método bits() es unsafe.
    dp.EXINT.eimsk().modify(|_, w| unsafe { w.int().bits(0x01) }); 

    // Habilitar interrupciones globales
    unsafe { avr_device::interrupt::enable() };

    uwriteln!(&mut serial, "Prueba de Encoder Hall Iniciada (D21 - INT0)").unwrap();

    loop {
        // Obtenemos el valor actual del contador
        let counts = ENCODER_FR.get_counts();
        
        // Imprimimos el valor cada 500ms
        uwriteln!(&mut serial, "Pulsos detectados: {}", counts).unwrap();
        
        arduino_hal::delay_ms(500);
    }
}

#[avr_device::interrupt(atmega2560)]
fn INT0() {
    // Incrementamos el contador del encoder
    ENCODER_FR.pulse();
}
