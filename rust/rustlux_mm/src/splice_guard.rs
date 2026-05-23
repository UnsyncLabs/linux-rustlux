// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// splice_guard.rs — permission check for splice() to page cache

use crate::page_cache::{InodeFlags, PageWriteError, PageWritePermit};

const EPERM: i32 = -1;
const EROFS: i32 = -30;
const EACCES: i32 = -13;
const S_ISUID: u32 = 0o4000;
const S_ISGID: u32 = 0o2000;

/// checks if splice write to page cache is allowed.
/// returns 0 if allowed, negative errno if denied.
#[no_mangle]
pub extern "C" fn rustlux_check_splice_write(
    i_mode: u32,
    i_flags: u32,
    sb_readonly: u8,
) -> i32 {
    let mut flags_raw: u32 = 0;

    if i_mode & S_ISUID != 0 { flags_raw |= InodeFlags::SUID.0; }
    if i_mode & S_ISGID != 0 { flags_raw |= InodeFlags::SGID.0; }
    if i_flags & 0x10 != 0 { flags_raw |= InodeFlags::IMMUTABLE.0; }
    if i_flags & 0x20 != 0 { flags_raw |= InodeFlags::APPEND_ONLY.0; }

    let inode_flags = InodeFlags::from_raw(flags_raw);

    match PageWritePermit::new(inode_flags, sb_readonly != 0) {
        Ok(_) => 0,
        Err(PageWriteError::SuidOrSgid) => EPERM,
        Err(PageWriteError::ReadOnlyFilesystem) => EROFS,
        Err(PageWriteError::Immutable) => EPERM,
        Err(PageWriteError::AppendOnly) => EPERM,
        Err(PageWriteError::PermissionDenied) => EACCES,
    }
}

#[no_mangle]
pub extern "C" fn rustlux_check_alg_splice_write(i_mode: u32, i_flags: u32, sb_readonly: u8) -> i32 {
    rustlux_check_splice_write(i_mode, i_flags, sb_readonly)
}

#[no_mangle]
pub extern "C" fn rustlux_check_xfrm_splice_write(i_mode: u32, i_flags: u32, sb_readonly: u8) -> i32 {
    rustlux_check_splice_write(i_mode, i_flags, sb_readonly)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_file_splice_allowed() {
        assert_eq!(rustlux_check_splice_write(0o644, 0, 0), 0);
    }

    #[test]
    fn suid_file_splice_denied() {
        assert_eq!(rustlux_check_splice_write(0o4755, 0, 0), EPERM);
    }

    #[test]
    fn sgid_file_splice_denied() {
        assert_eq!(rustlux_check_splice_write(0o2755, 0, 0), EPERM);
    }

    #[test]
    fn readonly_fs_splice_denied() {
        assert_eq!(rustlux_check_splice_write(0o644, 0, 1), EROFS);
    }

    #[test]
    fn immutable_file_splice_denied() {
        assert_eq!(rustlux_check_splice_write(0o644, 0x10, 0), EPERM);
    }

    #[test]
    fn append_only_splice_denied() {
        assert_eq!(rustlux_check_splice_write(0o644, 0x20, 0), EPERM);
    }

    #[test]
    fn copy_fail_scenario_blocked() {
        assert_eq!(rustlux_check_splice_write(0o104755, 0, 0), EPERM);
    }

    #[test]
    fn dirty_frag_scenario_blocked() {
        assert_eq!(rustlux_check_xfrm_splice_write(0o104755, 0, 0), EPERM);
    }
}
