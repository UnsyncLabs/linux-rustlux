# Rustlux

A memory-safe Linux kernel with the BORE+EEVDF scheduler, aggressive hardening, and protection against page-cache write vulnerabilities.

Based on Linux 7.0.9. Compatible with Arch Linux, CachyOS, and Artix.

## Features

- **BORE Scheduler** — Burst-Oriented Response Enhancer over EEVDF. Penalizes CPU-hogging tasks so interactive apps (compositor, terminal, audio) stay responsive under load.
- **Page-Cache Write Protection** — Rust-based splice guard eliminates the entire class of Dirty Pipe/Copy Fail/Dirty Frag vulnerabilities. Uses Rust FFI when `CONFIG_RUST=y`, C inline fallback otherwise.
- **Hardened by Default** — Maximum ASLR, stack randomization, TIOCSTI restricted, overlayfs unprivileged disabled, perf restricted, BPF hardened, ptrace restricted.
- **Sysctl Hardening** — Ships `/usr/lib/sysctl.d/90-rustlux-hardening.conf` with secure defaults applied at boot.
- **Module Blacklist** — Dangerous modules (`esp4`, `esp6`, `rxrpc`, `algif_aead`, `dccp`, `sctp`, `rds`, `tipc`) blacklisted by default.
- **DKMS Compatible** — Existing C modules (NVIDIA, VirtualBox, ZFS) work via standard DKMS.
- **Rust in Kernel** — Security functions compiled as part of the kernel via `rust/kernel/rustlux.rs` (requires `CONFIG_RUST=y`).

## Prerequisites

Before building, ensure you have the Rust toolchain with kernel support:

```bash
# Option A: Using pacman (Arch/CachyOS)
sudo pacman -S rust rust-bindgen rust-src clang llvm lld

# Option B: Using rustup
rustup update
rustup component add rust-src
# Also install bindgen:
cargo install bindgen-cli
# And LLVM/Clang:
sudo pacman -S clang llvm lld
```

Verify Rust is ready for kernel compilation:

```bash
rustc --version    # needs 1.85+
bindgen --version  # needs 0.65+
```

Other build dependencies:

```bash
sudo pacman -S base-devel bc cpio gettext libelf pahole perl python tar xz zstd openssl
```

## Quick Start

```bash
# Clone
git clone https://github.com/UnsyncLabs/linux-rustlux.git
cd linux-rustlux

# Run Rust crate tests (standalone)
make rust-test

# Build the kernel manually
make kernel-fetch
make kernel-patch
make kernel-config
make release-build
```

## Install (Arch/CachyOS)

```bash
cd linux-rustlux/pkg/arch

# I use this command for my AMD PC
_use_llvm_lto=thin _processor_opt=zen4 _build_zfs=yes makepkg -si

# If you fail building, add the GPG keys:
# gpg --recv-keys XXXXXXXXXXXXXXXA

# If you need to retry building, delete src/ or patches may fail:
# rm -rf src/
```

Build options via environment variables:

```bash
# With Clang LTO (recommended for performance)
_use_llvm_lto=thin makepkg -si

# Optimized for AMD Zen4
_processor_opt=zen4 makepkg -si

# With ZFS module
_build_zfs=yes makepkg -si

# Combined (all options)
_use_llvm_lto=thin _processor_opt=zen4 _build_zfs=yes makepkg -si
```

## Install (Artix)

```bash
#untested
cd pkg/artix
makepkg -si
```

## Project Structure

```
├── rust/                   Rust crates (kernel components)
│   ├── rustlux_mm/         Page-cache protection, RoAfterInit, splice guard
│   ├── rustlux_sched/      BORE scheduler implementation
│   ├── rustlux_security/   Hardening config, capabilities, FFI
│   └── rustlux_bindings.h  C header for FFI declarations
├── patches/                Patches applied over Linux 7.0.9
│   ├── 0001-hardened.patch         Security hardening
│   ├── 0002-bore-cachy.patch       BORE scheduler
│   ├── 0003-dkms-clang.patch       DKMS compatibility
│   ├── 0004-rustlux-splice-guard.patch  Page-cache write protection
│   ├── 0005-rustlux-kernel-module.patch Rust FFI functions in kernel
│   ├── 0006-rustlux-perf-hook.patch     Rust perf_event access control
│   └── 0007-rustlux-tiocsti-hook.patch  Rust TIOCSTI injection prevention
├── config/                 Kernel config + modprobe + sysctl
├── modules/                Example kernel module (Rust + DKMS)
├── pkg/
│   ├── arch/               PKGBUILD for Arch/CachyOS (LTO, ZFS, march options)
│   └── artix/              PKGBUILD for Artix (simple, s6 focused)
├── scripts/                Build helpers (initramfs, module signing)
├── docs/                   Documentation
└── ideas/                  Design notes and future plans
```

## Vulnerabilities Mitigated

| CVE | Name | Mechanism | Status |
|---|---|---|---|
| CVE-2022-0847 | Dirty Pipe | pipe + splice page-cache write | Blocked by splice guard |
| CVE-2026-31431 | Copy Fail | AF_ALG + splice page-cache write | Blocked + module blacklisted |
| CVE-2026-43284 | Dirty Frag (xfrm) | xfrm-ESP page-cache write | Blocked + module blacklisted |
| CVE-2026-43500 | Dirty Frag (rxrpc) | RxRPC page-cache write | Blocked + module blacklisted |

## Post-Install Verification

```bash
# Verify BORE is active
cat /proc/sys/kernel/sched_bore  # should be 1

# Verify Rust functions are in kernel (requires CONFIG_RUST=y)
grep rustlux /proc/kallsyms

# Verify hardening sysctls
cat /proc/sys/kernel/perf_event_paranoid  # should be 3
cat /proc/sys/kernel/yama/ptrace_scope    # should be 2
cat /proc/sys/kernel/unprivileged_bpf_disabled  # should be 1

# Verify blacklisted modules are not loaded
lsmod | grep -E "esp4|esp6|rxrpc|algif_aead"  # should be empty
```

## Toolchain Requirements

- Rust 1.85+ with `rust-src` component
- rust-bindgen 0.65+
- Clang 18+ / LLVM (for LTO builds)
- GCC 14+ (for non-LTO builds)
- pahole (for BTF, disabled with LTO+Rust)

## Note on CONFIG_RUST + LTO

Linux 7.0.9 has a Kconfig constraint: `CONFIG_RUST` cannot coexist with `DEBUG_INFO_BTF` when LTO is enabled. The PKGBUILD automatically disables BTF when building with LTO to allow Rust compilation. This means `bpftool` BTF features won't be available in LTO+Rust builds.

## License

- Kernel (patches, config): GPL-2.0-only
- Rust components (rust/rustlux_*): GPL-2.0-only
s
## Credits

- Linux kernel: https://kernel.org
- BORE Scheduler: Masahito Suzuki (firelzrd)
- linux-hardened: anthraxx
- CachyOS patches: CachyOS team (ptr1337, Piotr Gorski)
