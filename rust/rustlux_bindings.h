/* SPDX-License-Identifier: GPL-2.0-only */
/*
 * rustlux_bindings.h — C header for Rustlux FFI functions
 *
 * Include this from kernel C code to call Rustlux Rust functions.
 * These functions are implemented in:
 *   - rust/rustlux_sched/src/bore_ffi.rs
 *   - rust/rustlux_mm/src/splice_guard.rs
 *   - rust/rustlux_security/src/hardening_ffi.rs
 */

#ifndef _RUSTLUX_BINDINGS_H
#define _RUSTLUX_BINDINGS_H

#include <linux/types.h>

/* ── BORE Scheduler (rustlux_sched) ──────────────────────────── */

struct bore_params_ffi {
	u8  enabled;
	u8  burst_inherit_type;
	u8  burst_smoothness;
	u8  burst_penalty_offset;
	u32 burst_penalty_scale;
	u32 burst_cache_lifetime;
};

/*
 * Update BORE context after a task runs for delta_ns nanoseconds.
 * Returns 1 if penalty changed (needs reweight), 0 otherwise.
 */
extern u8 rustlux_bore_update_curr(
	struct bore_ctx *ctx,
	const struct bore_params_ffi *params,
	u64 delta_ns,
	u8 is_kthread
);

/*
 * Restart burst when a task goes to sleep.
 */
extern void rustlux_bore_restart_burst(
	struct bore_ctx *ctx,
	const struct bore_params_ffi *params,
	u8 is_kthread
);

/*
 * Calculate effective priority with BORE.
 * Returns priority offset (0-39).
 */
extern u8 rustlux_bore_effective_prio(
	u8 static_prio_offset,
	u8 score,
	u8 bore_enabled
);

/*
 * Calculate burst penalty for a given burst_time.
 */
extern u32 rustlux_bore_calc_penalty(
	u64 burst_time,
	const struct bore_params_ffi *params
);

/*
 * Reset a bore_ctx to zero (for init_task and new tasks).
 */
extern void rustlux_bore_reset(struct bore_ctx *ctx);

/* ── Page Cache / Splice Guard (rustlux_mm) ──────────────────── */

/*
 * Check if splice() to page cache is allowed for the given inode.
 * Returns 0 if allowed, negative errno if denied.
 *
 * Mitigates: Dirty Pipe, Copy Fail, Dirty Frag
 *
 * Usage in fs/splice.c:
 *   int ret = rustlux_check_splice_write(
 *       inode->i_mode, inode->i_flags, IS_RDONLY(inode));
 *   if (ret < 0) return ret;
 */
extern int rustlux_check_splice_write(
	u32 i_mode,
	u32 i_flags,
	u8 sb_readonly
);

/*
 * Same check, named for AF_ALG (crypto) splice path.
 * Mitigates: Copy Fail (CVE-2026-31431)
 */
extern int rustlux_check_alg_splice_write(
	u32 i_mode,
	u32 i_flags,
	u8 sb_readonly
);

/*
 * Same check, named for xfrm-ESP splice path.
 * Mitigates: Dirty Frag (CVE-2026-43284)
 */
extern int rustlux_check_xfrm_splice_write(
	u32 i_mode,
	u32 i_flags,
	u8 sb_readonly
);

/* ── Security / Hardening (rustlux_security) ─────────────────── */

/*
 * Check if perf_event is allowed for the given capabilities.
 * Returns 1 if allowed, 0 if denied.
 */
extern u8 rustlux_perf_event_allowed(u64 cap_effective, int paranoid);

/*
 * Check if TIOCSTI ioctl is allowed.
 * Returns 1 if allowed, 0 if denied.
 */
extern u8 rustlux_tiocsti_allowed(u64 cap_effective, u8 same_tty);

/*
 * Check if new USB device connection is allowed.
 * Returns 1 if allowed, 0 if denied.
 */
extern u8 rustlux_usb_new_device_allowed(void);

/*
 * Get the configured perf_event_paranoid level.
 */
extern int rustlux_perf_paranoid_level(void);

/*
 * Get filesystem protection levels.
 */
extern u8 rustlux_protected_symlinks(void);
extern u8 rustlux_protected_hardlinks(void);
extern u8 rustlux_protected_fifos(void);
extern u8 rustlux_protected_regular(void);

#endif /* _RUSTLUX_BINDINGS_H */
