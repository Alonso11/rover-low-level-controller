// Version: v1.0
//! # Driver INA226 — Monitor de corriente, voltaje y potencia
//!
//! Mide la tensión del bus de batería y la corriente total del sistema
//! mediante una resistencia de shunt externa. Comparte el bus I2C software
//! en D42 (SDA/PL7) y D43 (SCL/PL6) con el VL53L0X (0x29).
//!
//! ## Características
//! | Parámetro | Valor |
//! |-----------|-------|
//! | Voltaje bus | 0 – 36 V |
//! | Corriente | Bidireccional, configurable por shunt |
//! | ADC | 16 bits |
//! | Precisión ganancia | ±0.1% (max.) |
//! | Interfaz | I2C, dirección 0x40 (A0=GND, A1=GND) |
//! | Voltaje operación | 2.7 – 5.5 V |
//!
//! ## Conexión física
//! - VBUS: positivo de batería (hasta 36 V, GND común)
//! - IN+ / IN−: terminales de la resistencia shunt (en serie con la carga)
//! - A0, A1: a GND → dirección I2C 0x40
//! - SDA/SCL: D42/D43 (bus soft I2C compartido con VL53L0X)
//!
//! ## Calibración
//! El driver fija `Current_LSB = 1 mA` para que el registro CURRENT
//! devuelva mA directamente sin conversión adicional.
//! `CAL = 5120 / shunt_mohm` (shunt en mΩ, aritmética entera exacta).
//!
//! ## Módulo de referencia
//! <https://componenteselectronicoscr.com/product/monitor-de-derivacion-de-corriente-y-potencia-con-inter>
//! (Texas Instruments INA226, datasheet SBOS547)

use super::soft_i2c::SoftI2C;

// ─── Dirección I2C (A0=GND, A1=GND) ─────────────────────────────────────────
const ADDR: u8 = 0x40;

// ─── Registros ───────────────────────────────────────────────────────────────
const REG_CONFIG:  u8 = 0x00;
const REG_VBUS:    u8 = 0x02;
const REG_POWER:   u8 = 0x03;
const REG_CURRENT: u8 = 0x04;
const REG_CAL:     u8 = 0x05;
const REG_DIE_ID:  u8 = 0xFF; // debe leer 0x2260

// ─── Configuración ───────────────────────────────────────────────────────────
// 0x4127 = power-on default:
//   Bits 14:12 = 100 (reservado, reset value)
//   Bits 11:9  = 001 (AVG=4 promedios)
//   Bits 8:6   = 001 (VBUSCT=140 µs)
//   Bits 5:3   = 001 (VSHCT=140 µs)
//   Bits 2:0   = 111 (MODE=continuo shunt+bus)
// Frecuencia de actualización: ~280 µs × 4 promedios ≈ 1.1 ms/ciclo
const CONFIG_CONTINUOUS: u16 = 0x4127;

// ─── Driver ──────────────────────────────────────────────────────────────────

/// Driver para el monitor de potencia INA226 por I2C software.
pub struct INA226 {
    i2c: SoftI2C,
    /// `true` si `init()` verificó el die ID correctamente.
    pub ready: bool,
}

impl INA226 {
    /// Crea una instancia. Llamar a `init()` antes de leer.
    pub fn new() -> Self {
        INA226 { i2c: SoftI2C::new(), ready: false }
    }

    // ── Acceso a registros 16-bit ─────────────────────────────────────────────

    fn wr16(&self, reg: u8, val: u16) {
        self.i2c.write(ADDR, reg, &[(val >> 8) as u8, val as u8]);
    }

    fn rd16(&self, reg: u8) -> u16 {
        let mut buf = [0u8; 2];
        self.i2c.read(ADDR, reg, &mut buf);
        ((buf[0] as u16) << 8) | buf[1] as u16
    }

    // ── Inicialización ────────────────────────────────────────────────────────

    /// Configura el INA226 y lo deja en modo medición continua.
    ///
    /// `shunt_mohm`: resistencia de shunt en mΩ (p.ej. `10` para 0.01 Ω).
    ///
    /// Fija `Current_LSB = 1 mA` → el registro CURRENT devuelve mA directamente.
    /// Fórmula de calibración: `CAL = 5120 / shunt_mohm`.
    ///
    /// Retorna `true` si el chip respondió con el die ID correcto (0x2260).
    pub fn init(&mut self, shunt_mohm: u16) -> bool {
        if self.rd16(REG_DIE_ID) != 0x2260 {
            return false;
        }
        self.wr16(REG_CONFIG, CONFIG_CONTINUOUS);
        let cal = (5120u32 / shunt_mohm as u32) as u16;
        self.wr16(REG_CAL, cal);
        self.ready = true;
        true
    }

    // ── Lecturas ──────────────────────────────────────────────────────────────

    /// Lee la tensión del bus en mV.
    ///
    /// LSB = 1.25 mV → `bus_mv = reg × 5 / 4` (aritmética entera u32).
    /// Rango: 0 – 36 000 mV.
    pub fn read_bus_mv(&self) -> u16 {
        let raw = self.rd16(REG_VBUS);
        (raw as u32 * 5 / 4) as u16
    }

    /// Lee la corriente en mA (con signo).
    ///
    /// Con `Current_LSB = 1 mA` el registro devuelve mA directamente.
    /// Negativo indica flujo de corriente invertido (carga → batería / regenerativo).
    pub fn read_current_ma(&self) -> i32 {
        self.rd16(REG_CURRENT) as i16 as i32
    }

    /// Lee la potencia calculada en mW.
    ///
    /// LSB = 25 × Current_LSB = 25 mW.
    pub fn read_power_mw(&self) -> u32 {
        self.rd16(REG_POWER) as u32 * 25
    }
}
