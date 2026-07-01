// Version: v1.0
//! # Driver MPU-6050 — Acelerómetro y Giroscopio de 6 ejes
//!
//! Driver para el sensor inercial MPU-6050 vía I2C software en D42/D43.
//! Provee velocidad angular (rad/s) y aceleración (m/s²).

use super::soft_i2c::SoftI2C;

// ─── Dirección I2C (AD0=GND) ───────────────────────────────────────────────
const ADDR: u8 = 0x68;

// ─── Registros ───────────────────────────────────────────────────────────────
const REG_CONFIG:      u8 = 0x1A;
const REG_GYRO_CONFIG: u8 = 0x1B;
const REG_ACCEL_CONFIG:u8 = 0x1C;
const REG_ACCEL_XOUT_H:u8 = 0x3B;
const REG_PWR_MGMT_1:  u8 = 0x6B;
const REG_WHO_AM_I:    u8 = 0x75;

// ─── Escalas ─────────────────────────────────────────────────────────────────
/// Escala ±2g: 16384 LSB/g -> m/s²
pub const ACCEL_SCALE: f32 = 9.80665 / 16384.0;
/// Escala ±250°/s: 131 LSB/(°/s) -> rad/s
pub const GYRO_SCALE:  f32 = (3.14159265 / 180.0) / 131.0;

/// Driver para el sensor inercial MPU-6050 por I2C software.
pub struct MPU6050 {
    i2c: SoftI2C,
    pub ready: bool,
    addr: u8,
}

impl MPU6050 {
    pub fn new() -> Self {
        MPU6050 { i2c: SoftI2C::new(), ready: false, addr: ADDR }
    }

    /// Inicializa el sensor. Prueba 0x68 y 0x69 (AD0 alto).
    /// Retorna el byte WHO_AM_I leído (0 si falla). Acepta cualquier
    /// valor != 0 para soportar clones que devuelven 0x70/0x72/etc.
    pub fn init(&mut self) -> u8 {
        for &candidate in &[0x68u8, 0x69u8] {
            let mut who_am_i = [0u8; 1];
            if self.i2c.read(candidate, REG_WHO_AM_I, &mut who_am_i) && who_am_i[0] != 0 {
                self.addr = candidate;
                // 1. Wake up (PWR_MGMT_1 = 0)
                self.i2c.write(self.addr, REG_PWR_MGMT_1, &[0x00]);
                // 2. Set gyro scale ±250 deg/s (GYRO_CONFIG = 0)
                self.i2c.write(self.addr, REG_GYRO_CONFIG, &[0x00]);
                // 3. Set accel scale ±2g (ACCEL_CONFIG = 0)
                self.i2c.write(self.addr, REG_ACCEL_CONFIG, &[0x00]);
                // 4. Set DLPF ~44Hz (CONFIG = 0x03)
                self.i2c.write(self.addr, REG_CONFIG, &[0x03]);
                self.ready = true;
                return who_am_i[0];
            }
        }
        0
    }

    /// Ax, Ay, Az, Temp, Gx, Gy, Gz (cada uno 16-bit big-endian).
    pub fn read_raw(&self) -> Option<(i16, i16, i16, i16, i16, i16, i16)> {
        if !self.ready { return None; }
        let mut buf = [0u8; 14];
        if !self.i2c.read(self.addr, REG_ACCEL_XOUT_H, &mut buf) {
            return None;
        }

        let ax = ((buf[0] as i16) << 8) | buf[1] as i16;
        let ay = ((buf[2] as i16) << 8) | buf[3] as i16;
        let az = ((buf[4] as i16) << 8) | buf[5] as i16;
        let t  = ((buf[6] as i16) << 8) | buf[7] as i16;
        let gx = ((buf[8] as i16) << 8) | buf[9] as i16;
        let gy = ((buf[10] as i16) << 8) | buf[11] as i16;
        let gz = ((buf[12] as i16) << 8) | buf[13] as i16;

        Some((ax, ay, az, t, gx, gy, gz))
    }
}
