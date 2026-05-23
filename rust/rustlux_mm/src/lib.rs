// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// rustlux_mm — memory management abstractions for rustlux
//
// implements page cache write protections that eliminate the class of
// vulnerabilities where subsystems can mark pages as dirty without
// verifying permissions on the underlying file. access to page cache
// writes is modeled with a permission token (PageWritePermit) that
// can only be obtained if the file has real write permissions.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

/// Page cache write protection — eliminates Dirty Pipe/Copy Fail/Dirty Frag class.
pub mod page_cache;
/// Read-only after init — Rust equivalent of __ro_after_init.
pub mod ro_after_init;
/// Splice guard — FFI for fs/splice.c write permission checks.
pub mod splice_guard;
