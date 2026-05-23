// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// ro_after_init.rs — rust equivalent of __ro_after_init

use core::cell::UnsafeCell;
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, Ordering};

/// wrapper for values that can only be written during kernel init.
/// after seal(), no write path exists.
///
/// ```
/// use rustlux_mm::ro_after_init::RoAfterInit;
///
/// static SUPPORTED_PTE_MASK: RoAfterInit<u64> = RoAfterInit::uninit();
///
/// unsafe { SUPPORTED_PTE_MASK.init(!0u64); }
/// SUPPORTED_PTE_MASK.seal();
/// let mask = SUPPORTED_PTE_MASK.read();
/// assert_eq!(*mask, !0u64);
/// ```
pub struct RoAfterInit<T> {
    value:  UnsafeCell<MaybeUninit<T>>,
    sealed: AtomicBool,
}

unsafe impl<T: Send> Sync for RoAfterInit<T> {}
unsafe impl<T: Send> Send for RoAfterInit<T> {}

impl<T> RoAfterInit<T> {
    pub const fn uninit() -> Self {
        Self {
            value:  UnsafeCell::new(MaybeUninit::uninit()),
            sealed: AtomicBool::new(false),
        }
    }

    pub const fn new(value: T) -> Self {
        Self {
            value:  UnsafeCell::new(MaybeUninit::new(value)),
            sealed: AtomicBool::new(false),
        }
    }

    /// # Safety
    /// must be called before seal(), from single-cpu init context.
    pub unsafe fn init(&self, value: T) {
        debug_assert!(!self.sealed.load(Ordering::Acquire));
        unsafe { (*self.value.get()).write(value); }
    }

    #[inline]
    pub fn seal(&self) {
        self.sealed.store(true, Ordering::Release);
    }

    #[inline]
    pub fn read(&self) -> &T {
        debug_assert!(self.sealed.load(Ordering::Acquire));
        unsafe { (*self.value.get()).assume_init_ref() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_init_and_read() {
        let x: RoAfterInit<u64> = RoAfterInit::uninit();
        unsafe { x.init(42u64); }
        x.seal();
        assert_eq!(*x.read(), 42u64);
    }

    #[test]
    fn const_new() {
        static MASK: RoAfterInit<u64> = RoAfterInit::new(!0u64);
        MASK.seal();
        assert_eq!(*MASK.read(), !0u64);
    }
}
