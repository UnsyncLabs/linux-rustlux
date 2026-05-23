// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// rustlux_sched — bore scheduler implementation for rustlux
//
// bore (burst-oriented response enhancer) modifies the eevdf scheduler
// in linux. it penalizes tasks that consume cpu without sleeping so that
// interactive tasks (compositor, terminal, audio) keep high priority
// even under heavy load.
//
// inspired by the bore-cachy patch by Masahito Suzuki (firelzrd@gmail.com).
// original bore concept: Copyright (C) 2021-2025 Masahito Suzuki.
// this rust implementation was written from scratch by neokuze.

#![no_std]
#![deny(unsafe_op_in_unsafe_fn)]
#![warn(missing_docs)]

/// BORE scheduler logic — pure Rust, memory-safe, testeable.
pub mod bore;
/// BORE tunable parameters (sysctl equivalents).
pub mod params;
/// FFI interface for bore.c in the C kernel.
pub mod bore_ffi;
