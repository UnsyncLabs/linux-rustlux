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
- **Memory Sentinel** — Detects unstable DRAM (aggressive EXPO/XMP profiles) by tracking memory-controller machine checks, warns with actionable guidance and contains uncorrected errors via hwpoison.
- **Soft-ECC** — Software SECDED (Hamming 72,64) scrubbing of critical kernel structures (`__ro_after_init`, IDT): single-bit DRAM flips are corrected in place, multi-bit corruption is reported. Also mitigates rowhammer-induced flips in kernel data.
- **Bigscreen Beyond / Beyond 2** — VR headset display patches applied by default (disable with `_bigscreen_beyond=no`).

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

# I use this command for my AMD PC for gaming., hardened can give problems with proton.
_hardened=no _processor_opt=zen4 makepkg -si

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

# Without the hardened patch + hardening config (default: on)
_hardened=no makepkg -si

# Without the Bigscreen Beyond / Beyond 2 VR patches (default: on)
_bigscreen_beyond=no makepkg -si

# Combined (all options)
_use_llvm_lto=thin _processor_opt=zen4 _build_zfs=yes makepkg -si
```

## Bigscreen Beyond / Beyond 2

The kernel ships with the patches needed for the Bigscreen Beyond and
Beyond 2 VR headsets (applied by default, disable with
`_bigscreen_beyond=no` on makepkg or `RUSTLUX_BSB=no` on make):

- `0010-bsb-beyond-display.patch` — DRM/EDID series by Yaroslav Bolyukin
  (parses the VESA DSC bits-per-pixel target from the headset EDID and
  uses it in amdgpu) so the display lights up correctly.
- `0011-bsb-amd-dsc-fix.patch` — corrects `max_qp` limits in the amdgpu
  DSC RC parameter tables (VESA DSC 1.1 Table E-5), fixing the "rainbow
  static" artifacts in complex geometry.

**NVIDIA users**: the DSC fix for NVIDIA lives in
`patches/nvidia/nvidia-bsb-dsc-fix.patch` and applies to the NVIDIA
**open kernel modules** source tree (580+, e.g. the DKMS sources at
`/usr/src/nvidia-open-*`), not to this kernel. Eye-tracking camera
support needs a separate UVC patch — see
https://wiki.vronlinux.org/docs/hardware/bigscreen-beyond

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
│   ├── 0007-rustlux-tiocsti-hook.patch  Rust TIOCSTI injection prevention
│   ├── 0008-rustlux-mem-sentinel.patch  DRAM instability detector
│   ├── 0009-rustlux-soft-ecc.patch      Software SECDED for critical structures
│   ├── 0010-bsb-beyond-display.patch    Bigscreen Beyond/Beyond 2 display (DRM/EDID, optional)
│   ├── 0011-bsb-amd-dsc-fix.patch       Bigscreen Beyond AMD DSC fix (optional)
│   └── nvidia/nvidia-bsb-dsc-fix.patch  BSB DSC fix for NVIDIA open modules (out-of-tree)
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

# Verify memory sentinel and soft-ECC are active
sysctl vm.rustlux_memsentinel      # should be 1
sysctl vm.rustlux_softecc          # should be 1
dmesg | grep rustlux_softecc       # should show "protecting ro_after_init" and "protecting idt"
```

## Memory Error Monitoring

Rustlux watches your DRAM at runtime. This matters on consumer hardware
(no host-visible ECC) running high-frequency EXPO/XMP profiles: an
unstable profile produces silent bit flips that normally only surface as
random crashes. The kernel cannot change the memory frequency (firmware
programs it before boot) — what it does is **detect, warn and contain**.

### Watching for errors

```bash
# Live view of everything Rustlux reports
sudo dmesg -w | grep -E "rustlux_memsentinel|rustlux_softecc"

# Everything from the current boot
journalctl -k -b | grep rustlux

# Check whether the kernel was tainted by a memory event (bit 4 = machine check)
cat /proc/sys/kernel/tainted
```

### What the messages mean

**Memory Sentinel** (machine checks from the memory controller):

```
rustlux_memsentinel: 8 corrected memory errors within 60s — DRAM is likely UNSTABLE at the current frequency.
```
Your RAM is producing real, hardware-corrected errors. The kernel keeps
running, but this is the early warning before silent corruption or a
crash. Suggested actions (in order): lower the EXPO/XMP frequency one
step, raise VSOC/VDDQ within safe limits, update BIOS/AGESA, run
memtest86+ overnight. Uncorrected errors are handed to the kernel's
hwpoison machinery: the affected page is unmapped and the owning process
killed instead of letting corrupted data spread.

**Soft-ECC** (scrubber over critical kernel structures):

```
rustlux_softecc: corrected single-bit flip in idt at ffffffff9a405000 (phys 0x405000) — DRAM produced a real error, check EXPO/XMP stability
```
A bit flipped inside a protected structure and was repaired in place.
One of these is hard evidence of memory instability (or rowhammer) — the
physical address tells you where, useful to cross-check with memtest86+.

```
rustlux_softecc: UNCORRECTABLE multi-bit corruption in ro_after_init at ...
```
Two or more bits flipped in the same 64-bit word. The kernel cannot
repair this and taints itself: treat the machine as unreliable, reboot,
and fix the memory configuration before trusting it again.

### Tuning

```bash
# Disable/enable at runtime (default: enabled)
sudo sysctl vm.rustlux_memsentinel=0
sudo sysctl vm.rustlux_softecc=0

# Corrected errors per 60s window before the sentinel warns (default: 8)
sudo sysctl vm.rustlux_memsentinel_threshold=4   # stricter, warn earlier

# Soft-ECC scrub interval in milliseconds (default: 2000, min: 100)
sudo sysctl vm.rustlux_softecc_interval_ms=500   # scrub more often

# Persist across reboots
echo "vm.rustlux_memsentinel_threshold = 4" | sudo tee /etc/sysctl.d/99-rustlux-mem.conf
```

### Testing your RAM at high frequency

1. Set your EXPO/XMP profile (e.g. 6000 MT/s) in the BIOS.
2. Boot Rustlux and put the machine under memory load (compile something
   big, run a game) while watching `sudo dmesg -w | grep rustlux`.
3. No messages after hours of load and idle = your profile is stable at
   the kernel's eye level. Sentinel warnings = the profile is marginal;
   act on them before trusting the machine with real data.

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
