// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// bore.rs — Implementacion del scheduler BORE
//
// BORE: Burst-Oriented Response Enhancer
// Basado en el trabajo de Masahito Suzuki (Copyright (C) 2021-2025)
// Adaptado a Rust para Rustlux.
//
// Concepto central:
//   Cada tarea acumula "burst_time" — tiempo que lleva corriendo sin dormir.
//   Las tareas con mucho burst_time reciben una penalizacion de prioridad.
//   Las tareas que duermen frecuentemente (interactivas) mantienen prioridad alta.
//
// Integracion con EEVDF:
//   BORE modifica el "effective priority" de cada tarea antes de que EEVDF
//   calcule su deadline virtual. Una tarea con score alto tiene menor prioridad
//   efectiva → su deadline se aleja → corre menos frecuentemente.
//
// AD: this is not connected to the kernel is just test wrapper that teorical can work. but is just and idea, im not going to implement this, maybe in a futher project i use the code.
// Im a big fan of this kernel modification, thats why i written this.

use crate::params::{BoreParams, BORE_BC_TIMESTAMP_SHIFT, BURST_CACHE_SAMPLE_LIMIT, MAX_BURST_PENALTY};

/// Cache de burst para un subarbol de procesos o thread group.
/// Empaqueta timestamp (48 bits) y penalty (16 bits) en un u64 atomico.
///
/// Layout:
///   bits 63..16 → timestamp (nanosegundos >> BORE_BC_TIMESTAMP_SHIFT)

///   bits 15..0  → penalty promedio del grupo

#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct BoreBc {
    /// Valor empaquetado: timestamp en bits altos, penalty en bits bajos.
    pub value: u64,
}

impl BoreBc {
    /// Extrae el timestamp del valor empaquetado.
    #[inline]
    pub fn timestamp(self) -> u64 {
        (self.value >> BORE_BC_TIMESTAMP_SHIFT) << BORE_BC_TIMESTAMP_SHIFT
    }

    /// Extrae la penalizacion del valor empaquetado.
    #[inline]
    pub fn penalty(self) -> u16 {
        self.value as u16
    }

    /// Empaqueta timestamp y penalty en un nuevo BoreBc.
    #[inline]
    pub fn pack(timestamp_ns: u64, penalty: u16) -> Self {
        let ts_shifted = timestamp_ns >> BORE_BC_TIMESTAMP_SHIFT;
        Self {
            value: (ts_shifted << BORE_BC_TIMESTAMP_SHIFT) | (penalty as u64),
        }
    }

    /// Verifica si el cache ha expirado dado el tiempo actual y el lifetime.
    #[inline]
    pub fn is_expired(self, now_ns: u64, lifetime_ns: u64) -> bool {
        now_ns.wrapping_sub(self.timestamp()) > lifetime_ns
    }
}

/// Contexto BORE por tarea. Se embebe en task_struct (en C) o en Task (en Rust).
///
/// Equivalente a `struct bore_ctx` del patch bore-cachy.
#[derive(Debug, Default, Clone)]
#[repr(C)]
pub struct BoreCtx {
    /// Tiempo acumulado corriendo sin dormir (nanosegundos).
    pub burst_time: u64,

    /// Penalizacion del burst anterior (para suavizado).
    pub prev_penalty: u16,

    /// Penalizacion del burst actual.
    pub curr_penalty: u16,

    /// Penalizacion efectiva (max de prev y curr, 0 para kthreads).
    pub penalty: u16,

    /// Score de prioridad derivado de penalty (0-39, se suma al static_prio).
    pub score: u8,

    /// Evita reentrada durante reweight_entity().
    pub stop_update: bool,

    /// Marcado true mientras la tarea espera en futex.
    /// Las esperas en futex no deben penalizarse como burst.
    pub futex_waiting: bool,

    /// Cache de penalizacion del subarbol de hijos directos.
    pub subtree: BoreBc,

    /// Cache de penalizacion del thread group.
    pub group: BoreBc,
}

impl BoreCtx {
    /// Crea un BoreCtx vacio (para init_task y nuevas tareas sin herencia).
    pub const fn new() -> Self {
        Self {
            burst_time:    0,
            prev_penalty:  0,
            curr_penalty:  0,
            penalty:       0,
            score:         0,
            stop_update:   false,
            futex_waiting: false,
            subtree:       BoreBc { value: 0 },
            group:         BoreBc { value: 0 },
        }
    }
}

/// Calcula log2(v) + 1 en punto fijo con `fp` bits de fraccion.
/// Retorna 0 si v == 0.
///
/// Equivalente a `log2p1_u64_u32fp` del patch bore-cachy.
#[inline]
pub fn log2p1_u64_fp(v: u64, fp: u8) -> u32 {
    if v == 0 {
        return 0;
    }
    let clz = v.leading_zeros();
    let exponent = 64 - clz;
    // Mantissa: los bits fraccionarios despues del bit implicito
    let mantissa = ((v << clz) << 1 >> (64 - fp as u32)) as u32;
    (exponent << fp as u32) | mantissa
}

/// Calcula la penalizacion de burst dado el tiempo acumulado.
///
/// Formula: penalty = clamp(max(0, log2(burst_time) - offset) * scale / 1024, MAX)
///
/// Equivalente a `calc_burst_penalty` del patch bore-cachy.
#[inline]
pub fn calc_burst_penalty(burst_time: u64, params: &BoreParams) -> u32 {
    let greed     = log2p1_u64_fp(burst_time, 8);
    let tolerance = (params.burst_penalty_offset as u32) << 8;

    // diff puede ser negativo — si greed < tolerance, no hay penalizacion
    let diff = greed.wrapping_sub(tolerance) as i32;
    // penalty = max(0, diff)
    let penalty = (diff & !(diff >> 31)) as u32;

    let scaled = penalty * params.burst_penalty_scale >> 10;

    // Clamp a MAX_BURST_PENALTY
    let overflow = scaled.wrapping_sub(MAX_BURST_PENALTY) as i32;
    scaled - (overflow & !(overflow >> 31)) as u32
}

/// Suavizado binario entre valor nuevo y viejo.
/// Si new > old: suaviza el incremento con un shift.
/// Si new <= old: retorna new directamente (bajadas son inmediatas).
///
/// Equivalente a `binary_smooth` del patch bore-cachy.
#[inline]
pub fn binary_smooth(new: u32, old: u32, smoothness: u8) -> u32 {
    if new > old {
        let increment = new - old;
        let shift = smoothness as u32;
        old + ((increment + (1u32 << shift) - 1) >> shift)
    } else {
        new
    }
}

/// Calcula la prioridad efectiva de una tarea con BORE.
///
/// effective_prio = clamp(static_prio - MAX_RT_PRIO + score, 0, 39)
///
/// Equivalente a `effective_prio_bore` del patch bore-cachy.
///
/// # Parametros
/// - `static_prio_offset`: static_prio - MAX_RT_PRIO (0-39 para tareas normales)
/// - `score`: bore score de la tarea (0-39)
/// - `bore_enabled`: si BORE esta activo
#[inline]
pub fn effective_prio(static_prio_offset: u8, score: u8, bore_enabled: bool) -> u8 {
    let prio = if bore_enabled {
        (static_prio_offset as u32) + (score as u32)
    } else {
        static_prio_offset as u32
    };
    // Clamp a [0, 39]
    prio.min(39) as u8
}

/// Actualiza el BoreCtx de una tarea despues de un periodo de ejecucion.
///
/// Llamado desde el tick del scheduler (equivalente a `update_curr_bore`).
///
/// # Parametros
/// - `ctx`: contexto BORE de la tarea
/// - `delta_ns`: tiempo de ejecucion desde la ultima actualizacion (nanosegundos)
/// - `params`: parametros globales de BORE
/// - `is_kthread`: true si la tarea es un kernel thread (no se penalizan)
///
/// Retorna true si la penalizacion cambio (necesita reweight).
pub fn update_curr(
    ctx: &mut BoreCtx,
    delta_ns: u64,
    params: &BoreParams,
    is_kthread: bool,
) -> bool {
    if ctx.stop_update {
        return false;
    }

    ctx.burst_time = ctx.burst_time.saturating_add(delta_ns);
    let curr_penalty = calc_burst_penalty(ctx.burst_time, params) as u16;
    ctx.curr_penalty = curr_penalty;

    if curr_penalty <= ctx.prev_penalty {
        return false;
    }

    // Actualizar penalty efectiva
    let diff = ctx.curr_penalty as i32 - ctx.prev_penalty as i32;
    let max_val = ctx.curr_penalty - (diff & (diff >> 31)) as u16;

    // Los kthreads no reciben penalizacion
    ctx.penalty = if is_kthread { 0 } else { max_val };

    // Score: byte alto de penalty (0-39 aproximadamente)
    ctx.score = (ctx.penalty >> 8) as u8;

    true // necesita reweight
}

/// Reinicia el burst de una tarea cuando va a dormir.
///
/// Equivalente a `restart_burst_bore`.
pub fn restart_burst(ctx: &mut BoreCtx, params: &BoreParams, is_kthread: bool) {
    let new_penalty = binary_smooth(
        ctx.curr_penalty as u32,
        ctx.prev_penalty as u32,
        params.burst_smoothness,
    ) as u16;

    ctx.prev_penalty = new_penalty;
    ctx.curr_penalty = 0;
    ctx.burst_time   = 0;

    // Recalcular penalty efectiva
    ctx.penalty = if is_kthread { 0 } else { new_penalty };
    ctx.score   = (ctx.penalty >> 8) as u8;
}

/// Tabla de reciprocos para calcular promedios sin division.
/// bore_reciprocal_lut[i] = ((0xFFFFFFFF + i) / i) as u32
/// para i en 1..=BURST_CACHE_SAMPLE_LIMIT
///
/// Nota: para i=1, el valor exacto es 0x100000000 que truncado a u32 = 0.
/// Esto es intencional — cuando count==1, `average_penalty` retorna `total`
/// directamente sin usar la LUT (ver `average_penalty`).
pub fn build_reciprocal_lut() -> [u32; BURST_CACHE_SAMPLE_LIMIT + 1] {
    let mut lut = [0u32; BURST_CACHE_SAMPLE_LIMIT + 1];
    for i in 1..=BURST_CACHE_SAMPLE_LIMIT {
        // Truncacion intencional a u32, igual que el cast en el patch C original
        lut[i] = ((0xFFFF_FFFFu64 + i as u64) / i as u64) as u32;
    }
    lut
}

/// Calcula el promedio de penalidades usando la LUT de reciprocos.
#[inline]
pub fn average_penalty(total: u32, count: usize, lut: &[u32]) -> u32 {
    if count == 0 {
        return 0;
    }
    if count == 1 {
        return total;
    }
    let recip = lut[count.min(BURST_CACHE_SAMPLE_LIMIT)];
    ((total as u64 * recip as u64) >> 32) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log2p1_zero() {
        assert_eq!(log2p1_u64_fp(0, 8), 0);
    }

    #[test]
    fn log2p1_one() {
        // log2(1) + 1 = 1, en fp8 = 256
        assert_eq!(log2p1_u64_fp(1, 8), 256);
    }

    #[test]
    fn no_penalty_for_short_burst() {
        let params = BoreParams::default();
        // burst_time pequeno → penalty = 0
        let p = calc_burst_penalty(1_000, &params);
        assert_eq!(p, 0);
    }

    #[test]
    fn penalty_grows_with_burst() {
        let params = BoreParams::default();
        let p_short = calc_burst_penalty(1_000_000, &params);
        let p_long  = calc_burst_penalty(1_000_000_000, &params);
        assert!(p_long > p_short, "penalty debe crecer con burst_time");
    }

    #[test]
    fn penalty_clamped_to_max() {
        let params = BoreParams::default();
        // burst_time enorme → penalty clamped a MAX_BURST_PENALTY
        let p = calc_burst_penalty(u64::MAX, &params);
        assert!(p <= MAX_BURST_PENALTY);
    }

    #[test]
    fn smooth_going_up() {
        // Con smoothness=1, el incremento se divide por 2 (redondeado arriba)
        let result = binary_smooth(100, 0, 1);
        assert_eq!(result, 50);
    }

    #[test]
    fn smooth_going_down_is_immediate() {
        let result = binary_smooth(10, 100, 1);
        assert_eq!(result, 10);
    }

    #[test]
    fn effective_prio_clamped() {
        // score muy alto no debe superar 39
        let prio = effective_prio(30, 20, true);
        assert_eq!(prio, 39);
    }

    #[test]
    fn kthread_no_penalty() {
        let params = BoreParams::default();
        let mut ctx = BoreCtx::new();
        // Simular mucho burst en un kthread
        update_curr(&mut ctx, 10_000_000_000, &params, true);
        assert_eq!(ctx.penalty, 0, "kthreads no deben recibir penalizacion");
        assert_eq!(ctx.score, 0);
    }

    #[test]
    fn futex_wait_flag() {
        let mut ctx = BoreCtx::new();
        ctx.futex_waiting = true;
        assert!(ctx.futex_waiting);
        ctx.futex_waiting = false;
        assert!(!ctx.futex_waiting);
    }

    #[test]
    fn reciprocal_lut_correctness() {
        let lut = build_reciprocal_lut();
        // lut[1]: (0xFFFFFFFF + 1) / 1 = 0x100000000, truncado a u32 = 0
        // Por eso average_penalty maneja count==1 como caso especial
        assert_eq!(lut[1], 0x0000_0000);
        // lut[2]: (0xFFFFFFFF + 2) / 2 = 0x80000000
        assert_eq!(lut[2], 0x8000_0000);
        // lut[3]: (0x100000002) / 3 = 0x55555556 (redondeo entero)
        assert_eq!(lut[3], 0x5555_5556);
        // La LUT debe ser monotonamente decreciente (reciprocos)
        for i in 2..BURST_CACHE_SAMPLE_LIMIT {
            assert!(lut[i] >= lut[i + 1],
                "lut[{}]={} debe ser >= lut[{}]={}", i, lut[i], i+1, lut[i+1]);
        }
    }

    #[test]
    fn restart_burst_resets_time() {
        let params = BoreParams::default();
        let mut ctx = BoreCtx::new();
        update_curr(&mut ctx, 5_000_000_000, &params, false);
        assert!(ctx.burst_time > 0);
        restart_burst(&mut ctx, &params, false);
        assert_eq!(ctx.burst_time, 0);
        assert_eq!(ctx.curr_penalty, 0);
    }
}
