// Version: v1.1
//! # Example: Proximity Sensor Test (HC-SR04 and TF-Luna)
//!
//! This example program demonstrates how to simultaneously use an ultrasonic
//! sensor and a LiDAR sensor for obstacle detection on the Rover.
//!
//! ## Suggested Connections (Arduino Mega 2560):
//! * **HC-SR04 (Ultrasound):**
//!     - Trigger -> Digital Pin D38 (PD7)
//!     - Echo    -> Digital Pin D39 (PG2)
//! * **TF-Luna (LiDAR):**
//!     - RX (sensor side) -> Pin D16 (Arduino TX2)
//!     - TX (sensor side) -> Pin D17 (Arduino RX2)
//! * **Note:** D14/D15 are reserved for the RPi5 (USART3). Do not use them for HC-SR04.
//! * **Debug Serial (USB):**
//!     - 115200 baud via the standard USB port.

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::sensors::{HCSR04, TFLuna, ProximitySensor};

#[arduino_hal::entry]
fn main() -> ! {
    // Acquire peripherals and pins
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Initialise the debug console
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    ufmt::uwriteln!(&mut serial, "--- Olympus Rover: Proximity System v1.0 ---\r").unwrap();

    // HC-SR04 (ultrasound) setup.
    // forget_imode() converts the Echo pin into a generic input type.
    let mut hc_sr04 = HCSR04::new(
        pins.d38.into_output(),
        pins.d39.into_floating_input().forget_imode(),
    );
    ufmt::uwriteln!(&mut serial, "[INFO] HC-SR04 ready on D38(T)/D39(E)\r").unwrap();

    // TF-Luna (LiDAR) setup on Serial2 (USART2).
    // The LiDAR defaults to 115200 baud.
    let serial2 = arduino_hal::Usart::new(
        dp.USART2,
        pins.d17.into_floating_input(),
        pins.d16.into_output(),
        115200.into(),
    );
    let mut tf_luna = TFLuna::new(serial2);
    ufmt::uwriteln!(&mut serial, "[INFO] TF-Luna ready on USART2 (D17/D16)\r").unwrap();

    loop {
        // Ultrasonic reading — get_distance_mm() returns Result<u16, SensorError>
        match hc_sr04.get_distance_mm() {
            Ok(dist) => {
                ufmt::uwrite!(&mut serial, "US: {} mm | ", dist).unwrap();
            }
            Err(_) => {
                ufmt::uwrite!(&mut serial, "US:  --  mm | ").unwrap();
            }
        }

        // LiDAR reading — get_distance_mm() returns Result<u16, SensorError>
        match tf_luna.get_distance_mm() {
            Ok(dist) => {
                ufmt::uwriteln!(&mut serial, "LiDAR: {} mm\r", dist).unwrap();
            }
            Err(_) => {
                ufmt::uwriteln!(&mut serial, "LiDAR:  --  mm\r").unwrap();
            }
        }

        // Brief pause so we don't flood the console and to keep sensor reads stable.
        arduino_hal::delay_ms(100);
    }
}