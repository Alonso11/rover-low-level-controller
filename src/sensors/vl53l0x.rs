// Version: v1.0
//! # Driver VL53L0X — Sensor Time-of-Flight (GY-VL53L0XV2)
//!
//! Mide distancia por tiempo de vuelo láser (940 nm VCSEL).
//! Comunica por I2C software en D42 (SDA) y D43 (SCL) para evitar
//! conflicto con el bus I2C hardware del ATmega2560 (D20/D21 = encoders).
//!
//! ## Características
//! | Parámetro | Valor |
//! |-----------|-------|
//! | Rango     | 3 cm – 200 cm |
//! | Precisión | ±3% |
//! | Frecuencia| hasta 50 Hz (default ~5 Hz con timing budget amplio) |
//! | Interfaz  | I2C, dirección 0x29 |
//! | Voltaje   | 3.3 V / 5 V (módulo GY incluye regulador y level-shifter) |
//!
//! ## Uso
//! ```ignore
//! let mut tof = VL53L0X::new();
//! if tof.init() {
//!     tof.start_continuous();
//!     loop {
//!         if let Some(mm) = tof.read_mm() {
//!             // usar distancia en mm
//!         }
//!     }
//! }
//! ```
//!
//! ## Referencia
//! Secuencia de inicialización derivada de la librería Pololu VL53L0X (MIT).
//! <https://github.com/pololu/vl53l0x-arduino>

use super::soft_i2c::SoftI2C;

// ─── Dirección I2C del VL53L0X ───────────────────────────────────────────────
const ADDR: u8 = 0x29;

// ─── Registros usados ────────────────────────────────────────────────────────
const SYSRANGE_START:                         u8 = 0x00;
const SYSTEM_SEQUENCE_CONFIG:                 u8 = 0x01;
const SYSTEM_INTERRUPT_CONFIG_GPIO:           u8 = 0x0A;
const SYSTEM_INTERRUPT_CLEAR:                 u8 = 0x0B;
const RESULT_INTERRUPT_STATUS:                u8 = 0x13;
const RESULT_RANGE_STATUS:                    u8 = 0x14;
const MSRC_CONFIG_CONTROL:                    u8 = 0x60;
const FINAL_RANGE_CONFIG_MIN_COUNT_RATE_RTN:  u8 = 0x44;
const GLOBAL_CONFIG_SPAD_ENABLES_REF_0:       u8 = 0xB0;
const GLOBAL_CONFIG_REF_EN_START_SELECT:      u8 = 0xB6;
const DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD:    u8 = 0x4E;
const GPIO_HV_MUX_ACTIVE_HIGH:                u8 = 0x84;
const VHV_CONFIG_PAD_SCL_SDA_EXTSUP_HV:       u8 = 0x89;

// Registro de resultado: distancia en mm en offset +10 desde RESULT_RANGE_STATUS
const RESULT_RANGE_MM: u8 = RESULT_RANGE_STATUS + 10; // 0x1E

// ─── Ajustes de sintonización (Pololu DefaultTuningSettings) ─────────────────
// Cada par es (register, value). Se aplican durante la inicialización.
#[rustfmt::skip]
const TUNING: &[(u8, u8)] = &[
    (0xFF, 0x01), (0x00, 0x00),
    (0xFF, 0x00), (0x09, 0x00), (0x10, 0x00), (0x11, 0x00),
    (0x24, 0x01), (0x25, 0xFF), (0x75, 0x00),
    (0xFF, 0x01), (0x4E, 0x2C), (0x48, 0x00), (0x30, 0x20),
    (0xFF, 0x00), (0x30, 0x09), (0x54, 0x00), (0x31, 0x04),
    (0x32, 0x03), (0x40, 0x83), (0x46, 0x25), (0x60, 0x00),
    (0x27, 0x00), (0x50, 0x06), (0x51, 0x00), (0x52, 0x96),
    (0x56, 0x08), (0x57, 0x30), (0x61, 0xDD), (0x62, 0x00),
    (0x64, 0x00), (0x65, 0x00), (0x66, 0xA0),
    (0xFF, 0x01), (0x22, 0x32), (0x47, 0x14), (0x49, 0xFF), (0x4A, 0x00),
    (0xFF, 0x00), (0x7A, 0x0A), (0x7B, 0x00), (0x78, 0x21),
    (0xFF, 0x01), (0x23, 0x34), (0x42, 0x00), (0x44, 0xFF),
    (0x45, 0x26), (0x46, 0x05), (0x40, 0x40), (0x0E, 0x06),
    (0x20, 0x1A), (0x43, 0x40),
    (0xFF, 0x00), (0x34, 0x03), (0x35, 0x44),
    (0xFF, 0x01), (0x31, 0x04), (0x4B, 0x09), (0x4C, 0x05), (0x4D, 0x04),
    (0xFF, 0x00), (0x44, 0x00), (0x45, 0x20), (0x47, 0x08),
    (0x48, 0x28), (0x67, 0x00), (0x70, 0x04), (0x71, 0x01),
    (0x72, 0xFE), (0x76, 0x00), (0x77, 0x00),
    (0xFF, 0x01), (0x0D, 0x01),
    (0xFF, 0x00), (0x80, 0x01), (0x01, 0xFF),
    (0xFF, 0x01), (0x8E, 0x01), (0x00, 0x01),
    (0xFF, 0x00), (0x80, 0x00),
];

// ─── Driver ──────────────────────────────────────────────────────────────────

/// Driver para el sensor de distancia VL53L0X por I2C software.
pub struct VL53L0X {
    i2c: SoftI2C,
    /// Variable interna requerida por la secuencia de inicio/parada del sensor.
    stop_var: u8,
    /// `true` si `init()` completó correctamente.
    pub ready: bool,
}

impl VL53L0X {
    /// Crea una instancia. Llamar a `init()` antes de usar.
    pub fn new() -> Self {
        VL53L0X { i2c: SoftI2C::new(), stop_var: 0, ready: false }
    }

    // ── Acceso a registros ────────────────────────────────────────────────────

    fn wr(&self, reg: u8, val: u8) {
        self.i2c.write(ADDR, reg, &[val]);
    }

    fn rd(&self, reg: u8) -> u8 {
        let mut buf = [0u8; 1];
        self.i2c.read(ADDR, reg, &mut buf);
        buf[0]
    }

    fn wr16(&self, reg: u8, val: u16) {
        self.i2c.write(ADDR, reg, &[(val >> 8) as u8, val as u8]);
    }

    fn rd16(&self, reg: u8) -> u16 {
        let mut buf = [0u8; 2];
        self.i2c.read(ADDR, reg, &mut buf);
        ((buf[0] as u16) << 8) | buf[1] as u16
    }

    fn rd_multi(&self, reg: u8, buf: &mut [u8]) {
        self.i2c.read(ADDR, reg, buf);
    }

    fn wr_multi(&self, reg: u8, data: &[u8]) {
        self.i2c.write(ADDR, reg, data);
    }

    // ── Inicialización ────────────────────────────────────────────────────────

    /// Lee información de SPAD (Single Photon Avalanche Diode) desde NVM.
    /// Retorna (count, is_aperture).
    fn get_spad_info(&self) -> (u8, bool) {
        self.wr(0x80, 0x01); self.wr(0xFF, 0x01); self.wr(0x00, 0x00);
        self.wr(0xFF, 0x06);
        self.wr(0x83, self.rd(0x83) | 0x04);
        self.wr(0xFF, 0x07); self.wr(0x81, 0x01);
        self.wr(0x80, 0x01); self.wr(0x94, 0x6B); self.wr(0x83, 0x00);

        // Esperar hasta que 0x83 != 0x00 (con timeout de iteraciones)
        let mut timeout = 0u16;
        while self.rd(0x83) == 0x00 {
            timeout = timeout.wrapping_add(1);
            if timeout > 5000 { break; }
        }

        self.wr(0x83, 0x01);
        let tmp = self.rd(0x92);
        let count = tmp & 0x7F;
        let is_aperture = (tmp >> 7) & 0x01 == 1;

        self.wr(0x81, 0x00); self.wr(0xFF, 0x06);
        self.wr(0x83, self.rd(0x83) & !0x04);
        self.wr(0xFF, 0x01); self.wr(0x00, 0x01);
        self.wr(0xFF, 0x00); self.wr(0x80, 0x00);

        (count, is_aperture)
    }

    /// Carga los ajustes de sintonización de ST/Pololu en el sensor.
    fn load_tuning(&self) {
        for &(reg, val) in TUNING {
            self.wr(reg, val);
        }
    }

    /// Realiza una calibración de referencia (VHV o Phase).
    fn single_ref_cal(&self, vhv_init: u8) -> bool {
        self.wr(SYSRANGE_START, 0x01 | vhv_init);
        let mut timeout = 0u16;
        while self.rd(RESULT_INTERRUPT_STATUS) & 0x07 == 0 {
            timeout = timeout.wrapping_add(1);
            if timeout > 5000 { return false; }
        }
        self.wr(SYSTEM_INTERRUPT_CLEAR, 0x01);
        self.wr(SYSRANGE_START, 0x00);
        true
    }

    /// Inicializa el sensor. Retorna `true` si la inicialización fue exitosa.
    ///
    /// Secuencia derivada de la librería Pololu VL53L0X (MIT License).
    /// Incluye: modo 2V8, configuración SPAD, tuning settings y calibración.
    pub fn init(&mut self) -> bool {
        // 1. Verificar ID del dispositivo
        if self.rd(0xC0) != 0xEE { return false; }

        // 2. Activar modo 2V8 (supply voltage 2.8V interno)
        self.wr(VHV_CONFIG_PAD_SCL_SDA_EXTSUP_HV,
                self.rd(VHV_CONFIG_PAD_SCL_SDA_EXTSUP_HV) | 0x01);

        // 3. Configurar I2C en modo estándar y obtener stop_var
        self.wr(0x88, 0x00);
        self.wr(0x80, 0x01); self.wr(0xFF, 0x01); self.wr(0x00, 0x00);
        self.stop_var = self.rd(0x91);
        self.wr(0x00, 0x01); self.wr(0xFF, 0x00); self.wr(0x80, 0x00);

        // 4. Deshabilitar SIGNAL_RATE_MSRC y SIGNAL_RATE_PRE_RANGE
        self.wr(MSRC_CONFIG_CONTROL, self.rd(MSRC_CONFIG_CONTROL) | 0x12);

        // 5. Límite de tasa de señal: 0.25 MCPS (Q9.7 = 32)
        self.wr16(FINAL_RANGE_CONFIG_MIN_COUNT_RATE_RTN, 32);

        // 6. Preparar configuración SPAD
        self.wr(0xFF, 0x01);
        self.wr(DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD, 0x00);
        self.wr(0xFF, 0x00);
        self.wr(GLOBAL_CONFIG_REF_EN_START_SELECT, 0xB4);

        let (spad_count, is_aperture) = self.get_spad_info();

        // Configurar mapa SPAD: leer 6 bytes y habilitar los primeros spad_count
        let mut spad_map = [0u8; 6];
        self.rd_multi(GLOBAL_CONFIG_SPAD_ENABLES_REF_0, &mut spad_map);

        self.wr(0xFF, 0x01);
        self.wr(DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD, 0x00);
        self.wr(0xFF, 0x00);
        self.wr(GLOBAL_CONFIG_REF_EN_START_SELECT, 0xB4);
        self.wr(DYNAMIC_SPAD_NUM_REQUESTED_REF_SPAD, spad_count);

        // Los SPADs de apertura empiezan en bit 12 (byte 1, bit 4)
        let first_spad_index: u8 = if is_aperture { 12 } else { 0 };
        let mut spads_enabled: u8 = 0;

        for i in 0..48u8 {
            if i < first_spad_index || spads_enabled >= spad_count {
                spad_map[(i / 8) as usize] &= !(1 << (i % 8));
            } else if (spad_map[(i / 8) as usize] >> (i % 8)) & 1 == 1 {
                spads_enabled += 1;
            }
        }
        self.wr_multi(GLOBAL_CONFIG_SPAD_ENABLES_REF_0, &spad_map);

        // 7. Cargar tuning settings de ST
        self.load_tuning();

        // 8. Configurar GPIO: interrupción cuando nueva muestra lista
        self.wr(SYSTEM_INTERRUPT_CONFIG_GPIO, 0x04);
        self.wr(GPIO_HV_MUX_ACTIVE_HIGH,
                self.rd(GPIO_HV_MUX_ACTIVE_HIGH) & !0x10);
        self.wr(SYSTEM_INTERRUPT_CLEAR, 0x01);

        // 9. Secuencia de calibración
        self.wr(SYSTEM_SEQUENCE_CONFIG, 0x01);
        if !self.single_ref_cal(0x40) { return false; } // VHV calibration

        self.wr(SYSTEM_SEQUENCE_CONFIG, 0x02);
        if !self.single_ref_cal(0x00) { return false; } // Phase calibration

        // 10. Restaurar secuencia completa
        self.wr(SYSTEM_SEQUENCE_CONFIG, 0xE8);

        self.ready = true;
        true
    }

    // ── Medición ──────────────────────────────────────────────────────────────

    /// Inicia el modo de medición continua (back-to-back).
    ///
    /// Llamar una vez tras `init()`. El sensor mide continuamente y almacena
    /// el último resultado, que se lee con `read_mm()`.
    pub fn start_continuous(&mut self) {
        self.wr(0x80, 0x01); self.wr(0xFF, 0x01); self.wr(0x00, 0x00);
        self.wr(0x91, self.stop_var);
        self.wr(0x00, 0x01); self.wr(0xFF, 0x00); self.wr(0x80, 0x00);
        self.wr(SYSRANGE_START, 0x02); // back-to-back continuous
    }

    /// Lee la distancia en mm si hay una medición disponible.
    ///
    /// Retorna `None` si el sensor aún no tiene una nueva muestra lista.
    /// No bloquea — llamar periódicamente en el loop principal.
    ///
    /// Valores típicos de error retornados por el sensor:
    /// - `8190` / `8191` = sin objetivo detectado (fuera de rango)
    pub fn read_mm(&self) -> Option<u16> {
        if self.rd(RESULT_INTERRUPT_STATUS) & 0x07 == 0 {
            return None; // nueva muestra aún no disponible
        }
        let mm = self.rd16(RESULT_RANGE_MM);
        self.wr(SYSTEM_INTERRUPT_CLEAR, 0x01);
        // Filtrar lecturas de error (out of range)
        if mm >= 8190 { None } else { Some(mm) }
    }
}
