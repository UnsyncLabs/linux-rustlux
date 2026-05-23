#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# module-signing.sh — Genera claves para firma de módulos del kernel
#
# Rustlux usa MODULE_SIG=y por defecto (del patch hardened).
# Los módulos deben estar firmados para cargarse.
#
# Para módulos DKMS (NVIDIA, etc.), MODULE_SIG_FORCE no está activo,
# así que módulos sin firmar cargan con un warning pero no se bloquean.
#
# Uso:
#   ./scripts/module-signing.sh generate   — generar par de claves
#   ./scripts/module-signing.sh sign <mod> — firmar un módulo .ko
#   ./scripts/module-signing.sh verify <mod> — verificar firma

set -euo pipefail

KERNEL_BUILD="${KERNEL_BUILD:-/usr/lib/modules/$(uname -r)/build}"
SIGN_FILE="${KERNEL_BUILD}/scripts/sign-file"
HASH_ALGO="sha512"
KEY_DIR="${HOME}/.config/rustlux/module-signing"
PRIVATE_KEY="${KEY_DIR}/signing_key.pem"
X509_CERT="${KEY_DIR}/signing_key.x509"

generate_keys() {
    echo "==> Generando claves de firma de módulos..."
    mkdir -p "${KEY_DIR}"
    chmod 700 "${KEY_DIR}"

    # Generar clave privada RSA 4096
    openssl req -new -nodes -utf8 \
        -sha512 -days 36500 \
        -batch -x509 \
        -outform PEM \
        -out "${X509_CERT}" \
        -keyout "${PRIVATE_KEY}" \
        -subj "/CN=Rustlux Module Signing Key/O=Rustlux Project/" \
        2>/dev/null

    chmod 600 "${PRIVATE_KEY}"
    echo "==> Claves generadas en ${KEY_DIR}"
    echo "    Privada: ${PRIVATE_KEY}"
    echo "    Cert:    ${X509_CERT}"
}

sign_module() {
    local module="$1"
    if [[ ! -f "${module}" ]]; then
        echo "ERROR: Módulo no encontrado: ${module}"
        exit 1
    fi
    if [[ ! -f "${PRIVATE_KEY}" ]]; then
        echo "ERROR: Clave privada no encontrada. Corre: $0 generate"
        exit 1
    fi

    echo "==> Firmando ${module}..."
    "${SIGN_FILE}" "${HASH_ALGO}" "${PRIVATE_KEY}" "${X509_CERT}" "${module}"
    echo "    Firmado OK."
}

verify_module() {
    local module="$1"
    if [[ ! -f "${module}" ]]; then
        echo "ERROR: Módulo no encontrado: ${module}"
        exit 1
    fi

    # Verificar si tiene firma (los últimos bytes son el magic de firma)
    if tail -c 28 "${module}" | grep -q "~Module signature appended~"; then
        echo "✅ ${module}: firmado"
    else
        echo "⚠️  ${module}: sin firma (cargará con warning si MODULE_SIG_FORCE=n)"
    fi
}

case "${1:-help}" in
    generate)
        generate_keys
        ;;
    sign)
        sign_module "${2:-}"
        ;;
    verify)
        verify_module "${2:-}"
        ;;
    *)
        echo "Uso: $0 {generate|sign <module.ko>|verify <module.ko>}"
        echo ""
        echo "  generate  — Generar par de claves para firma de módulos"
        echo "  sign      — Firmar un módulo .ko"
        echo "  verify    — Verificar si un módulo está firmado"
        exit 1
        ;;
esac
