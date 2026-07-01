// Version: v1.0 (HW DEBUG — HC-SR04 en pines REALES del rover D38/D39)
//! Verifica el HC-SR04 del rover usando EXACTAMENTE el mismo driver y pines que
//! main.rs (D38 Trig / D39 Echo), pero con timeout completo (~4 m) y loop
//! continuo. Sirve para confirmar que el sensor mide al acercar/alejar un
//! obstáculo (en la integración leyó 0 con timeout corto de ~300 mm).
//!
//! Cada ~300 ms imprime una de:
//!   - `dist_mm=NNN`        → medición válida (2–4000 mm)
//!   - `ERR=timeout`        → el eco no volvió (sin objeto en rango / sin energía / mal cableado)
//!   - `ERR=out_of_range`   → eco fuera de 2–4000 mm
//!
//! Mueve la mano frente al sensor: dist_mm debe seguirla. NO mueve motores.
//!
//! Pinout (= main.rs): Trig=D38, Echo=D39.
//!
//! Flash: `make flash-debug-hcsr04-d38 PORT=/dev/ttyACM2`

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::sensors::hc_sr04::HCSR04;
use rover_low_level_controller::sensors::SensorError;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# debug_hcsr04_d38 v1.0 - HC-SR04 D38(Trig)/D39(Echo), rango completo");
    let _ = ufmt::uwriteln!(&mut serial, "# mueve la mano: dist_mm debe seguirla");

    // Mismo patrón que main.rs / integration. Timeout por defecto = 30 ms (~4 m).
    let mut hcsr04 = HCSR04::new(
        pins.d38.into_output(),
        pins.d39.into_floating_input().forget_imode(),
    );

    let mut n: u32 = 0;
    loop {
        match hcsr04.measure_mm() {
            Ok(mm)                        => { let _ = ufmt::uwriteln!(&mut serial, "{},dist_mm={}", n, mm); }
            Err(SensorError::Timeout)     => { let _ = ufmt::uwriteln!(&mut serial, "{},ERR=timeout", n); }
            Err(SensorError::OutOfRange)  => { let _ = ufmt::uwriteln!(&mut serial, "{},ERR=out_of_range", n); }
            Err(_)                        => { let _ = ufmt::uwriteln!(&mut serial, "{},ERR=other", n); }
        }
        n += 1;
        arduino_hal::delay_ms(300);
    }
}
