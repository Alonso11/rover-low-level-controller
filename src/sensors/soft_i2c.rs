// Version: v1.0
//! # Software I2C (bit-bang) — ATmega2560, D42/D43
//!
//! Implementación de I2C por software para evitar el conflicto del bus I2C
//! hardware (TWI en D20/D21) con los encoders de rueda (INT0/INT1).
//!
//! ## Pines
//! | Señal | Pin Arduino | Puerto | Bit |
//! |-------|-------------|--------|-----|
//! | SDA   | D42         | PL7    | 7   |
//! | SCL   | D43         | PL6    | 6   |
//!
//! ## Modo open-drain
//! Para simular open-drain en AVR:
//! - Drive LOW  → DDR=1 (output), PORT=0
//! - Release HIGH → DDR=0 (input), pull-up externo en el módulo lo lleva a HIGH
//!
//! ## Frecuencia
//! Con `delay_us(5)` a 16 MHz → semiperíodo ≈ 5 µs → ~80–100 kHz.
//! El VL53L0X soporta hasta 400 kHz; 100 kHz es suficiente.

use arduino_hal::delay_us;

// ─── Registros Port L (ATmega2560 data memory addresses) ─────────────────────
const PINL:  *const u8 = 0x109 as *const u8;
const DDRL:  *mut u8   = 0x10A as *mut u8;
const PORTL: *mut u8   = 0x10B as *mut u8;

const SDA_BIT: u8 = 7; // D42 = PL7
const SCL_BIT: u8 = 6; // D43 = PL6

// ─── Macros de manipulación de pines ─────────────────────────────────────────

macro_rules! sda_low {
    () => { unsafe {
        *PORTL &= !(1 << SDA_BIT); // PORT=0 (ya debería estarlo)
        *DDRL  |=   1 << SDA_BIT;  // output → drive LOW
    }}
}
macro_rules! sda_release {
    () => { unsafe { *DDRL &= !(1 << SDA_BIT); } } // input → pull-up externo = HIGH
}
macro_rules! scl_low {
    () => { unsafe {
        *PORTL &= !(1 << SCL_BIT);
        *DDRL  |=   1 << SCL_BIT;
    }}
}
macro_rules! scl_release {
    () => { unsafe { *DDRL &= !(1 << SCL_BIT); } }
}
macro_rules! sda_read {
    () => { unsafe { (*PINL >> SDA_BIT) & 1 == 1 } }
}

#[inline(always)]
fn hp() { delay_us(5); } // half-period ≈ 5 µs

// ─── Driver ──────────────────────────────────────────────────────────────────

/// Bus I2C por software fijo en D42 (SDA) y D43 (SCL).
pub struct SoftI2C;

impl SoftI2C {
    /// Inicializa los pines en estado idle (ambas líneas liberadas = HIGH).
    pub fn new() -> Self {
        unsafe {
            // PORT=0: nunca driven HIGH activamente (open-drain puro)
            *PORTL &= !(1 << SDA_BIT | 1 << SCL_BIT);
            // DDR=0: input inicialmente (líneas sueltas, pull-up externo)
            *DDRL  &= !(1 << SDA_BIT | 1 << SCL_BIT);
        }
        SoftI2C
    }

    // ── Condiciones de bus ────────────────────────────────────────────────────

    fn start(&self) {
        sda_release!(); scl_release!();
        hp();
        sda_low!();  // SDA cae mientras SCL=H → START
        hp();
        scl_low!();
    }

    fn restart(&self) {
        sda_release!(); hp();
        scl_release!(); hp();
        sda_low!();     hp();
        scl_low!();
    }

    fn stop(&self) {
        sda_low!();
        hp();
        scl_release!();
        hp();
        sda_release!(); // SDA sube mientras SCL=H → STOP
        hp();
    }

    // ── Transferencia de bits ─────────────────────────────────────────────────

    /// Escribe un byte MSB-first. Retorna `true` si el esclavo respondió ACK.
    fn write_byte(&self, byte: u8) -> bool {
        for bit in (0..8).rev() {
            if (byte >> bit) & 1 == 1 { sda_release!(); } else { sda_low!(); }
            hp();
            scl_release!(); hp();
            scl_low!();
        }
        // Leer ACK del esclavo
        sda_release!(); hp();
        scl_release!(); hp();
        let ack = !sda_read!(); // LOW = ACK
        scl_low!();
        ack
    }

    /// Lee un byte MSB-first. Envía ACK si `send_ack=true`, NACK si `false`.
    fn read_byte(&self, send_ack: bool) -> u8 {
        let mut byte = 0u8;
        sda_release!();
        for _ in 0..8 {
            byte <<= 1;
            hp();
            scl_release!(); hp();
            if sda_read!() { byte |= 1; }
            scl_low!();
        }
        // Enviar ACK/NACK
        if send_ack { sda_low!(); } else { sda_release!(); }
        hp();
        scl_release!(); hp();
        scl_low!();
        sda_release!();
        byte
    }

    // ── API pública: transacciones completas ──────────────────────────────────

    /// Escribe `data` a `reg` del dispositivo con dirección de 7 bits `addr`.
    pub fn write(&self, addr: u8, reg: u8, data: &[u8]) {
        self.start();
        self.write_byte(addr << 1);  // write mode
        self.write_byte(reg);
        for &b in data { self.write_byte(b); }
        self.stop();
    }

    /// Lee `buf.len()` bytes desde `reg` del dispositivo `addr`.
    pub fn read(&self, addr: u8, reg: u8, buf: &mut [u8]) {
        self.start();
        self.write_byte(addr << 1);        // write mode — set register pointer
        self.write_byte(reg);
        self.restart();
        self.write_byte((addr << 1) | 1);  // read mode
        let last = buf.len().saturating_sub(1);
        for (i, b) in buf.iter_mut().enumerate() {
            *b = self.read_byte(i != last); // ACK todo menos el último
        }
        self.stop();
    }
}
