// Version: v1.0
//! # Type-Erased Motor (`ErasedMotor`)
//!
//! Provides a fixed-size, heap-free wrapper around any type that implements
//! the `Motor` trait, enabling motors on different AVR timers/pins to be
//! stored in a homogeneous array.
//!
//! See `docs/consideration_implementation.md` for the full design rationale.

use crate::motor_control::Motor;

/// Monomorphized shim for `Motor::set_speed`. Cast to an erased fn pointer.
unsafe fn set_speed_impl<M: Motor>(ptr: *mut (), speed: i16) {
    (*(ptr as *mut M)).set_speed(speed);
}

/// Monomorphized shim for `Motor::stop`. Cast to an erased fn pointer.
unsafe fn stop_impl<M: Motor>(ptr: *mut ()) {
    (*(ptr as *mut M)).stop();
}

/// A type-erased motor with a fixed memory footprint.
///
/// Internally stores a raw pointer to the concrete motor and two function
/// pointers — one per `Motor` method — that are monomorphized to the
/// correct concrete type at the `ErasedMotor::new` call site.
///
/// This is structurally equivalent to a manual vtable, without heap allocation.
///
/// # Safety Invariant
/// The concrete motor pointed to by `data` must outlive this `ErasedMotor`.
/// In AVR firmware (`fn main() -> !`), stack-allocated motors never go out
/// of scope, satisfying this invariant automatically.
pub struct ErasedMotor {
    data: *mut (),
    set_speed_fn: unsafe fn(*mut (), i16),
    stop_fn: unsafe fn(*mut ()),
}

impl ErasedMotor {
    /// Erases the concrete type of a motor, producing an `ErasedMotor`.
    ///
    /// # Safety
    /// `motor` must outlive the returned `ErasedMotor`. The caller is
    /// responsible for ensuring no other mutable access to `motor` occurs
    /// while this `ErasedMotor` is alive.
    pub unsafe fn new<M: Motor>(motor: &mut M) -> Self {
        Self {
            data: motor as *mut M as *mut (),
            set_speed_fn: set_speed_impl::<M>,
            stop_fn: stop_impl::<M>,
        }
    }
}

impl Motor for ErasedMotor {
    fn set_speed(&mut self, speed: i16) {
        unsafe { (self.set_speed_fn)(self.data, speed) }
    }

    fn stop(&mut self) {
        unsafe { (self.stop_fn)(self.data) }
    }
}
