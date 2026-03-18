// Version: v1.1
//! # Controlador Central del Rover (Refactorizado con AnyPin)
//!
//! Este módulo implementa la lógica de control para el chasis de 6 ruedas
//! utilizando Type Erasure (AnyPin) para permitir que motores en diferentes pines
//! convivan en el mismo array.

use crate::motor_control::Motor;
use crate::sensors::encoder::Encoder;

/// Estructura que representa un canal de tracción (Motor + Encoder).
/// Usamos un rasgo dinámico (Box no disponible, así que usamos tipos genéricos
/// en la estructura pero permitiremos que se guarden en un array mediante AnyPin
/// en el nivel del Motor).
pub struct DriveChannel<M: Motor, E: Encoder> {
    pub motor: M,
    pub encoder: E,
    last_count: i32,
    stall_timer: u16,
}

impl<M: Motor, E: Encoder> DriveChannel<M, E> {
    pub fn new(motor: M, encoder: E) -> Self {
        Self {
            motor,
            encoder,
            last_count: 0,
            stall_timer: 0,
        }
    }

    pub fn check_stall(&mut self, speed: i16) -> bool {
        let current_count = self.encoder.get_counts();
        let speed_abs = speed.abs();

        if speed_abs > 20 && current_count == self.last_count {
            self.stall_timer = self.stall_timer.saturating_add(1);
        } else {
            self.stall_timer = 0;
        }

        self.last_count = current_count;
        self.stall_timer > 50
    }
}

/// Controlador principal para un chasis de 6 ruedas.
pub struct RoverController<M: Motor, E: Encoder> {
    pub channels: [DriveChannel<M, E>; 6],
    pub is_stalled: [bool; 6],
    pub emergency_stop: bool,
}

impl<M: Motor, E: Encoder> RoverController<M, E> {
    pub fn new(channels: [DriveChannel<M, E>; 6]) -> Self {
        Self {
            channels,
            is_stalled: [false; 6],
            emergency_stop: false,
        }
    }

    pub fn tank_drive(&mut self, left_speed: i16, right_speed: i16) {
        if self.emergency_stop { return; }
        for i in 0..3 { self.channels[i].motor.set_speed(left_speed); }
        for i in 3..6 { self.channels[i].motor.set_speed(right_speed); }
    }

    pub fn update(&mut self, current_speeds: [i16; 6]) {
        if self.emergency_stop { return; }

        let mut left_stalls: u8 = 0;
        let mut right_stalls: u8 = 0;

        for i in 0..6 {
            self.is_stalled[i] = self.channels[i].check_stall(current_speeds[i]);
            if self.is_stalled[i] {
                self.channels[i].motor.stop();
                if i < 3 { left_stalls += 1; } 
                else { right_stalls += 1; }
            }
        }

        if left_stalls >= 2 || right_stalls >= 2 {
            self.stop_all();
            self.emergency_stop = true;
        }
    }

    pub fn stop_all(&mut self) {
        for channel in &mut self.channels {
            channel.motor.stop();
        }
    }

    pub fn reset_emergency(&mut self) {
        self.emergency_stop = false;
        for i in 0..6 { self.is_stalled[i] = false; }
    }
}
