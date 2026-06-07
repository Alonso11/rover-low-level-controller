// Version: v2.0 — Logger DEDICADO de descarga de batería (INA3221, voltaje 2 bancos)
//! Arduino Mega 2560 standalone, SOLO para registrar la curva de descarga de los
//! DOS bancos del rover vía INA3221 (modo bus-only = voltaje, NO corriente).
//! Pensado para correr en un segundo Mega desatendido durante una prueba de
//! autonomía, mientras un PC captura el serial a archivo.
//!
//! ## Arquitectura de potencia del rover (2 bancos 4S 18650, BMS)
//! - **Banco 1 (LÓGICA)** → 3 step-down a 5 V (Arduino/RPi/sensores). Consumo bajo y constante.
//! - **Banco 2 (MOTORES)** → 6 step-down a 12 V (uno por motor). Consumo alto e intermitente
//!   → es el que LIMITA la autonomía y se descarga primero.
//!
//! ## Umbrales 4S 18650 (informativos, se anotan en el log como `# WARN`)
//! | Nivel | mV | Significado |
//! |-------|------|-------------|
//! | Carga máx | 16800 | 4.2 V/celda (no exceder al cargar) |
//! | Dropout buck 12 V | ~13500 | los XL4015 dejan de sostener 12 V → fin de autonomía ÚTIL (dep. del convertidor; verificar) |
//! | Corte celda | 12000 | 3.0 V/celda → detener la prueba para no dañar las celdas |
//!
//! ## Robustez para corridas largas
//! - `init()` se reintenta cada 1 s hasta que el chip responde (die ID 0x3220).
//! - Cada `HEALTH_EVERY` muestras revalida el die ID; si el bus se cayó, intenta
//!   re-init sin abortar el stream (`# WARN`).
//!
//! ## Conexión (= driver INA3221)
//! VS=5V, GND común con los 2 bancos, SDA=D42(PL7), SCL=D43(PL6), A0=GND (0x40),
//! pull-ups del bus a 3.3 V. Topología voltaje-solo: CH IN+/IN- al (+) del banco.
//!   - CH1 IN+/IN- → (+) Banco 1 (lógica/5V)
//!   - CH2 IN+/IN- → (+) Banco 2 (motores/12V)
//!
//! CSV: t_s,b1_mv,b2_mv     (t_s = segundos desde el arranque; b1=lógica, b2=motores)
//!
//! Captura en PC (sin timeout, corrida larga):
//!   stty -F /dev/ttyACMx 115200 raw -echo
//!   cat /dev/ttyACMx | tee logs/descarga_baterias.csv
//!
//! Flash: `make flash-ina-logger PORT=/dev/ttyACMx`

#![no_std]
#![no_main]

use panic_halt as _;
use rover_low_level_controller::sensors::INA3221;

/// Intervalo entre muestras (ms). 2 s → miles de puntos en una descarga de horas.
const SAMPLE_MS: u32 = 2000;
/// Cada cuántas muestras revalidar la comunicación I2C (salud del bus).
const HEALTH_EVERY: u32 = 30; // ~cada 60 s con SAMPLE_MS=2000

/// Umbral de dropout del buck de 12 V (motores). Dependiente del convertidor; ajustar.
const DROPOUT_MV: u16 = 13500;
/// Corte de protección de celda (3.0 V/celda × 4S).
const CUTOFF_MV: u16 = 12000;

#[arduino_hal::entry]
fn main() -> ! {
    let dp   = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);
    let mut serial = arduino_hal::default_serial!(dp, pins, 115200);

    let _ = ufmt::uwriteln!(&mut serial, "# ina3221_battery_logger v2.0 - curva de descarga (voltaje 2 bancos)");
    let _ = ufmt::uwriteln!(&mut serial, "# b1=LOGICA(5V)  b2=MOTORES(12V, limita autonomia)");
    let _ = ufmt::uwriteln!(&mut serial, "# dropout_buck12v={}mV  corte_celda={}mV", DROPOUT_MV, CUTOFF_MV);
    let _ = ufmt::uwriteln!(&mut serial, "# csv_cols=t_s,b1_mv,b2_mv");

    let mut ina = INA3221::new();

    // Reintentar init hasta que el chip responda (die ID 0x3220).
    let mut tries: u32 = 0;
    while !ina.init() {
        tries += 1;
        let _ = ufmt::uwriteln!(&mut serial,
            "# WARN init fallo (intento {}) die_id=0x{:X} manuf=0x{:X} - revisar SDA/SCL/VCC/pull-ups",
            tries, ina.read_die_id(), ina.read_manufacturer_id());
        arduino_hal::delay_ms(1000);
    }
    let _ = ufmt::uwriteln!(&mut serial, "# init OK die_id=0x{:X} - logueando cada {} ms", ina.read_die_id(), SAMPLE_MS);

    let mut t_s: u32 = 0;
    let mut sample: u32 = 0;
    let mut dropout_logged = false; // banderas: anotar el cruce de umbral una sola vez
    let mut cutoff_logged  = false;
    loop {
        // Chequeo de salud periódico: si el bus se cayó, re-init sin abortar.
        if sample != 0 && sample % HEALTH_EVERY == 0 && ina.read_die_id() != 0x3220 {
            let _ = ufmt::uwriteln!(&mut serial, "# WARN i2c perdido en t_s={} - reintentando init", t_s);
            let _ = ina.init();
        }

        let b1 = ina.read_bank1_mv(); // lógica (5V)
        let b2 = ina.read_bank2_mv(); // motores (12V)
        let _ = ufmt::uwriteln!(&mut serial, "{},{},{}", t_s, b1, b2);

        // Anotaciones de umbral sobre el banco de motores (no ensucian las columnas CSV).
        if !dropout_logged && b2 > 0 && b2 < DROPOUT_MV {
            let _ = ufmt::uwriteln!(&mut serial, "# WARN dropout buck 12V: b2={}mV en t_s={} (fin autonomia util)", b2, t_s);
            dropout_logged = true;
        }
        if !cutoff_logged && b2 > 0 && b2 < CUTOFF_MV {
            let _ = ufmt::uwriteln!(&mut serial, "# WARN CORTE celda: b2={}mV en t_s={} - DETENER prueba", b2, t_s);
            cutoff_logged = true;
        }

        arduino_hal::delay_ms(SAMPLE_MS);
        t_s += SAMPLE_MS / 1000;
        sample += 1;
    }
}
