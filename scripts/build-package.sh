#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# build-package.sh — Genera el paquete pacman de linux-rustlux
#
# Uso:
#   ./scripts/build-package.sh          — build completo
#   ./scripts/build-package.sh --quick  — skip tests, solo build
#
# Requisitos:
#   pacman -S base-devel bc cpio gettext libelf pahole perl python \
#             tar xz zstd rust rust-bindgen gcc clang llvm

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "${SCRIPT_DIR}")"
PKG_DIR="${PROJECT_DIR}/pkg"

QUICK=0
if [[ "${1:-}" == "--quick" ]]; then
    QUICK=1
fi

echo "╔══════════════════════════════════════════╗"
echo "║     Rustlux Kernel — Package Builder    ║"
echo "╚══════════════════════════════════════════╝"
echo ""

# Paso 1: Tests de Rust
if [[ ${QUICK} -eq 0 ]]; then
    echo "==> [1/4] Corriendo tests de Rust..."
    (cd "${PROJECT_DIR}" && cargo test --workspace --quiet)
    echo "    ✅ Todos los tests pasan"
    echo ""
fi

# Paso 2: Preparar patches en pkg/
echo "==> [2/4] Preparando patches..."
cp "${PROJECT_DIR}/patches/0001-hardened.patch" "${PKG_DIR}/"
cp "${PROJECT_DIR}/patches/0002-bore-cachy.patch" "${PKG_DIR}/"
cp "${PROJECT_DIR}/patches/0003-dkms-clang.patch" "${PKG_DIR}/"
cp "${PROJECT_DIR}/config/rustlux_desktop.config" "${PKG_DIR}/"
echo "    ✅ Patches y config copiados a pkg/"
echo ""

# Paso 3: Build del paquete
echo "==> [3/4] Compilando paquete pacman..."
echo "    Esto puede tardar 30-60 minutos dependiendo del hardware."
echo ""
(cd "${PKG_DIR}" && makepkg -sf --noconfirm)
echo ""
echo "    ✅ Paquete compilado"
echo ""

# Paso 4: Mostrar resultado
echo "==> [4/4] Paquetes generados:"
ls -lh "${PKG_DIR}"/*.pkg.tar.zst 2>/dev/null || echo "    (no se encontraron paquetes .pkg.tar.zst)"
echo ""
echo "Para instalar:"
echo "  sudo pacman -U pkg/linux-rustlux-*.pkg.tar.zst"
echo ""
echo "Para instalar headers (necesario para DKMS):"
echo "  sudo pacman -U pkg/linux-rustlux-headers-*.pkg.tar.zst"
