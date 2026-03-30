// Version: v1.1
//! # Driver para el sensor ultrasónico HC-SR04.
//!
//! Este driver permite medir distancias utilizando el sensor ultrasónico HC-SR04.
//! El funcionamiento se basa en enviar un pulso sónico y medir el tiempo que tarda
//! el eco en regresar al sensor.
//!
//! ## Control de latencia — `echo_timeout_us`
//!
//! La espera de eco es bloqueante (busy-wait con `delay_us(1)`). Sin límite, un
//! obstáculo a 4 m provoca ~23 ms de bloqueo, excediendo el ciclo de 20 ms del
//! loop principal.
//!
//! Usar `with_timeout(µs)` para acotar el bloqueo según el rango máximo de interés:
//!
//! | Distancia máx | Tiempo eco aprox | Timeout sugerido |
//! |---------------|-----------------|------------------|
//! | 200 mm (emergencia) | ~1 166 µs   | 1 750 µs         |
//! | 500 mm        | ~2 916 µs       | 3 200 µs         |
//! | 4 000 mm (full) | ~23 326 µs    | 30 000 µs (defecto) |
//!
//! Fórmula: `timeout_us = distance_mm × 10 000 / 1 715`.

use arduino_hal::port::Pin;
use arduino_hal::port::mode::{Input, Output, AnyInput};
use crate::sensors::ProximitySensor;

/// Estructura para el sensor ultrasónico HC-SR04.
///
/// Posee un pin de Trigger (disparador) y un pin de Echo (receptor).
pub struct HCSR04<TPIN, EPIN> {
    /// Pin de salida para iniciar la ráfaga ultrasónica.
    trigger: Pin<Output, TPIN>,
    /// Pin de entrada para medir la duración del pulso de retorno.
    echo: Pin<Input<AnyInput>, EPIN>,
    /// Última distancia válida medida (mm).
    last_valid: Option<u16>,
    /// Contador de errores consecutivos para invalidar la lectura.
    consecutive_errors: u8,
    /// Tiempo máximo de espera del eco en µs.
    /// Limita el bloqueo del loop principal. Ver `with_timeout()`.
    echo_timeout_us: u32,
}

impl<TPIN, EPIN> HCSR04<TPIN, EPIN>
where
    TPIN: arduino_hal::port::PinOps,
    EPIN: arduino_hal::port::PinOps,
{
    /// Crea una nueva instancia con timeout de eco completo (30 000 µs ≈ 4 m).
    ///
    /// Para limitar el bloqueo del loop principal usar `.with_timeout(µs)`.
    pub fn new(trigger: Pin<Output, TPIN>, echo: Pin<Input<AnyInput>, EPIN>) -> Self {
        Self {
            trigger,
            echo,
            last_valid: None,
            consecutive_errors: 0,
            echo_timeout_us: 30_000,
        }
    }

    /// Restringe el tiempo máximo de espera del eco (builder).
    ///
    /// Solo detecta objetos dentro del rango `timeout_us × 1715 / 10 000` mm.
    /// Los objetos más lejanos provocan `handle_error()` (retorna `last_valid`
    /// hasta 5 errores consecutivos, luego `None`).
    ///
    /// # Ejemplo
    /// ```ignore
    /// // Detectar solo emergencias < 300 mm, bloqueo máx ~1.75 ms en vez de ~30 ms
    /// let hcsr04 = HCSR04::new(trig, echo).with_timeout(1_750);
    /// ```
    pub fn with_timeout(mut self, echo_timeout_us: u32) -> Self {
        self.echo_timeout_us = echo_timeout_us;
        self
    }

    /// Realiza una medición de distancia enviando un pulso.
    /// 
    /// Retorna la distancia calculada en milímetros (mm).
    /// Si hay un fallo puntual, retorna la última lectura válida.
    /// Retorna `None` si hay demasiados errores consecutivos o rango inválido.
    pub fn measure_mm(&mut self) -> Option<u16> {
        // Aseguramos que el trigger esté en BAJO antes de iniciar el ciclo.
        self.trigger.set_low();
        arduino_hal::delay_us(10);

        // Enviamos el pulso de disparo (mínimo 10 microsegundos).
        self.trigger.set_high();
        arduino_hal::delay_us(20);
        self.trigger.set_low();

        // Esperamos a que el pin Echo suba a ALTO (inicio del retorno).
        let mut count = 0;
        while self.echo.is_low() {
            count += 1;
            if count > 30000 { 
                return self.handle_error();
            } 
        }

        // Medimos cuánto tiempo permanece el pin Echo en ALTO.
        // El timeout limita el bloqueo al rango máximo de interés (ver with_timeout).
        let mut duration_us: u32 = 0;
        while self.echo.is_high() {
            duration_us += 1;
            arduino_hal::delay_us(1);
            if duration_us > self.echo_timeout_us {
                return self.handle_error();
            }
        }

        // Cálculo de distancia: (Tiempo * Velocidad del Sonido) / 2
        let distance = (duration_us * 1715) / 10000;
        
        // El rango práctico del HC-SR04 es de 2cm a 400cm.
        if distance > 4000 || distance < 2 {
            self.handle_error()
        } else {
            self.consecutive_errors = 0;
            let val = distance as u16;
            self.last_valid = Some(val);
            Some(val)
        }
    }

    /// Gestiona un fallo de lectura devolviendo la última válida si es posible.
    fn handle_error(&mut self) -> Option<u16> {
        self.consecutive_errors += 1;
        // Si hay más de 5 errores seguidos, invalidamos todo.
        if self.consecutive_errors > 5 {
            self.last_valid = None;
            None
        } else {
            // Retornamos el último valor bueno para dar estabilidad.
            self.last_valid
        }
    }
}

impl<TPIN, EPIN> ProximitySensor for HCSR04<TPIN, EPIN>
where
    TPIN: arduino_hal::port::PinOps,
    EPIN: arduino_hal::port::PinOps,
{
    /// Implementación de la interfaz común para obtener la distancia.
    fn get_distance_mm(&mut self) -> Option<u16> {
        self.measure_mm()
    }
}
