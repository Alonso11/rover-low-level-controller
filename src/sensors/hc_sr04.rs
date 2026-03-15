// Version: v1.0
//! # Driver para el sensor ultrasónico HC-SR04.
//!
//! Este driver permite medir distancias utilizando el sensor ultrasónico HC-SR04.
//! El funcionamiento se basa en enviar un pulso sónico y medir el tiempo que tarda
//! el eco en regresar al sensor.

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
}

impl<TPIN, EPIN> HCSR04<TPIN, EPIN>
where
    TPIN: arduino_hal::port::PinOps,
    EPIN: arduino_hal::port::PinOps,
{
    /// Crea una nueva instancia del sensor HC-SR04.
    ///
    /// # Parámetros
    /// * `trigger`: Pin configurado como salida (Output).
    /// * `echo`: Pin configurado como entrada genérica (Input<AnyInput>).
    pub fn new(trigger: Pin<Output, TPIN>, echo: Pin<Input<AnyInput>, EPIN>) -> Self {
        Self { trigger, echo }
    }

    /// Realiza una medición de distancia enviando un pulso.
    /// 
    /// Retorna la distancia calculada en milímetros (mm).
    /// Retorna `None` si la lectura excede el tiempo de espera (timeout)
    /// o si la distancia está fuera del rango operativo del sensor (aprox. 4m).
    pub fn measure_mm(&mut self) -> Option<u16> {
        // Aseguramos que el trigger esté en BAJO antes de iniciar el ciclo.
        self.trigger.set_low();
        arduino_hal::delay_us(2);

        // Enviamos el pulso de disparo (mínimo 10 microsegundos).
        self.trigger.set_high();
        arduino_hal::delay_us(10);
        self.trigger.set_low();

        // Esperamos a que el pin Echo suba a ALTO (inicio del retorno).
        // Se utiliza un contador simple para implementar un timeout.
        let mut count = 0;
        while self.echo.is_low() {
            count += 1;
            if count > 20000 { return None; } 
        }

        // Medimos cuánto tiempo permanece el pin Echo en ALTO.
        // La duración es proporcional a la distancia recorrida por el sonido.
        let mut duration_us: u32 = 0;
        while self.echo.is_high() {
            duration_us += 1;
            arduino_hal::delay_us(1);
            // Timeout preventivo si no hay objeto cercano o error (max ~5.1 metros).
            if duration_us > 30000 { return None; }
        }

        // Cálculo de distancia: (Tiempo * Velocidad del Sonido) / 2
        // Velocidad del sonido aprox. 0.343 mm/µs.
        // Formula: (duration_us * 1715) / 10000 es equivalente a (dur * 0.1715).
        let distance = (duration_us * 1715) / 10000;
        
        // El rango práctico del HC-SR04 es de 2cm a 400cm.
        if distance > 4000 || distance < 2 {
            None
        } else {
            Some(distance as u16)
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
