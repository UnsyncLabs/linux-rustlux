// SPDX-License-Identifier: GPL-2.0-only
//
// rustlux_example — Modulo de ejemplo para Rustlux
//
// Este modulo demuestra como escribir un modulo del kernel en Rust
// usando la infraestructura de rust-for-linux que Rustlux hereda
// de Linux
//
// Para compilar como modulo del kernel:
//   make -C /lib/modules/$(uname -r)/build M=$(pwd) modules
//
// Para cargar:
//   insmod rustlux_example.ko
//
// Para hacer unload:
//   rmmod rustlux_example
//
// En dmesg veras:
//   [rustlux_example] Hello from Rustlux! kernel module loaded.
//   [rustlux_example] Goodbye from Rustlux! Module unloaded.

// En el kernel real, esto usa las macros de rust-for-linux
//
// use kernel::prelude::*;
//
// module! {
//     type: RustluxExample,
//     name: "rustlux_example",
//     author: "Rustlux Project",
//     description: "Example Rustlux kernel module",
//     license: "GPL",
// }
//
// struct RustluxExample;
//
// impl kernel::Module for RustluxExample {
//     fn init(_module: &'static ThisModule) -> Result<Self> {
//         pr_info!("Hello from Rustlux! kernel module loaded.\n");
//         Ok(RustluxExample)
//     }
// }
//
// impl Drop for RustluxExample {
//     fn drop(&mut self) {
//         pr_info!("Goodbye from Rustlux! Module unloaded.\n");
//     }
// }

// Para testing standalone (fuera del kernel).
#![no_std]

/// Version del modulo de ejemplo.
pub const VERSION: &str = "0.1.0";

/// Nombre del modulo.
pub const NAME: &str = "rustlux_example";

/// funcion de inicializacion del modulo (logica pura).
pub fn init() -> Result<(), &'static str> {
    // En el kernel real, aqui iria:
    // - Registrar un dispositivo
    // - Crear entradas en /proc o /sys
    // - Registrar un driver
    Ok(())
}

/// funcion de limpieza del modulo (logica pura).
pub fn cleanup() {
    // En el kernel real, aqui iria:
    // - Desregistrar dispositivos
    // - Liberar recursos
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_init_succeeds() {
        assert!(init().is_ok());
    }

    #[test]
    fn module_has_name() {
        assert_eq!(NAME, "rustlux_example");
    }
}
