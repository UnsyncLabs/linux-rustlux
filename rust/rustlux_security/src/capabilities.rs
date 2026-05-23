// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// capabilities.rs — linux capability abstractions

/// linux capabilities relevant for rustlux security checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cap {
    SysAdmin = 21,
    PerfMon = 38,
    DacOverride = 1,
    DacReadSearch = 2,
    NetAdmin = 12,
    SysModule = 16,
}

/// capability set as 64-bit bitmask.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CapSet(u64);

impl CapSet {
    pub const EMPTY: Self = Self(0);
    pub const FULL: Self = Self(u64::MAX);

    #[inline]
    pub const fn from_raw(raw: u64) -> Self { Self(raw) }

    #[inline]
    pub const fn has(self, cap: Cap) -> bool { (self.0 >> (cap as u64)) & 1 == 1 }

    #[inline]
    pub const fn with(self, cap: Cap) -> Self { Self(self.0 | (1u64 << cap as u64)) }

    #[inline]
    pub const fn without(self, cap: Cap) -> Self { Self(self.0 & !(1u64 << cap as u64)) }
}

/// check if perf_event access is allowed.
#[inline]
pub fn perf_allowed(caps: CapSet, paranoid: i32) -> bool {
    match paranoid {
        p if p <= 0 => true,
        _ => caps.has(Cap::PerfMon),
    }
}

/// check if tiocsti is allowed.
#[inline]
pub fn tiocsti_allowed(caps: CapSet, restrict: bool, same_tty: bool) -> bool {
    if same_tty && !restrict { return true; }
    caps.has(Cap::SysAdmin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capset_has() {
        let caps = CapSet::EMPTY.with(Cap::SysAdmin);
        assert!(caps.has(Cap::SysAdmin));
        assert!(!caps.has(Cap::PerfMon));
    }

    #[test]
    fn capset_without() {
        let caps = CapSet::FULL.without(Cap::SysAdmin);
        assert!(!caps.has(Cap::SysAdmin));
        assert!(caps.has(Cap::PerfMon));
    }

    #[test]
    fn perf_paranoid_3_requires_cap() {
        assert!(!perf_allowed(CapSet::EMPTY, 3));
        assert!(perf_allowed(CapSet::EMPTY.with(Cap::PerfMon), 3));
    }

    #[test]
    fn perf_paranoid_0_allows_all() {
        assert!(perf_allowed(CapSet::EMPTY, 0));
    }

    #[test]
    fn tiocsti_restrict_requires_sysadmin() {
        assert!(!tiocsti_allowed(CapSet::EMPTY, true, true));
        assert!(tiocsti_allowed(CapSet::EMPTY.with(Cap::SysAdmin), true, true));
    }

    #[test]
    fn tiocsti_same_tty_no_restrict() {
        assert!(tiocsti_allowed(CapSet::EMPTY, false, true));
    }
}
