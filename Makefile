# SPDX-License-Identifier: GPL-2.0-only
# Rustlux Kernel — Makefile principal
#
# Uso:
#   make help          — mostrar targets disponibles
#   make rust-test     — correr tests de los crates Rust
#   make rust-check    — verificar compilación sin generar binarios
#   make dev-build     — compilar kernel con CCC (desarrollo/QEMU)
#   make release-build — compilar kernel con GCC (producción)
#   make qemu          — arrancar en QEMU x86_64 con initramfs mínimo

# ── Versión del kernel base ──────────────────────────────────────────────────
KERNEL_VERSION  := 7.0.9
KERNEL_NAME     := linux-$(KERNEL_VERSION)
KERNEL_URL      := https://cdn.kernel.org/pub/linux/kernel/v7.x/$(KERNEL_NAME).tar.xz

# ── Compiladores ─────────────────────────────────────────────────────────────
# Producción: GCC o Clang (validados para el kernel)
CC              ?= gcc
HOSTCC          ?= gcc

# Desarrollo: CCC (Claude's C Compiler) — para QEMU/testing
CCC_DIR         := tools/ccc
CCC             := $(CCC_DIR)/target/release/ccc-x86

# ── Arquitectura ─────────────────────────────────────────────────────────────
ARCH            := x86_64
CROSS_COMPILE   :=

# ── Paths ────────────────────────────────────────────────────────────────────
KERNEL_DIR      := kernel/$(KERNEL_NAME)
BUILD_DIR       := build
PATCHES_DIR     := patches

# ── Targets principales ──────────────────────────────────────────────────────

.PHONY: help rust-test rust-check rust-clippy \
        kernel-fetch kernel-patch kernel-config \
        dev-build release-build \
        ccc-build qemu clean

help:
	@echo ""
	@echo "  Rustlux Kernel $(KERNEL_VERSION) — Targets disponibles"
	@echo ""
	@echo "  Rust:"
	@echo "    rust-test      Correr tests de todos los crates Rust"
	@echo "    rust-check     Verificar compilación (sin generar binarios)"
	@echo "    rust-clippy    Linter Rust"
	@echo ""
	@echo "  Kernel:"
	@echo "    kernel-fetch   Descargar Linux $(KERNEL_VERSION)"
	@echo "    kernel-patch   Aplicar patches (hardened + BORE)"
	@echo "    kernel-config  Generar .config base para Rustlux"
	@echo "    dev-build      Compilar con CCC (desarrollo/QEMU)"
	@echo "    release-build  Compilar con GCC (producción)"
	@echo ""
	@echo "  Herramientas:"
	@echo "    ccc-build      Compilar Claude's C Compiler"
	@echo "    qemu           Arrancar en QEMU x86_64"
	@echo "    clean          Limpiar artefactos de build"
	@echo ""

# ── Tests y verificación Rust ────────────────────────────────────────────────

rust-test:
	@echo "==> Corriendo tests de crates Rust..."
	cargo test --workspace

rust-check:
	@echo "==> Verificando compilación Rust..."
	cargo check --workspace

rust-clippy:
	@echo "==> Corriendo clippy..."
	cargo clippy --workspace -- -D warnings

# ── Kernel: fetch, patch, config ─────────────────────────────────────────────

kernel-fetch:
	@echo "==> Descargando Linux $(KERNEL_VERSION)..."
	mkdir -p kernel
	wget -c $(KERNEL_URL) -O kernel/$(KERNEL_NAME).tar.xz
	@echo "==> Extrayendo..."
	tar -xf kernel/$(KERNEL_NAME).tar.xz -C kernel/
	@echo "==> Listo: $(KERNEL_DIR)"

kernel-patch: $(KERNEL_DIR)
	@echo "==> Aplicando patch hardened..."
	cd $(KERNEL_DIR) && patch --forward -p1 < ../../$(PATCHES_DIR)/0001-hardened.patch || true
	@echo "==> Aplicando patch BORE..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0002-bore-cachy.patch
	@echo "==> Aplicando patch dkms-clang..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0003-dkms-clang.patch
	@echo "==> Aplicando patch rustlux splice guard..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0004-rustlux-splice-guard.patch
	@echo "==> Aplicando patch rustlux kernel module (Rust FFI)..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0005-rustlux-kernel-module.patch
	@echo "==> Aplicando patch rustlux perf hook..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0006-rustlux-perf-hook.patch
	@echo "==> Aplicando patch rustlux tiocsti hook..."
	cd $(KERNEL_DIR) && patch -p1 < ../../$(PATCHES_DIR)/0007-rustlux-tiocsti-hook.patch
	@echo "==> Patches aplicados."

kernel-config: $(KERNEL_DIR)
	@echo "==> Generando config base de Rustlux..."
	cp config/rustlux_desktop.config $(KERNEL_DIR)/.config
	cd $(KERNEL_DIR) && make ARCH=$(ARCH) olddefconfig
	@echo "==> Config lista en $(KERNEL_DIR)/.config"

# ── Builds del kernel ────────────────────────────────────────────────────────

dev-build: $(KERNEL_DIR) $(CCC)
	@echo "==> Build de desarrollo con CCC (Claude's C Compiler)..."
	cd $(KERNEL_DIR) && make \
		ARCH=$(ARCH) \
		CC=$(abspath $(CCC)) \
		HOSTCC=$(abspath $(CCC)) \
		-j$$(nproc)
	@echo "==> Kernel listo: $(KERNEL_DIR)/arch/x86/boot/bzImage"

release-build: $(KERNEL_DIR)
	@echo "==> Build de producción con GCC..."
	cd $(KERNEL_DIR) && make \
		ARCH=$(ARCH) \
		CC=$(CC) \
		HOSTCC=$(HOSTCC) \
		-j$$(nproc)
	@echo "==> Kernel listo: $(KERNEL_DIR)/arch/x86/boot/bzImage"

# ── CCC (Claude's C Compiler) ────────────────────────────────────────────────

ccc-build:
	@echo "==> Clonando CCC..."
	mkdir -p tools
	[ -d $(CCC_DIR) ] || git clone https://github.com/anthropics/claudes-c-compiler $(CCC_DIR)
	@echo "==> Compilando CCC..."
	cd $(CCC_DIR) && cargo build --release
	@echo "==> CCC listo: $(CCC)"

$(CCC):
	$(MAKE) ccc-build

# ── QEMU ─────────────────────────────────────────────────────────────────────

qemu: $(KERNEL_DIR)/arch/x86/boot/bzImage
	@echo "==> Arrancando Rustlux en QEMU x86_64..."
	qemu-system-x86_64 \
		-M q35 \
		-m 512M \
		-kernel $(KERNEL_DIR)/arch/x86/boot/bzImage \
		-initrd build/initramfs.cpio.gz \
		-append "console=ttyS0 nokaslr quiet" \
		-nographic \
		-no-reboot \
		-serial mon:stdio

# ── Limpieza ─────────────────────────────────────────────────────────────────

clean:
	cargo clean
	[ -d $(KERNEL_DIR) ] && cd $(KERNEL_DIR) && make clean || true
	rm -rf $(BUILD_DIR)

$(KERNEL_DIR):
	@echo "ERROR: Kernel no encontrado. Corre 'make kernel-fetch' primero."
	@exit 1
