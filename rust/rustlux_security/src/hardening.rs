// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// hardening.rs — Defaults de seguridad del patch linux-hardened
//
// Todas las constantes aqui reflejan los valores por defecto del patch
// linux-hardened-v7.0.9-hardened1. En el kernel C se aplican via Kconfig
// y sysctl; aqui los documentamos como tipos Rust para los subsistemas
// que escribamos en Rust.

/// Configuracion de protecciones del filesystem.
/// Equivalente a los sysctl fs.protected_* del patch hardened.
#[derive(Debug, Clone)]
pub struct FsProtection {
    /// fs.protected_symlinks = 1
    /// Previene seguir symlinks en sticky dirs si el owner no coincide.
    /// Mitiga ataques TOCTOU en /tmp.
    pub symlinks: u8,

    /// fs.protected_hardlinks = 1
    /// Previene crear hardlinks a archivos que no son del usuario.
    pub hardlinks: u8,

    /// fs.protected_fifos = 2
    /// Previene abrir FIFOs en sticky dirs si el owner no coincide.
    /// Nivel 2 = proteccion maxima.
    pub fifos: u8,

    /// fs.protected_regular = 2
    /// Previene abrir archivos regulares en sticky dirs si el owner no coincide.
    pub regular: u8,
}

impl Default for FsProtection {
    fn default() -> Self {
        Self {
            symlinks:  1,
            hardlinks: 1,
            fifos:     2,
            regular:   2,
        }
    }
}

/// Configuracion de ASLR.
/// El patch hardened cambia los defaults de ARCH_MMAP_RND_BITS a MAX.
#[derive(Debug, Clone)]
pub struct AslrConfig {
    /// Bits de entropia para mmap base (x86_64: hasta 32 bits).
    /// Hardened default: ARCH_MMAP_RND_BITS_MAX (maximo posible).
    pub mmap_rnd_bits: u8,

    /// Bits de entropia para mmap compat (32-bit apps en 64-bit kernel).
    /// Hardened default: ARCH_MMAP_RND_COMPAT_BITS_MAX.
    pub mmap_rnd_compat_bits: u8,

    /// Stack randomization habilitada por defecto.
    /// RANDOMIZE_KSTACK_OFFSET_DEFAULT = y en el patch hardened.
    pub kstack_offset: bool,
}

impl Default for AslrConfig {
    fn default() -> Self {
        Self {
            mmap_rnd_bits:        32, // maximo en x86_64
            mmap_rnd_compat_bits: 16, // maximo para compat
            kstack_offset:        true,
        }
    }
}

/// Configuracion de TTY security.
/// Del patch hardened: tiocsti_restrict y legacy_tiocsti.
#[derive(Debug, Clone)]
pub struct TtyConfig {
    /// Restringir TIOCSTI a CAP_SYS_ADMIN.
    /// Previene que procesos sin privilegios inyecten comandos en otras TTYs.
    /// kernel.tiocsti_restrict = 1 por defecto en hardened.
    pub tiocsti_restrict: bool,

    /// Deshabilitar TIOCSTI legacy completamente.
    /// CONFIG_LEGACY_TIOCSTI no tiene default y en hardened se deja sin marcar.
    pub legacy_tiocsti: bool,
}

impl Default for TtyConfig {
    fn default() -> Self {
        Self {
            tiocsti_restrict: true,
            legacy_tiocsti:   false,
        }
    }
}

/// Configuracion de USB.
/// Del patch hardened: deny_new_usb sysctl.
#[derive(Debug, Clone)]
pub struct UsbConfig {
    /// kernel.deny_new_usb = 0 por defecto (se puede activar en runtime).
    /// Cuando es true, bloquea la conexion de nuevos dispositivos USB.
    /// Útil en servidores o sistemas donde el USB debe estar bloqueado.
    pub deny_new_usb: bool,
}

impl Default for UsbConfig {
    fn default() -> Self {
        Self {
            deny_new_usb: false, // off por defecto, activable via sysctl
        }
    }
}

/// Configuracion de perf_event.
/// Del patch hardened: perf_event_paranoid >= 3 deshabilita perf para usuarios.
#[derive(Debug, Clone)]
pub struct PerfConfig {
    /// kernel.perf_event_paranoid
    /// 0 = sin restricciones
    /// 1 = sin CPU events para usuarios sin CAP_PERFMON
    /// 2 = sin kernel profiling para usuarios sin CAP_PERFMON
    /// 3 = sin ningun evento para usuarios sin CAP_PERFMON (hardened default)
    pub paranoid: i32,
}

impl Default for PerfConfig {
    fn default() -> Self {
        Self { paranoid: 3 }
    }
}

/// Configuracion de overlayfs.
/// Del patch hardened: OVERLAY_FS_UNPRIVILEGED = n por defecto.
#[derive(Debug, Clone)]
pub struct OverlayFsConfig {
    /// Permitir mounts de overlayfs sin privilegios.
    /// false = solo root puede montar overlayfs (mas seguro).
    /// Overlayfs ha sido vector de varios LPE recientes.
    pub unprivileged_mounts: bool,
}

impl Default for OverlayFsConfig {
    fn default() -> Self {
        Self {
            unprivileged_mounts: false,
        }
    }
}

/// Configuracion global de hardening de Rustlux.
/// Agrupa todos los subsistemas de seguridad con sus defaults hardened.
#[derive(Debug, Clone, Default)]
pub struct HardeningConfig {
    pub fs:       FsProtection,
    pub aslr:     AslrConfig,
    pub tty:      TtyConfig,
    pub usb:      UsbConfig,
    pub perf:     PerfConfig,
    pub overlayfs: OverlayFsConfig,
}

impl HardeningConfig {
    /// Configuracion hardened maxima (todos los toggles al maximo de seguridad).
    pub fn maximum() -> Self {
        Self {
            fs: FsProtection {
                symlinks:  1,
                hardlinks: 1,
                fifos:     2,
                regular:   2,
            },
            aslr: AslrConfig {
                mmap_rnd_bits:        32,
                mmap_rnd_compat_bits: 16,
                kstack_offset:        true,
            },
            tty: TtyConfig {
                tiocsti_restrict: true,
                legacy_tiocsti:   false,
            },
            usb: UsbConfig {
                deny_new_usb: true, // maximo: bloquear USB
            },
            perf: PerfConfig {
                paranoid: 3,
            },
            overlayfs: OverlayFsConfig {
                unprivileged_mounts: false,
            },
        }
    }

    /// Configuracion compatible con desktop (algunos toggles relajados).
    /// deny_new_usb = false para que el USB funcione normalmente.
    pub fn desktop() -> Self {
        let mut cfg = Self::default();
        cfg.usb.deny_new_usb = false;
        cfg.perf.paranoid = 2; // permite profiling basico
        cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_hardened() {
        let cfg = HardeningConfig::default();
        assert_eq!(cfg.fs.symlinks, 1);
        assert_eq!(cfg.fs.hardlinks, 1);
        assert_eq!(cfg.fs.fifos, 2);
        assert_eq!(cfg.fs.regular, 2);
        assert!(cfg.aslr.kstack_offset);
        assert!(cfg.tty.tiocsti_restrict);
        assert!(!cfg.tty.legacy_tiocsti);
        assert!(!cfg.usb.deny_new_usb);
        assert_eq!(cfg.perf.paranoid, 3);
        assert!(!cfg.overlayfs.unprivileged_mounts);
    }

    #[test]
    fn desktop_relaxes_usb_and_perf() {
        let cfg = HardeningConfig::desktop();
        assert!(!cfg.usb.deny_new_usb);
        assert_eq!(cfg.perf.paranoid, 2);
        // El resto sigue hardened
        assert!(cfg.tty.tiocsti_restrict);
        assert!(!cfg.overlayfs.unprivileged_mounts);
    }

    #[test]
    fn maximum_locks_usb() {
        let cfg = HardeningConfig::maximum();
        assert!(cfg.usb.deny_new_usb);
        assert_eq!(cfg.perf.paranoid, 3);
    }
}
