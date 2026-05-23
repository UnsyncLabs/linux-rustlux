// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// params.rs — Parametros tunables de BORE (equivalente a los sysctl)
//
// En Linux estos se exponen via /proc/sys/kernel/sched_bore, etc.
// Aqui los definimos como constantes con sus valores por defecto,
// igual que en el patch bore-cachy.

/// Penalizacion maxima posible: (40 << 8) - 1 = 10239
pub const MAX_BURST_PENALTY: u32 = (40u32 << 8) - 1;

/// Limite de muestras para el cache de burst de hijos.
pub const BURST_CACHE_SAMPLE_LIMIT: usize = 63;

/// Limite de escaneo (2x el limite de muestras).
pub const BURST_CACHE_SCAN_LIMIT: usize = BURST_CACHE_SAMPLE_LIMIT * 2;

/// Shift para el timestamp en BoreBc (48 bits de timestamp, 16 de penalty).
pub const BORE_BC_TIMESTAMP_SHIFT: u32 = 16;

/// Parametros tunables de BORE con sus valores por defecto.
/// En el kernel real estos se exponen via sysctl.
#[derive(Debug, Clone)]
pub struct BoreParams {
    /// BORE habilitado (1) o deshabilitado (0).
    /// sysctl: kernel.sched_bore
    pub enabled: u8,

    /// Tipo de herencia de burst al hacer fork:
    /// 0 = sin herencia
    /// 1 = heredar del padre directo
    /// 2 = heredar del ancestor hub (default)
    /// sysctl: kernel.sched_burst_inherit_type
    pub burst_inherit_type: u8,

    /// Suavizado de la transicion entre bursts.
    /// 0 = sin suavizado, 3 = maximo suavizado.
    /// sysctl: kernel.sched_burst_smoothness
    pub burst_smoothness: u8,

    /// Offset de penalizacion: cuanto burst se tolera antes de penalizar.
    /// Unidades: bits de log2(burst_time).
    /// sysctl: kernel.sched_burst_penalty_offset
    pub burst_penalty_offset: u8,

    /// Escala de penalizacion (0-4095).
    /// sysctl: kernel.sched_burst_penalty_scale
    pub burst_penalty_scale: u32,

    /// Tiempo de vida del cache de burst (nanosegundos).
    /// sysctl: kernel.sched_burst_cache_lifetime
    pub burst_cache_lifetime_ns: u32,
}

impl Default for BoreParams {
    fn default() -> Self {
        Self {
            enabled:               1,
            burst_inherit_type:    2,    // ancestor hub
            burst_smoothness:      1,
            burst_penalty_offset:  24,
            burst_penalty_scale:   1536,
            burst_cache_lifetime_ns: 75_000_000, // 75ms
        }
    }
}
