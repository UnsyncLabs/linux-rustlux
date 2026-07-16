#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# secureboot_sign.sh — Firma el kernel Rustlux instalado con TUS llaves de Secure Boot
#
# Por qué existe: los paquetes linux-rustlux traen el vmlinuz SIN FIRMAR. Con
# Secure Boot activo el firmware rechaza una imagen sin firma, así que el kernel
# no arranca. Este script lo firma con tus propias llaves usando sbctl.
#
# Por qué con TUS llaves y no con las del repo: así no dependés de UnsyncLabs
# para arrancar tu máquina, y no le delegás a nadie el permiso de firmar algo
# que tu firmware vaya a aceptar. La llave privada del repo nunca sale del
# firmador, y no queremos que tenga que salir.
#
# sbctl instala un hook de pacman (zz-sbctl.hook) que vuelve a firmar solo
# después de cada actualización del kernel, así que esto se corre una vez.
#
# Uso:
#   ./scripts/secureboot_sign.sh           — firma los kernels rustlux instalados
#   ./scripts/secureboot_sign.sh --status  — solo muestra el estado, no toca nada

set -euo pipefail

die() { echo "error: $*" >&2; exit 1; }

show_status() {
    echo "== Estado de Secure Boot =="
    sbctl status || true
    echo
    echo "== Kernels rustlux instalados =="
    local found=0
    for k in /usr/lib/modules/*/vmlinuz; do
        [ -e "$k" ] || continue
        case "$k" in *rustlux*) ;; *) continue ;; esac
        found=1
        if sbctl verify 2>/dev/null | grep -qF "$k"; then
            echo "  $k"
        else
            echo "  $k"
        fi
    done
    [ "$found" -eq 1 ] || echo "  (ninguno)"
    echo
    echo "== Archivos que sbctl tiene registrados =="
    sbctl list-files 2>/dev/null || echo "  (ninguno)"
}

main() {
    command -v sbctl >/dev/null || die "falta sbctl (sudo pacman -S sbctl)"

    if [ "${1:-}" = "--status" ]; then
        show_status
        exit 0
    fi

    [ "$(id -u)" -eq 0 ] || die "hay que correr esto como root"

    # Sin Secure Boot no hay nada que firmar: el firmware no valida la imagen.
    if ! sbctl status 2>/dev/null | grep -qi 'Secure Boot.*enabled\|Setup Mode.*enabled'; then
        echo "Secure Boot parece estar desactivado y el firmware no está en Setup Mode."
        echo "En ese caso no hace falta firmar nada: el kernel arranca igual."
        echo "Si querés activar Secure Boot, entrá al firmware, borrá las llaves"
        echo "de fábrica para entrar en Setup Mode, y volvé a correr esto."
        exit 0
    fi

    # Las llaves propias son requisito. Crearlas no toca el firmware; enrolarlas sí.
    if ! sbctl status 2>/dev/null | grep -qi 'Installed.*sbctl is installed'; then
        cat <<'EOF'

No tenés llaves de sbctl creadas todavía. Hacen falta dos pasos:

    sbctl create-keys
    sbctl enroll-keys --microsoft

ATENCIÓN con enroll-keys — esto reemplaza las llaves de Secure Boot del
firmware por las tuyas:

  * El flag --microsoft conserva las llaves de Microsoft. Sin él, cualquier
    cosa firmada solo por Microsoft deja de arrancar: eso incluye las option
    ROM de muchas placas de video y de red, y Windows si tenés dual boot.
    En algunas máquinas eso significa no llegar ni al bootloader.
  * Sabé cómo volver atrás antes de correrlo: en el firmware, restaurar las
    llaves de fábrica ("Restore Factory Keys") y/o desactivar Secure Boot.
  * Hay firmwares con implementaciones rotas donde esto deja la placa
    inutilizable. Buscá tu modelo antes si no estás seguro.

Corré esos dos comandos vos mismo, con eso entendido, y volvé a correr este
script para firmar el kernel.

EOF
        exit 1
    fi

    local signed=0
    for k in /usr/lib/modules/*/vmlinuz; do
        [ -e "$k" ] || continue
        case "$k" in *rustlux*) ;; *) continue ;; esac
        echo "==> Firmando $k"
        # -s registra el archivo en la base de sbctl, así el hook de pacman
        # (zz-sbctl.hook) lo vuelve a firmar solo en cada update del kernel.
        sbctl sign -s "$k"
        signed=$((signed + 1))
    done

    if [ "$signed" -eq 0 ]; then
        die "no encontré ningún vmlinuz de rustlux en /usr/lib/modules/*/"
    fi

    # El bootloader también tiene que estar firmado o no se llega al kernel.
    for b in /boot/EFI/BOOT/BOOTX64.EFI /boot/EFI/systemd/systemd-bootx64.efi \
             /boot/EFI/GRUB/grubx64.efi /boot/vmlinuz-linux; do
        [ -e "$b" ] || continue
        echo "==> Firmando $b"
        sbctl sign -s "$b" || echo "    (no se pudo, seguí a mano si hace falta)"
    done

    echo
    echo "Listo: $signed kernel(s) rustlux firmado(s)."
    echo "Verificá que no quede nada sin firmar antes de reiniciar:"
    echo "    sbctl verify"
    echo
    echo "Si sbctl verify muestra algo importante sin firmar, firmalo con"
    echo "'sbctl sign -s <ruta>' — si reiniciás con el bootloader sin firmar,"
    echo "la máquina no va a bootear con Secure Boot activo."
}

main "$@"
