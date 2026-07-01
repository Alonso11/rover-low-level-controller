// Version: v1.0
//! # Driver INA3221 — Monitor triple-canal de voltaje de batería
//!
//! Mide la tensión de tres bancos de batería independientes con un único chip.
//! Configurado en modo **bus-only** (sin medición de corriente) para evitar
//! quemar los shunts on-board de 100 mΩ (límite ADC: ±1.638 A) con las
//! corrientes de motor del rover, que pueden alcanzar 6.5 A de stall
//! (NFP-5840-31ZY-EN @ 12 V → P_shunt = 6.5² × 0.1 = 4.2 W >> 1/4 W del SMD).
//!
//! ## Topología "voltaje-solamente"
//!
//! Para cada canal, IN+n y IN-n se conectan **ambos** al (+) del banco
//! correspondiente. La corriente del banco va directamente a la carga,
//! NO pasa por las patas del chip. El shunt on-board queda con 0 V →
//! no disipa potencia, no se quema. El chip mide bus voltage en IN-n
//! contra GND (rango 0–26 V, LSB = 8 mV).
//!
//! ```text
//!  Banco N (+) ──┬─────────────► Carga (motores / buck DC-DC)
//!                │
//!                ├──► IN+n
//!                │      [R100 on-board: 0 V, sin corriente]
//!                └──► IN-n  ◄── bus voltage de la batería
//! ```
//!
//! ## Conexión física
//! | Señal | Pin/destino | Notas |
//! |-------|-------------|-------|
//! | VS (VCC) | 5 V Mega | rango 2.7–5.5 V |
//! | GND | GND común | unir con GND de los 3 bancos |
//! | SDA | D42 (PL7) | bus soft I2C compartido |
//! | SCL | D43 (PL6) | bus soft I2C compartido |
//! | A0 | GND | dirección I2C = 0x40 |
//! | CH1 IN+/IN- | (+) del Banco 1 | ambos pines al MISMO punto |
//! | CH2 IN+/IN- | (+) del Banco 2 | idem |
//! | CH3 IN+/IN- | (+) del Banco 3 | idem |
//!
//! ## Características
//! | Parámetro | Valor |
//! |-----------|-------|
//! | Voltaje bus | 0 – 26 V por canal |
//! | Resolución bus voltage | 8 mV/LSB (13-bit) |
//! | Canales | 3 independientes |
//! | Voltaje supply | 2.7 – 5.5 V |
//! | Interfaz | I2C, 0x40 (A0=GND) |
//!
//! ## Limites absolutos (NO violar)
//! - V max en IN+/IN-/GND: **+26 V** (12 V por banco → margen ×2)
//! - V min en IN+/IN-: −0.3 V (no invertir polaridad — destruye el chip)
//!
//! ## Identificación
//! El chip responde a `REG_DIE_ID` (0xFF) con `0x3220` y a `REG_MANUFACTURER`
//! (0xFE) con `0x5449` ("TI"). El `init()` valida el die ID para confirmar
//! comunicación correcta antes de configurar el modo.
//!
//! Ref.: Texas Instruments INA3221 datasheet SBOS576 (rev. June 2016).

use super::soft_i2c::SoftI2C;

// ─── Dirección I2C (A0=GND) ──────────────────────────────────────────────────
const ADDR: u8 = 0x40;

// ─── Registros ───────────────────────────────────────────────────────────────
const REG_CONFIG:       u8 = 0x00;
const REG_BUS_CH1:      u8 = 0x02;
const REG_BUS_CH2:      u8 = 0x04;
const REG_BUS_CH3:      u8 = 0x06;
const REG_MANUFACTURER: u8 = 0xFE; // 0x5449 = "TI"
const REG_DIE_ID:       u8 = 0xFF; // 0x3220

/// Valor esperado en `REG_DIE_ID` para validar comunicación.
const DIE_ID_EXPECTED: u16 = 0x3220;

// ─── Configuración ───────────────────────────────────────────────────────────
//
// 0x7726 decompuesto:
//   bit 15      = 0     → no reset
//   bits 14:12  = 111   → CH1, CH2, CH3 habilitados
//   bits 11:9   = 011   → AVG=64 muestras (rejection de ruido ~50/60 Hz)
//   bits 8:6    = 100   → VBUSCT = 1.1 ms (default)
//   bits 5:3    = 100   → VSHCT  = 1.1 ms (irrelevante en modo bus-only)
//   bits 2:0    = 110   → MODE = continuous bus voltage only (NO shunt)
//
// Tiempo de actualización por ciclo completo:
//   AVG × VBUSCT × N_canales = 64 × 1.1 ms × 3 ≈ 211 ms
//
// El loop de telemetría lee cada SEN_READ_PERIOD × LOOP_MS = 25 × 20 = 500 ms.
// 500 ms > 211 ms → siempre hay dato fresco entre lecturas.
const CONFIG_BUS_ONLY: u16 = 0x7726;

// ─── Driver ──────────────────────────────────────────────────────────────────

/// Driver para el monitor de voltaje triple-canal INA3221 vía soft I2C.
///
/// Configurado en **modo bus-only**: mide solo la tensión de los 3 bancos,
/// sin corriente. Imprescindible porque los shunts on-board (R100) no soportan
/// las corrientes del rover (stall 6.5 A vs 1.638 A máximo del ADC y
/// 4.2 W disipados vs 1/4 W del SMD).
pub struct INA3221 {
    i2c: SoftI2C,
    /// `true` si `init()` verificó el die ID correctamente.
    pub ready: bool,
}

impl INA3221 {
    /// Crea una instancia. Llamar a [`Self::init`] antes de leer.
    pub fn new() -> Self {
        INA3221 { i2c: SoftI2C::new(), ready: false }
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

    /// Configura el INA3221 en modo medición de voltaje continuo en los 3 canales.
    ///
    /// Retorna `true` si el chip respondió con el die ID correcto (0x3220).
    /// Si retorna `false`: comprobar cableado SDA/SCL/VCC/GND, dirección I2C
    /// (A0 a GND) y pull-ups del bus.
    pub fn init(&mut self) -> bool {
        if self.rd16(REG_DIE_ID) != DIE_ID_EXPECTED {
            return false;
        }
        self.wr16(REG_CONFIG, CONFIG_BUS_ONLY);
        self.ready = true;
        true
    }

    /// Lee el ID del fabricante (debe ser `0x5449` = "TI"). Útil para diagnóstico.
    pub fn read_manufacturer_id(&self) -> u16 {
        self.rd16(REG_MANUFACTURER)
    }

    /// Lee el die ID (debe ser `0x3220`). Útil para diagnóstico.
    pub fn read_die_id(&self) -> u16 {
        self.rd16(REG_DIE_ID)
    }

    // ── Lecturas de voltaje ───────────────────────────────────────────────────

    /// Convierte el registro raw del bus voltage a mV.
    ///
    /// El registro es 16-bit con bits 2:0 reservados (siempre 0) y bit 15 de signo.
    /// Bits 14:3 contienen el valor de 12 bits con LSB = 8 mV. Como bits 2:0
    /// son 0, el registro raw ya representa directamente el valor en mV.
    /// La máscara `0x7FF8` descarta el bit de signo (siempre 0 para bus voltage
    /// en operación normal) y los bits 2:0 reservados.
    #[inline]
    fn raw_to_mv(raw: u16) -> u16 {
        raw & 0x7FF8
    }

    /// Lee el voltaje del Banco 1 (canal 1) en mV. Rango: 0 – 26 000 mV.
    pub fn read_bank1_mv(&self) -> u16 {
        Self::raw_to_mv(self.rd16(REG_BUS_CH1))
    }

    /// Lee el voltaje del Banco 2 (canal 2) en mV. Rango: 0 – 26 000 mV.
    pub fn read_bank2_mv(&self) -> u16 {
        Self::raw_to_mv(self.rd16(REG_BUS_CH2))
    }

    /// Lee el voltaje del Banco 3 (canal 3) en mV. Rango: 0 – 26 000 mV.
    pub fn read_bank3_mv(&self) -> u16 {
        Self::raw_to_mv(self.rd16(REG_BUS_CH3))
    }

    /// Lee los tres bancos en una sola llamada. Devuelve `[V_B1, V_B2, V_B3]` en mV.
    pub fn read_all_mv(&self) -> [u16; 3] {
        [self.read_bank1_mv(), self.read_bank2_mv(), self.read_bank3_mv()]
    }
}

impl Default for INA3221 {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests x86 (lógica pura: conversión raw → mV) ────────────────────────────

#[cfg(test)]
mod tests {
    use super::INA3221;

    #[test]
    fn raw_to_mv_zero() {
        assert_eq!(INA3221::raw_to_mv(0x0000), 0);
    }

    #[test]
    fn raw_to_mv_lsb_8mv() {
        // Bit 3 = 1 → 8 mV (LSB del bus voltage)
        assert_eq!(INA3221::raw_to_mv(0x0008), 8);
    }

    #[test]
    fn raw_to_mv_strips_reserved_bits() {
        // Bits 2:0 siempre deben ignorarse aunque vinieran con basura.
        assert_eq!(INA3221::raw_to_mv(0x0007), 0);
        assert_eq!(INA3221::raw_to_mv(0x000F), 0x0008);
    }

    #[test]
    fn raw_to_mv_typical_12v_bank() {
        // 12 000 mV = 0x2EE0 → registro lee 0x2EE0 (bits 2:0 ya en 0).
        assert_eq!(INA3221::raw_to_mv(0x2EE0), 12_000);
    }

    #[test]
    fn raw_to_mv_max_26v() {
        // 26 000 mV = 0x6590 (el max del rango bus voltage).
        assert_eq!(INA3221::raw_to_mv(0x6590), 26_000);
    }

    #[test]
    fn raw_to_mv_masks_sign_bit() {
        // Bit 15 siempre 0 para bus voltage en operación normal.
        // Si por error viniera a 1, la máscara debe descartarlo.
        assert_eq!(INA3221::raw_to_mv(0xFFFF), 0x7FF8);
    }
}
