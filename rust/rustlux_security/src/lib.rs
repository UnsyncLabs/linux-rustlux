// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// rustlux_security — hardening and security abstractions for rustlux
//
// implements protections from the linux-hardened patch:
// - maximum aslr by default
// - symlink/hardlink/fifo protections
// - capability restrictions
// - tiocsti_restrict
// - deny_new_usb

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

/// Hardening configuration — defaults from linux-hardened patch.
pub mod hardening;
/// Capabilities abstraction — CapSet, permission checks.
pub mod capabilities;
/// FFI interface for C kernel hardening checks.
pub mod hardening_ffi;
