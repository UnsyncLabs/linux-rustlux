// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// bore_ffi.rs — Interfaz FFI entre bore.c (C) y la logica BORE en Rust
//
// Este modulo exporta funciones `extern "C"` que bore.c llama directamente.
// La logica pura de BORE vive en bore.rs (memory-safe, testeable).
// Aqui solo hacemos la conversion de tipos C ↔ Rust.
//
// Flujo:
//   bore.c (C) → bore_ffi.rs (FFI) → bore.rs (logica pura Rust)

use crate::bore::{self, BoreCtx};
use crate::params::BoreParams;

/// Representacion FFI del BoreCtx para C.
/// Debe tener el mismo layout que `struct bore_ctx` en include/linux/sched.h.
/// Usamos #[repr(C)] para garantizar compatibilidad de layout.
#[repr(C)]
pub struct BoreCtxFfi {
    pub burst_time:    u64,
    pub prev_penalty:  u16,
    pub curr_penalty:  u16,
    pub penalty:       u16,
    pub score:         u8,
    pub _pad:          u8,
    pub stop_update:   u8, // bool en C es u8
    pub futex_waiting: u8,
    pub _pad2:         [u8; 6],
    pub subtree_value: u64,
    pub group_value:   u64,
}

/// Parametros globales de BORE — inicializados desde los sysctl de C.
/// En el kernel real, bore.c mantiene estos como variables globales.
/// Aqui los leemos desde C en cada llamada (son __read_mostly, no cambian frecuentemente).
#[repr(C)]
pub struct BoreParamsFfi {
    pub enabled:               u8,
    pub burst_inherit_type:    u8,
    pub burst_smoothness:      u8,
    pub burst_penalty_offset:  u8,
    pub burst_penalty_scale:   u32,
    pub burst_cache_lifetime:  u32,
}

impl From<&BoreParamsFfi> for BoreParams {
    fn from(ffi: &BoreParamsFfi) -> Self {
        BoreParams {
            enabled:               ffi.enabled,
            burst_inherit_type:    ffi.burst_inherit_type,
            burst_smoothness:      ffi.burst_smoothness,
            burst_penalty_offset:  ffi.burst_penalty_offset,
            burst_penalty_scale:   ffi.burst_penalty_scale,
            burst_cache_lifetime_ns: ffi.burst_cache_lifetime,
        }
    }
}

/// Convierte un BoreCtxFfi (de C) a un BoreCtx (Rust) para operar.
fn ffi_to_ctx(ffi: &BoreCtxFfi) -> BoreCtx {
    BoreCtx {
        burst_time:    ffi.burst_time,
        prev_penalty:  ffi.prev_penalty,
        curr_penalty:  ffi.curr_penalty,
        penalty:       ffi.penalty,
        score:         ffi.score,
        stop_update:   ffi.stop_update != 0,
        futex_waiting: ffi.futex_waiting != 0,
        subtree:       bore::BoreBc { value: ffi.subtree_value },
        group:         bore::BoreBc { value: ffi.group_value },
    }
}

/// Escribe un BoreCtx (Rust) de vuelta al BoreCtxFfi (C).
fn ctx_to_ffi(ctx: &BoreCtx, ffi: &mut BoreCtxFfi) {
    ffi.burst_time    = ctx.burst_time;
    ffi.prev_penalty  = ctx.prev_penalty;
    ffi.curr_penalty  = ctx.curr_penalty;
    ffi.penalty       = ctx.penalty;
    ffi.score         = ctx.score;
    ffi.stop_update   = ctx.stop_update as u8;
    ffi.futex_waiting = ctx.futex_waiting as u8;
    ffi.subtree_value = ctx.subtree.value;
    ffi.group_value   = ctx.group.value;
}

/// Actualiza el contexto BORE despues de un periodo de ejecucion.
///
/// Llamada desde `update_curr_bore()` en bore.c.
///
/// # Safety
///
/// - `ctx_ptr` debe ser un puntero valido a un `bore_ctx` embebido en task_struct.
/// - `params_ptr` debe ser un puntero valido a los parametros globales de BORE.
/// - El caller debe tener el rq lock de la tarea.
///
/// Retorna 1 si la penalizacion cambio (necesita reweight), 0 si no.
#[no_mangle]
pub unsafe extern "C" fn rustlux_bore_update_curr(
    ctx_ptr: *mut BoreCtxFfi,
    params_ptr: *const BoreParamsFfi,
    delta_ns: u64,
    is_kthread: u8,
) -> u8 {
    // SAFETY: El caller (bore.c) garantiza que ctx_ptr es valido y que
    // tiene el rq lock, por lo que no hay data races.
    let ffi_ctx = unsafe { &mut *ctx_ptr };
    let ffi_params = unsafe { &*params_ptr };

    let mut ctx = ffi_to_ctx(ffi_ctx);
    let params = BoreParams::from(ffi_params);

    let changed = bore::update_curr(&mut ctx, delta_ns, &params, is_kthread != 0);

    ctx_to_ffi(&ctx, ffi_ctx);

    changed as u8
}

/// Reinicia el burst de una tarea cuando va a dormir.
///
/// Llamada desde `restart_burst_bore()` en bore.c.
///
/// # Safety
///
/// - `ctx_ptr` debe ser un puntero valido a un `bore_ctx`.
/// - `params_ptr` debe ser un puntero valido a los parametros globales.
/// - El caller debe tener el rq lock.
#[no_mangle]
pub unsafe extern "C" fn rustlux_bore_restart_burst(
    ctx_ptr: *mut BoreCtxFfi,
    params_ptr: *const BoreParamsFfi,
    is_kthread: u8,
) {
    let ffi_ctx = unsafe { &mut *ctx_ptr };
    let ffi_params = unsafe { &*params_ptr };

    let mut ctx = ffi_to_ctx(ffi_ctx);
    let params = BoreParams::from(ffi_params);

    bore::restart_burst(&mut ctx, &params, is_kthread != 0);

    ctx_to_ffi(&ctx, ffi_ctx);
}

/// Calcula la prioridad efectiva de una tarea con BORE.
///
/// Llamada desde `effective_prio_bore()` en bore.c y core.c.
///
/// # Safety
///
/// - `ctx_ptr` debe ser un puntero valido a un `bore_ctx`.
/// - `bore_enabled` indica si BORE esta activo (static key en C).
#[no_mangle]
pub unsafe extern "C" fn rustlux_bore_effective_prio(
    static_prio_offset: u8,
    score: u8,
    bore_enabled: u8,
) -> u8 {
    bore::effective_prio(static_prio_offset, score, bore_enabled != 0)
}

/// Calcula la penalizacion de burst para un tiempo dado.
///
/// Funcion auxiliar exportada para que bore.c pueda usarla
/// sin reimplementar la logica en C.
///
/// # Safety
///
/// - `params_ptr` debe ser un puntero valido.
#[no_mangle]
pub unsafe extern "C" fn rustlux_bore_calc_penalty(
    burst_time: u64,
    params_ptr: *const BoreParamsFfi,
) -> u32 {
    let ffi_params = unsafe { &*params_ptr };
    let params = BoreParams::from(ffi_params);
    bore::calc_burst_penalty(burst_time, &params)
}

/// Resetea un BoreCtx a cero (para init_task y nuevas tareas).
///
/// # Safety
///
/// - `ctx_ptr` debe ser un puntero valido.
#[no_mangle]
pub unsafe extern "C" fn rustlux_bore_reset(ctx_ptr: *mut BoreCtxFfi) {
    let ffi_ctx = unsafe { &mut *ctx_ptr };
    let ctx = BoreCtx::new();
    ctx_to_ffi(&ctx, ffi_ctx);
}
