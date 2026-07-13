// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// mem_sentinel.rs — DRAM instability detector (decision core)
//
// The kernel cannot make unstable DRAM stable: frequency and timings
// are programmed by firmware during memory training, before boot.
// This module implements the lock-free decision core that classifies
// the *symptom* — machine-check errors sourced from the memory
// controller — into actions:
//
//   None    → just another corrected error, keep counting
//   Warn    → corrected-error storm inside the window: the EXPO/XMP
//             profile is almost certainly unstable, tell the user
//   Offline → uncorrected error: hand the page to hwpoison
//
// Mirrored by the C fallback in mm/rustlux_memsentinel.c
// (patch 0008-rustlux-mem-sentinel.patch).

use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// sliding window in milliseconds
pub const WINDOW_MS: u64 = 60_000;
/// default corrected-errors-per-window threshold
pub const DEFAULT_THRESHOLD: u32 = 8;

/// action decided for one recorded memory error
#[repr(i32)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SentinelAction {
    /// corrected error below threshold — log only
    None = 0,
    /// corrected-error storm — warn that DRAM is unstable
    Warn = 1,
    /// uncorrected error — queue page for memory_failure()
    Offline = 2,
}

/// lock-free fixed-window error rate tracker
pub struct MemSentinel {
    window_start_ms: AtomicU64,
    window_count: AtomicU32,
}

impl MemSentinel {
    /// creates a tracker with an empty window
    pub const fn new() -> Self {
        Self {
            window_start_ms: AtomicU64::new(0),
            window_count: AtomicU32::new(0),
        }
    }

    /// records one memory error at `now_ms` and decides the action.
    /// `uncorrected` errors always escalate to Offline immediately;
    /// corrected errors escalate to Warn exactly once per window,
    /// when the count reaches `threshold`.
    pub fn on_error(&self, now_ms: u64, uncorrected: bool, threshold: u32) -> SentinelAction {
        if uncorrected {
            return SentinelAction::Offline;
        }

        let start = self.window_start_ms.load(Ordering::Relaxed);
        if now_ms.wrapping_sub(start) > WINDOW_MS {
            // new window; only one CPU wins the reset
            if self
                .window_start_ms
                .compare_exchange(start, now_ms, Ordering::Relaxed, Ordering::Relaxed)
                .is_ok()
            {
                self.window_count.store(0, Ordering::Relaxed);
            }
        }

        let count = self.window_count.fetch_add(1, Ordering::Relaxed) + 1;
        if count == threshold {
            SentinelAction::Warn
        } else {
            SentinelAction::None
        }
    }
}

static SENTINEL: MemSentinel = MemSentinel::new();

/// FFI entry point called from mm/rustlux_memsentinel.c when
/// CONFIG_RUST=y. Returns the RUSTLUX_MS_ACTION_* value.
#[no_mangle]
pub extern "C" fn rustlux_memsentinel_on_error(
    now_ms: u64,
    uncorrected: u8,
    threshold: u32,
) -> i32 {
    SENTINEL.on_error(now_ms, uncorrected != 0, threshold.max(1)) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn corrected_below_threshold_is_none() {
        let s = MemSentinel::new();
        for _ in 0..DEFAULT_THRESHOLD - 1 {
            assert_eq!(s.on_error(1_000, false, DEFAULT_THRESHOLD), SentinelAction::None);
        }
    }

    #[test]
    fn corrected_storm_warns_once_at_threshold() {
        let s = MemSentinel::new();
        for _ in 0..DEFAULT_THRESHOLD - 1 {
            s.on_error(1_000, false, DEFAULT_THRESHOLD);
        }
        // the error that reaches the threshold triggers the warning
        assert_eq!(s.on_error(1_500, false, DEFAULT_THRESHOLD), SentinelAction::Warn);
        // further errors in the same window do not spam
        assert_eq!(s.on_error(2_000, false, DEFAULT_THRESHOLD), SentinelAction::None);
    }

    #[test]
    fn window_expiry_resets_count() {
        let s = MemSentinel::new();
        for _ in 0..DEFAULT_THRESHOLD - 1 {
            s.on_error(1_000, false, DEFAULT_THRESHOLD);
        }
        // next error arrives after the window closed → count restarts
        let late = 1_000 + WINDOW_MS + 1;
        assert_eq!(s.on_error(late, false, DEFAULT_THRESHOLD), SentinelAction::None);
    }

    #[test]
    fn uncorrected_always_offlines() {
        let s = MemSentinel::new();
        assert_eq!(s.on_error(0, true, DEFAULT_THRESHOLD), SentinelAction::Offline);
    }

    #[test]
    fn expo_6000_instability_scenario() {
        // typical unstable EXPO profile: burst of corrected UMC errors
        // under memory load, then an uncorrected one
        let s = MemSentinel::new();
        let mut warned = false;
        for i in 0..20u64 {
            if s.on_error(i * 100, false, DEFAULT_THRESHOLD) == SentinelAction::Warn {
                warned = true;
            }
        }
        assert!(warned, "storm must trigger the instability warning");
        assert_eq!(s.on_error(2_100, true, DEFAULT_THRESHOLD), SentinelAction::Offline);
    }

    #[test]
    fn ffi_zero_threshold_is_clamped() {
        assert_eq!(rustlux_memsentinel_on_error(0, 0, 0), SentinelAction::Warn as i32);
    }
}
