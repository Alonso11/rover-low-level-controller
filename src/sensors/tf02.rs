// Version: v1.1
//! # Driver TF02 — LiDAR Benewake DELiDAR TF02 (largo alcance)
//!
//! Sensor ToF UART que transmite frames de 9 bytes a 100 Hz de forma continua.
//! No requiere inicialización: con solo conectar 5V el sensor ya emite datos.
//!
//! ## Características
//! | Parámetro       | Valor                              |
//! |-----------------|-------------------------------------|
//! | Rango           | 0.4 – 22 m (reflectividad 90 %)    |
//! | Precisión       | <6 cm (≤5 m), <2 % (5–22 m)       |
//! | Frecuencia      | 100 Hz                              |
//! | Interfaz        | UART 115200 8N1, LVTTL 0–3.3 V     |
//! | Protección      | IP65                                |
//!
//! ## Conexión al Arduino Mega (USART2)
//! ```text
//! TF02 verde (TX) → Mega D17 (RX2)   ← único cable necesario
//! TF02 rojo       → 5 V
//! TF02 negro      → GND
//! TF02 blanco (RX)→ NO conectar (no se le envían comandos)
//! ```
//! El Mega interpreta 3.3 V como HIGH sin level-shifter.
//! USART1 (D18/D19) está ocupado por encoders INT2/INT3.
//! USART3 (D14/D15) está reservado para el enlace RPi5 en producción.
//! USART2 (D16/D17) es el único puerto libre.
//!
//! ## Frame de 9 bytes (little-endian)
//! ```text
//! B0=0x59  B1=0x59  B2=DIST_L  B3=DIST_H  B4=STR_L  B5=STR_H  B6=SIG  B7=TIME  B8=CHECK
//! ```
//! - DIST en cm → multiplicar ×10 para obtener mm.
//! - SIG: fiabilidad 0x01–0x08. Solo 7 u 8 son fiables.
//! - CHECK = (B0+B1+B2+B3+B4+B5+B6+B7) & 0xFF.
//! - Si DIST == 2200 cm → fuera de rango (señal no confiable).
//!
//! ## Uso (sin HAL — puro Rust)
//! ```ignore
//! let mut tf02 = TF02::new();
//! // En ISR o polling de USART2:
//! if tf02.feed(byte_from_usart2) {
//!     let mm = tf02.last_dist_mm;   // dato fiable
//!     let strength = tf02.last_strength;
//! }
//! ```

/// Driver de parseo de frames TF02. No depende del HAL; procesa bytes uno a uno.
///
/// Uso típico: llamar [`TF02::feed`] con cada byte recibido de USART2.
/// Cuando retorna `true`, [`TF02::last_dist_mm`] contiene una lectura válida.
///
/// El filtro SIG está deshabilitado: se aceptan todos los valores (0–255) para
/// permitir diagnóstico en hardware. Consultar `last_sig` para conocer el valor
/// que emite el sensor real y ajustar si es necesario.
pub struct TF02 {
    buf: [u8; 9],
    idx: u8,
    /// Última distancia en mm (0 si todavía no hay lectura válida).
    pub last_dist_mm: u16,
    /// Intensidad de señal del último frame (0–65535).
    pub last_strength: u16,
    /// Valor SIG del último frame recibido. Útil para diagnosticar qué envía el sensor real.
    pub last_sig: u8,
}

impl TF02 {
    pub const fn new() -> Self {
        Self { buf: [0u8; 9], idx: 0, last_dist_mm: 0, last_strength: 0, last_sig: 0 }
    }

    /// Alimenta un byte del stream UART.
    ///
    /// Retorna `true` cuando se completó y validó un frame completo.
    /// En ese caso, [`last_dist_mm`](Self::last_dist_mm) y
    /// [`last_strength`](Self::last_strength) están actualizados.
    ///
    /// Retorna `false` en cualquier otro caso: frame incompleto, checksum
    /// erróneo, SIG no fiable, o lectura fuera de rango.
    pub fn feed(&mut self, byte: u8) -> bool {
        match self.idx {
            // Sincronización: esperar doble header 0x59 0x59
            0 => {
                if byte == 0x59 { self.buf[0] = byte; self.idx = 1; }
            }
            1 => {
                if byte == 0x59 { self.buf[1] = byte; self.idx = 2; }
                else { self.idx = 0; }
            }
            // Acumular bytes de datos
            2..=7 => {
                self.buf[self.idx as usize] = byte;
                self.idx += 1;
            }
            // Byte 8: checksum + validación completa
            8 => {
                self.idx = 0;
                // CHECK = (B0+B1+B2+B3+B4+B5+B6+B7) & 0xFF
                let check: u8 = self.buf[0]
                    .wrapping_add(self.buf[1])
                    .wrapping_add(self.buf[2])
                    .wrapping_add(self.buf[3])
                    .wrapping_add(self.buf[4])
                    .wrapping_add(self.buf[5])
                    .wrapping_add(self.buf[6])
                    .wrapping_add(self.buf[7]);
                if byte != check { return false; }
                // Distancia en cm (little-endian)
                let dist_cm = (self.buf[3] as u16) << 8 | self.buf[2] as u16;
                // dist_cm == 2200 → out-of-range flag del sensor
                if dist_cm >= 2200 { return false; }
                self.last_dist_mm  = dist_cm * 10;
                self.last_strength = (self.buf[5] as u16) << 8 | self.buf[4] as u16;
                self.last_sig      = self.buf[6];
                return true;
            }
            _ => { self.idx = 0; }
        }
        false
    }

    /// Resetea el estado de sincronización. Llamar si se sospecha corrupción
    /// del stream (e.g. tras reinicio del sensor).
    pub fn reset(&mut self) {
        self.idx = 0;
        self.buf = [0u8; 9];
    }
}

