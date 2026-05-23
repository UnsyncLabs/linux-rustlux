// SPDX-License-Identifier: GPL-2.0-only
// Copyright (C) 2026 Yadiel Abner Rodriguez Jorge (neokuze) <imneokuze@gmail.com>
//
// hardening_ffi.rs — FFI para verificaciones de seguridad desde C
//
// Exporta funciones que el kernel C llama para verificar permisos
// de hardening (perf_event, TIOCSTI, USB, etc.)

use crate::capabilities::{Cap, CapSet, perf_allowed, tiocsti_allowed};
use crate::hardening::HardeningConfig;

/// Configuracion global de hardening — inicializada una vez al boot.
/// En el kernel real, estos valores vienen de los sysctl.
/// Aqui usamos los defaults hardened.
static HARDENING: HardeningConfig = HardeningConfig {
    fs: crate::hardening::FsProtection {
        symlinks: 1,
        hardlinks: 1,
        fifos: 2,
        regular: 2,
    },
    aslr: crate::hardening::AslrConfig {
        mmap_rnd_bits: 32,
        mmap_rnd_compat_bits: 16,
        kstack_offset: true,
    },
    tty: crate::hardening::TtyConfig {
        tiocsti_restrict: true,
        legacy_tiocsti: false,
    },
    usb: crate::hardening::UsbConfig {
        deny_new_usb: false,
    },
    perf: crate::hardening::PerfConfig {
        paranoid: 3,
    },
    overlayfs: crate::hardening::OverlayFsConfig {
        unprivileged_mounts: false,
    },
};

/// Verifica si perf_event esta permitido para un conjunto de capabilities.
///
/// Llamada desde kernel/events/core.c cuando un usuario intenta usar perf.
///
/// # Parametros
/// - `cap_effective`: bitmask de capabilities efectivas del proceso
/// - `paranoid`: valor actual de kernel.perf_event_paranoid
///
/// # Retorno
/// - 1 si permitido, 0 si denegado
#[no_mangle]
pub extern "C" fn rustlux_perf_event_allowed(
    cap_effective: u64,
    paranoid: i32,
) -> u8 {
    let caps = CapSet::from_raw(cap_effective);
    perf_allowed(caps, paranoid) as u8
}

/// Verifica si TIOCSTI esta permitido.
///
/// Llamada desde drivers/tty/tty_io.c en tiocsti().
///
/// # Parametros
/// - `cap_effective`: bitmask de capabilities efectivas
/// - `same_tty`: 1 si el proceso esta en la misma TTY, 0 si no
///
/// # Retorno
/// - 1 si permitido, 0 si denegado
#[no_mangle]
pub extern "C" fn rustlux_tiocsti_allowed(
    cap_effective: u64,
    same_tty: u8,
) -> u8 {
    let caps = CapSet::from_raw(cap_effective);
    let restrict = HARDENING.tty.tiocsti_restrict;
    tiocsti_allowed(caps, restrict, same_tty != 0) as u8
}

/// Verifica si se permite conectar un nuevo dispositivo USB.
///
/// Llamada desde drivers/usb/core/hub.c en hub_port_connect().
///
/// # Retorno
/// - 1 si permitido (deny_new_usb = false), 0 si denegado
#[no_mangle]
pub extern "C" fn rustlux_usb_new_device_allowed() -> u8 {
    (!HARDENING.usb.deny_new_usb) as u8
}

/// Retorna el nivel de paranoia de perf_event configurado.
///
/// Llamada desde kernel/events/core.c para obtener el default.
#[no_mangle]
pub extern "C" fn rustlux_perf_paranoid_level() -> i32 {
    HARDENING.perf.paranoid
}

/// Retorna el nivel de proteccion de symlinks.
///
/// Llamada desde fs/namei.c.
#[no_mangle]
pub extern "C" fn rustlux_protected_symlinks() -> u8 {
    HARDENING.fs.symlinks
}

/// Retorna el nivel de proteccion de hardlinks.
#[no_mangle]
pub extern "C" fn rustlux_protected_hardlinks() -> u8 {
    HARDENING.fs.hardlinks
}

/// Retorna el nivel de proteccion de FIFOs.
#[no_mangle]
pub extern "C" fn rustlux_protected_fifos() -> u8 {
    HARDENING.fs.fifos
}

/// Retorna el nivel de proteccion de archivos regulares.
#[no_mangle]
pub extern "C" fn rustlux_protected_regular() -> u8 {
    HARDENING.fs.regular
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_denied_without_cap() {
        // Sin CAP_PERFMON, paranoid=3 → denegado
        assert_eq!(rustlux_perf_event_allowed(0, 3), 0);
    }

    #[test]
    fn perf_allowed_with_cap() {
        // Con CAP_PERFMON (bit 38), paranoid=3 → permitido
        let cap_perfmon = 1u64 << (Cap::PerfMon as u64);
        assert_eq!(rustlux_perf_event_allowed(cap_perfmon, 3), 1);
    }

    #[test]
    fn tiocsti_denied_without_sysadmin() {
        // tiocsti_restrict=true, sin CAP_SYS_ADMIN → denegado
        assert_eq!(rustlux_tiocsti_allowed(0, 1), 0);
    }

    #[test]
    fn usb_allowed_by_default() {
        // deny_new_usb = false por defecto → permitido
        assert_eq!(rustlux_usb_new_device_allowed(), 1);
    }

    #[test]
    fn hardened_defaults() {
        assert_eq!(rustlux_perf_paranoid_level(), 3);
        assert_eq!(rustlux_protected_symlinks(), 1);
        assert_eq!(rustlux_protected_hardlinks(), 1);
        assert_eq!(rustlux_protected_fifos(), 2);
        assert_eq!(rustlux_protected_regular(), 2);
    }
}
