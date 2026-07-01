#![no_std]
#![no_main]

//! # Prueba del Driver HC-SR04 con Resiliencia de Datos (Filtro v1.1)
//!
//! Este ejemplo demuestra cómo el driver filtrado en `src/sensors/hc_sr04.rs`
//! estabiliza la lectura de sensores que presentan fallos intermitentes.
//!
//! ### Justificación:
//! Durante las pruebas, se observó que los sensores HC-SR04 fallan por "timeout"
//! de forma impredecible. Para evitar que esto detenga la lógica del Rover,
//! el driver ahora devuelve la última distancia válida medida con éxito.
//!
//! ### Comportamiento:
//! 1. Si el sensor falla, se sigue devolviendo el último valor correcto.
//! 2. Si el sensor falla **6 veces consecutivas**, entonces se devuelve `None`.
//!
//! Pines: Trigger D40, Echo D41.

use panic_halt as _;
use rover_low_level_controller::sensors::hc_sr04::HCSR04;
use rover_low_level_controller::sensors::ProximitySensor;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    
    // Usamos el driver con la nueva lógica de filtrado
    // Trigger: D40, Echo: D41
    let mut sensor = HCSR04::new(
        pins.d40.into_output(),
        pins.d41.into_floating_input().forget_imode(),
    );

    ufmt::uwriteln!(&mut serial, "--- HC-SR04 Filtered Driver Test D40/D41 ---\r").unwrap();

    loop {
        match sensor.get_distance_mm() {
            Some(mm) => {
                ufmt::uwriteln!(&mut serial, "Distancia: {} mm\r", mm).unwrap();
            }
            None => {
                ufmt::uwriteln!(&mut serial, "ERROR: Lectura invalida (o >5 fallos seguidos)\r").unwrap();
            }
        }

        // Esperamos 200ms entre lecturas (el driver ya es robusto)
        arduino_hal::delay_ms(200);
    }
}
