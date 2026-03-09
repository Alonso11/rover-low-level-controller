#![no_std]
#![no_main]

use panic_halt as _;
use arduino_hal::prelude::*;
use arduino_hal::simple_pwm::IntoPwmPin;
use ufmt::uwriteln;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // --- Serial (Comunicacion con Pi 5) ---
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);
    let _ = uwriteln!(serial, "Rover 6-Wheels Controller [Ready]");

    // --- Configuracion de Timers para 6 PWMs ---
    let timer3 = arduino_hal::simple_pwm::Timer3Pwm::new(dp.TC3, arduino_hal::simple_pwm::Prescaler::Prescale64);
    let timer4 = arduino_hal::simple_pwm::Timer4Pwm::new(dp.TC4, arduino_hal::simple_pwm::Prescaler::Prescale64);

    // --- MOTORES IZQUIERDOS (Timer 3) ---
    let mut pwm_lf = pins.d2.into_output().into_pwm(&timer3); // Left Front
    let mut dir_lf1 = pins.d22.into_output();
    let mut dir_lf2 = pins.d23.into_output();

    let mut pwm_lm = pins.d3.into_output().into_pwm(&timer3); // Left Middle
    let mut dir_lm1 = pins.d24.into_output();
    let mut dir_lm2 = pins.d25.into_output();

    let mut pwm_lr = pins.d5.into_output().into_pwm(&timer3); // Left Rear
    let mut dir_lr1 = pins.d26.into_output();
    let mut dir_lr2 = pins.d27.into_output();

    // --- MOTORES DERECHOS (Timer 4) ---
    let mut pwm_rf = pins.d6.into_output().into_pwm(&timer4); // Right Front
    let mut dir_rf1 = pins.d28.into_output();
    let mut dir_rf2 = pins.d29.into_output();

    let mut pwm_rm = pins.d7.into_output().into_pwm(&timer4); // Right Middle
    let mut dir_rm1 = pins.d30.into_output();
    let mut dir_rm2 = pins.d31.into_output();

    let mut pwm_rr = pins.d8.into_output().into_pwm(&timer4); // Right Rear
    let mut dir_rr1 = pins.d32.into_output();
    let mut dir_rr2 = pins.d33.into_output();

    // Habilitar todos los PWMs
    pwm_lf.enable(); pwm_lm.enable(); pwm_lr.enable();
    pwm_rf.enable(); pwm_rm.enable(); pwm_rr.enable();

    let mut led = pins.d13.into_output();

    loop {
        if let Ok(byte) = serial.read() {
            match byte {
                b'M' => { // Mover Adelante (Los 6)
                    dir_lf1.set_high(); dir_lf2.set_low(); pwm_lf.set_duty(150);
                    dir_lm1.set_high(); dir_lm2.set_low(); pwm_lm.set_duty(150);
                    dir_lr1.set_high(); dir_lr2.set_low(); pwm_lr.set_duty(150);
                    
                    dir_rf1.set_high(); dir_rf2.set_low(); pwm_rf.set_duty(150);
                    dir_rm1.set_high(); dir_rm2.set_low(); pwm_rm.set_duty(150);
                    dir_rr1.set_high(); dir_rr2.set_low(); pwm_rr.set_duty(150);
                    
                    let _ = uwriteln!(serial, "ACK: 6 Motors Moving");
                }
                b'S' => { // Stop (Los 6)
                    pwm_lf.set_duty(0); pwm_lm.set_duty(0); pwm_lr.set_duty(0);
                    pwm_rf.set_duty(0); pwm_rm.set_duty(0); pwm_rr.set_duty(0);
                    let _ = uwriteln!(serial, "ACK: 6 Motors Stopped");
                }
                _ => { let _ = uwriteln!(serial, "ERR: Unknown"); }
            }
            led.toggle();
        }
    }
}

#[no_mangle]
pub extern "C" fn _exit() -> ! { loop {} }
#[no_mangle]
pub extern "C" fn exit() -> ! { loop {} }
