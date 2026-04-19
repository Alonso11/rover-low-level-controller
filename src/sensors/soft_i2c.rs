// Version: v1.2
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
//! Para simular open-drain en AVR con pull-ups EXTERNOS a 3.3V:
//! - Drive LOW  → DDR=1 (output), PORT=0
//! - Release HIGH → DDR=0 (input), PORT=0 (pull-up interno DESACTIVADO)
//!
//! IMPORTANTE: el pull-up interno del Mega (~50kΩ a 5V) NO debe activarse
//! cuando hay pull-ups externos a 3.3V. Si PORT=1 en modo input, los 50kΩ a 5V
//! pelean con las resistencias externas y elevan el bus a ~3.5V, por encima del
//! VCC del sensor (3.3V) → posible latch-up y fallo de comunicación.
//!
//! ## Frecuencia
//! Con `delay_us(5)` a 16 MHz → semiperíodo ≈ 5 µs → ~80–100 kHz.
//! El VL6180X soporta hasta 400 kHz; 100 kHz es suficiente.

use arduino_hal::delay_us;

// ─── Registros Port L (ATmega2560 data memory addresses) ─────────────────────
const PINL:  *const u8 = 0x109 as *const u8;
const DDRL:  *mut u8   = 0x10A as *mut u8;
const PORTL: *mut u8   = 0x10B as *mut u8;

const SDA_BIT: u8 = 7; // D42 = PL7
const SCL_BIT: u8 = 6; // D43 = PL6

/// Máximo de half-periods esperando clock-stretch (~500 × 5 µs = 2.5 ms).
const I2C_STRETCH_MAX: u16 = 500;

// ─── Macros de manipulación de pines ─────────────────────────────────────────

macro_rules! sda_low {
    () => { unsafe {
        *PORTL &= !(1 << SDA_BIT); // PORT=0 (ya debería estarlo)
        *DDRL  |=   1 << SDA_BIT;  // output → drive LOW
    }}
}
macro_rules! sda_release {
    () => { unsafe {
        *DDRL  &= !(1 << SDA_BIT); // input (hi-Z)
        *PORTL &= !(1 << SDA_BIT); // pull-up interno DESACTIVADO — pull-up externo a 3.3V eleva el bus
    }}
}
macro_rules! scl_low {
    () => { unsafe {
        *PORTL &= !(1 << SCL_BIT);
        *DDRL  |=   1 << SCL_BIT;
    }}
}
macro_rules! scl_release {
    () => { unsafe {
        *DDRL  &= !(1 << SCL_BIT); // input (hi-Z)
        *PORTL &= !(1 << SCL_BIT); // pull-up interno DESACTIVADO — pull-up externo a 3.3V eleva el bus
    }}
}
macro_rules! sda_read {
    () => { unsafe { (*PINL >> SDA_BIT) & 1 == 1 } }
}
macro_rules! scl_read {
    () => { unsafe { (*PINL >> SCL_BIT) & 1 == 1 } }
}

#[inline(always)]
fn hp() { delay_us(5); } // half-period ≈ 5 µs

// ─── Driver ──────────────────────────────────────────────────────────────────

/// Bus I2C por software fijo en D42 (SDA) y D43 (SCL).
pub struct SoftI2C;

impl SoftI2C {
    /// Espera a que SCL suba (clock-stretch del esclavo).
    /// Retorna `false` si supera `I2C_STRETCH_MAX` half-periods (~2.5 ms).
    fn wait_scl_high(&self) -> bool {
        for _ in 0..I2C_STRETCH_MAX {
            if scl_read!() { return true; }
            hp();
        }
        false
    }

    /// Inicializa los pines en estado idle (hi-Z). Requiere pull-ups externos a 3.3V.
    pub fn new() -> Self {
        unsafe {
            // DDR=0 (input) + PORT=0 (pull-up interno desactivado) → hi-Z
            // Los pull-ups externos (4.7kΩ SDA, 10kΩ SCL a 3.3V) elevan el bus al nivel correcto.
            *DDRL  &= !(1 << SDA_BIT | 1 << SCL_BIT);
            *PORTL &= !(1 << SDA_BIT | 1 << SCL_BIT);
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
    /// Retorna `false` si el esclavo no respondió ACK **o** clock-stretch timeout.
    fn write_byte(&self, byte: u8) -> bool {
        for bit in (0..8).rev() {
            if (byte >> bit) & 1 == 1 { sda_release!(); } else { sda_low!(); }
            hp();
            scl_release!();
            if !self.wait_scl_high() { scl_low!(); return false; }
            hp();
            scl_low!();
        }
        // Leer ACK del esclavo
        sda_release!(); hp();
        scl_release!();
        if !self.wait_scl_high() { scl_low!(); return false; }
        hp();
        let ack = !sda_read!(); // LOW = ACK
        scl_low!();
        ack
    }

    /// Lee un byte MSB-first. Envía ACK si `send_ack=true`, NACK si `false`.
    /// Retorna `None` si clock-stretch timeout; `Some(byte)` si OK.
    fn read_byte(&self, send_ack: bool) -> Option<u8> {
        let mut byte = 0u8;
        sda_release!();
        for _ in 0..8 {
            byte <<= 1;
            hp();
            scl_release!();
            if !self.wait_scl_high() { scl_low!(); return None; }
            hp();
            if sda_read!() { byte |= 1; }
            scl_low!();
        }
        // Enviar ACK/NACK
        if send_ack { sda_low!(); } else { sda_release!(); }
        hp();
        scl_release!();
        if !self.wait_scl_high() { scl_low!(); sda_release!(); return None; }
        hp();
        scl_low!();
        sda_release!();
        Some(byte)
    }

    // ── API pública: transacciones completas ──────────────────────────────────

    /// Escribe `data` a `reg` (8 bits) del dispositivo con dirección de 7 bits `addr`.
    /// Retorna `false` si se detecta NACK o clock-stretch timeout.
    pub fn write(&self, addr: u8, reg: u8, data: &[u8]) -> bool {
        self.start();
        if !self.write_byte(addr << 1) { self.stop(); return false; }
        if !self.write_byte(reg)       { self.stop(); return false; }
        for &b in data {
            if !self.write_byte(b) { self.stop(); return false; }
        }
        self.stop();
        true
    }

    /// Lee `buf.len()` bytes desde `reg` (8 bits) del dispositivo `addr`.
    /// Retorna `false` si se detecta NACK o clock-stretch timeout.
    pub fn read(&self, addr: u8, reg: u8, buf: &mut [u8]) -> bool {
        self.start();
        if !self.write_byte(addr << 1)       { self.stop(); return false; }
        if !self.write_byte(reg)             { self.stop(); return false; }
        self.restart();
        if !self.write_byte((addr << 1) | 1) { self.stop(); return false; }
        let last = buf.len().saturating_sub(1);
        for (i, b) in buf.iter_mut().enumerate() {
            match self.read_byte(i != last) {
                Some(v) => *b = v,
                None    => { self.stop(); return false; }
            }
        }
        self.stop();
        true
    }

    /// Escribe `data` a `reg` (16 bits, MSB primero) — para sensores como VL6180X.
    pub fn write16(&self, addr: u8, reg: u16, data: &[u8]) -> bool {
        self.start();
        if !self.write_byte(addr << 1)          { self.stop(); return false; }
        if !self.write_byte((reg >> 8) as u8)   { self.stop(); return false; }
        if !self.write_byte((reg & 0xFF) as u8) { self.stop(); return false; }
        for &b in data {
            if !self.write_byte(b) { self.stop(); return false; }
        }
        self.stop();
        true
    }

    /// Lee `buf.len()` bytes desde `reg` (16 bits, MSB primero) — para VL6180X.
    pub fn read16(&self, addr: u8, reg: u16, buf: &mut [u8]) -> bool {
        self.start();
        if !self.write_byte(addr << 1)          { self.stop(); return false; }
        if !self.write_byte((reg >> 8) as u8)   { self.stop(); return false; }
        if !self.write_byte((reg & 0xFF) as u8) { self.stop(); return false; }
        self.restart();
        if !self.write_byte((addr << 1) | 1)    { self.stop(); return false; }
        let last = buf.len().saturating_sub(1);
        for (i, b) in buf.iter_mut().enumerate() {
            match self.read_byte(i != last) {
                Some(v) => *b = v,
                None    => { self.stop(); return false; }
            }
        }
        self.stop();
        true
    }
}
