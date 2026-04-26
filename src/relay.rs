// Version: v1.0
//! # Módulo de Control de Relays — Gestión de Bancos de Batería
//!
//! Controla el módulo de 2 relays SRD-05VDC-SL-C para gestionar la potencia
//! entregada a los puentes H (L298N/BTS7960) desde dos bancos de batería
//! independientes.
//!
//! ## Arquitectura de potencia
//!
//! ```text
//! Bank 2 (+) → COM1 → NO1 → Buck 2 → ─┐
//!                                       ├→ L298N VCC (todos los motores)
//! Bank 3 (+) → COM2 → NO2 → Buck 3 → ─┘
//! ```
//!
//! - **IN1 (D40) → Relay 1 → Bank 2**: banco primario (operación normal)
//! - **IN2 (D41) → Relay 2 → Bank 3**: banco de respaldo (failover / paralelo)
//!
//! ## Lógica de trigger: ACTIVE LOW (módulo SRD-05VDC-SL-C estándar)
//!
//! | IN1 | IN2 | Resultado                              |
//! |-----|-----|----------------------------------------|
//! | LOW | HIGH | Bank 2 activo — operación normal      |
//! | HIGH | LOW | Bank 3 activo — failover manual       |
//! | LOW  | LOW | Ambos activos — máxima corriente      |
//! | HIGH | HIGH | Ambos cortados — emergencia / apagado |
//!
//! ## Fail-safe
//!
//! En `FAULT` o `Safe Mode`, ambos pines se ponen HIGH → motores sin potencia.
//! Si el Mega se resetea (PORF/EXTRF/BORF/WDRF), los pines de salida del
//! ATmega2560 arrancan en estado de entrada (alta impedancia) → el módulo
//! relay interpreta eso como HIGH por su pull-down interno → relay abierto →
//! motores cortados. Sin potencia entregada hasta que el firmware configure
//! los pines explícitamente.

use arduino_hal::port::{mode::Output, Pin, PinOps};
use crate::state_machine::BankMode;

/// Controlador del módulo relay de 2 canales SRD-05VDC-SL-C.
///
/// Genérico sobre los tipos de pin para ser compatible con cualquier par de
/// pines GPIO del ATmega2560.
pub struct RelayModule<P1: PinOps, P2: PinOps> {
    in1: Pin<Output, P1>,   // D40 — Relay 1 → Bank 2
    in2: Pin<Output, P2>,   // D41 — Relay 2 → Bank 3
    mode: BankMode,
}

impl<P1: PinOps, P2: PinOps> RelayModule<P1, P2> {
    /// Inicializa el módulo con Bank 2 activo (estado normal de operación).
    ///
    /// Aplica el estado en los pines inmediatamente en la construcción para
    /// evitar cualquier período transitorio con ambos bancos deshabilitados.
    pub fn new(mut in1: Pin<Output, P1>, mut in2: Pin<Output, P2>) -> Self {
        in1.set_low();   // Bank 2 ON
        in2.set_high();  // Bank 3 OFF
        Self { in1, in2, mode: BankMode::Bank2Only }
    }

    /// Retorna el modo de banco actualmente aplicado.
    #[inline(always)]
    pub fn mode(&self) -> BankMode { self.mode }

    /// Aplica un nuevo modo de banco y actualiza los pines de control.
    pub fn set_mode(&mut self, mode: BankMode) {
        self.mode = mode;
        match mode {
            BankMode::Bank2Only => { self.in1.set_low();  self.in2.set_high(); }
            BankMode::Bank3Only => { self.in1.set_high(); self.in2.set_low();  }
            BankMode::BothBanks => { self.in1.set_low();  self.in2.set_low();  }
            BankMode::AllOff    => { self.in1.set_high(); self.in2.set_high(); }
        }
    }

    /// Corte de emergencia: ambos bancos OFF.
    ///
    /// Llamar en cualquier transición a `RoverState::Fault` o `RoverState::Safe`.
    /// Es idempotente — llamarlo múltiples veces no tiene efecto adicional.
    #[inline(always)]
    pub fn emergency_off(&mut self) {
        self.set_mode(BankMode::AllOff);
    }

    /// Restaura el estado normal: Bank 2 activo, Bank 3 en espera.
    ///
    /// Llamar en `Command::Reset` para restaurar la potencia a los motores
    /// tras salir de FAULT o Safe Mode.
    #[inline(always)]
    pub fn reset_normal(&mut self) {
        self.set_mode(BankMode::Bank2Only);
    }
}
