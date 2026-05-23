#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# build-initramfs.sh — Construye un initramfs minimo para testing en QEMU
#
# Usa busybox como base. Para produccion se usa mkinitcpio con el preset
# de linux-rustlux.
#
# Uso:
#   ./scripts/build-initramfs.sh           — build con busybox del sistema
#   ./scripts/build-initramfs.sh /path/to/busybox — build con busybox especifico

set -euo pipefail

BUSYBOX="${1:-$(which busybox 2>/dev/null || echo "")}"
BUILD_DIR="build/initramfs"
OUTPUT="build/initramfs.cpio.gz"

if [[ -z "${BUSYBOX}" || ! -x "${BUSYBOX}" ]]; then
    echo "ERROR: busybox no encontrado."
    echo "Instala busybox o pasa la ruta: $0 /path/to/busybox"
    exit 1
fi

echo "==> Construyendo initramfs con busybox: ${BUSYBOX}"

# Limpiar y crear estructura
rm -rf "${BUILD_DIR}"
mkdir -p "${BUILD_DIR}"/{bin,sbin,dev,proc,sys,etc,usr/bin,usr/sbin,tmp,run}

# Copiar busybox
cp "${BUSYBOX}" "${BUILD_DIR}/bin/busybox"
chmod 755 "${BUILD_DIR}/bin/busybox"

# Crear init script (compatible con s6 en el futuro)
cat > "${BUILD_DIR}/init" << 'INIT_EOF'
#!/bin/busybox sh
# Rustlux minimal init — para testing en QEMU
# En produccion se usa s6 como PID 1

/bin/busybox mkdir -p /dev /proc /sys /tmp /run
/bin/busybox mount -t proc proc /proc
/bin/busybox mount -t sysfs sys /sys
/bin/busybox mount -t devtmpfs dev /dev
/bin/busybox mount -t tmpfs tmpfs /tmp
/bin/busybox mount -t tmpfs tmpfs /run

# Instalar symlinks de busybox
/bin/busybox --install -s 2>/dev/null

# Configurar terminal
export TERM=linux
export PATH=/bin:/sbin:/usr/bin:/usr/sbin
export HOME=/root

# Reducir verbosidad del kernel
echo 4 > /proc/sys/kernel/printk

# Limpiar page cache contaminado (mitigacion Dirty Frag)
echo 3 > /proc/sys/vm/drop_caches 2>/dev/null || true

# Mostrar info del kernel
echo ""
echo "======================================"
echo "  Rustlux Kernel — Testing Shell"
echo "======================================"
echo ""
uname -a
echo ""
echo "Modules loaded: $(cat /proc/modules 2>/dev/null | wc -l)"
echo "Memory: $(free -m 2>/dev/null | grep Mem | awk '{print $2}')MB"
echo ""
echo "Type 'poweroff' to exit QEMU."
echo ""

# Shell interactivo
exec /bin/busybox setsid /bin/busybox cttyhack /bin/sh
INIT_EOF

chmod 755 "${BUILD_DIR}/init"

# Crear nodos de dispositivo minimos
# (devtmpfs los crea automaticamente, pero por si acaso)
mknod -m 600 "${BUILD_DIR}/dev/console" c 5 1 2>/dev/null || true
mknod -m 666 "${BUILD_DIR}/dev/null" c 1 3 2>/dev/null || true
mknod -m 666 "${BUILD_DIR}/dev/tty" c 5 0 2>/dev/null || true

# Generar cpio
echo "==> Generando ${OUTPUT}..."
mkdir -p "$(dirname "${OUTPUT}")"
(cd "${BUILD_DIR}" && find . | cpio -o -H newc --quiet | gzip -9) > "${OUTPUT}"

SIZE=$(du -h "${OUTPUT}" | cut -f1)
echo "==> Initramfs listo: ${OUTPUT} (${SIZE})"
echo ""
echo "Para probar en QEMU:"
echo "  make qemu"
