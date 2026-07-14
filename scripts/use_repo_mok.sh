#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# use_repo_mok.sh — Enrola el certificado del repo UnsyncLabs como MOK
#
# Solo hace falta si usás Secure Boot. Sin Secure Boot no tiene sentido:
# el firmware no valida nada y el keyring .machine del kernel queda vacío,
# así que mokutil no cambia nada.
#
# Qué resuelve: con Secure Boot activo, shim solo deja cargar módulos firmados
# por una llave que conoce. Los kernels linux-rustlux ya vienen con el cert de
# UnsyncLabs horneado en .builtin_trusted_keys, pero eso cubre la carga de
# módulos, no la validación de la imagen del kernel por parte de shim. Enrolar
# la MOK cierra ese hueco.
#
# Uso:
#   ./scripts/use_repo_mok.sh              — descarga el cert del repo y enrola
#   ./scripts/use_repo_mok.sh ruta.der     — enrola un cert local
#   ./scripts/use_repo_mok.sh --show       — solo muestra el fingerprint

set -euo pipefail

REPO_URL="${REPO_URL:-https://repo-unsynclabs.neokuze.org/repo}"
CERT_URL="${REPO_URL}/UnsyncLabs.der"

# Fingerprint publicado del cert del repo. Está acá a propósito: si el cert que
# bajás no coincide con esto, alguien te está dando otro.
EXPECTED_SHA256="79:39:E1:BB:25:C9:DB:7B:04:08:AC:23:1A:68:3E:3F:80:D1:D1:32:42:13:12:A0:C6:FF:5B:1D:17:C9:FD:76"

die() { echo "error: $*" >&2; exit 1; }

fingerprint() {
    openssl x509 -inform DER -in "$1" -noout -fingerprint -sha256 \
        | sed 's/.*=//'
}

describe() {
    local cert="$1"
    echo "  Subject:     $(openssl x509 -inform DER -in "$cert" -noout -subject | sed 's/.*subject=//')"
    echo "  Válido hasta: $(openssl x509 -inform DER -in "$cert" -noout -enddate | sed 's/.*=//')"
    echo "  SHA256:      $(fingerprint "$cert")"
}

main() {
    local cert
    cert="$(mktemp)"
    trap 'rm -f "$cert"' EXIT

    if [ "${1:-}" = "--show" ] || [ -z "${1:-}" ]; then
        command -v curl >/dev/null || die "hace falta curl"
        echo "==> Descargando cert desde ${CERT_URL}"
        curl -fsSL "$CERT_URL" -o "$cert" || die "no se pudo descargar el cert"
    else
        [ -r "$1" ] || die "no se puede leer $1"
        cp "$1" "$cert"
    fi

    openssl x509 -inform DER -in "$cert" -noout >/dev/null 2>&1 \
        || die "el archivo no es un certificado X.509 en DER"

    echo
    echo "Certificado:"
    describe "$cert"
    echo

    local got
    got="$(fingerprint "$cert")"
    if [ "$got" != "$EXPECTED_SHA256" ]; then
        echo "  !! El fingerprint NO coincide con el publicado en este repo." >&2
        echo "     esperado: $EXPECTED_SHA256" >&2
        echo "     obtenido: $got" >&2
        die "abortando"
    fi
    echo "  Fingerprint coincide con el publicado en este repo."

    if [ "${1:-}" = "--show" ]; then
        exit 0
    fi

    [ "$(id -u)" -eq 0 ] || die "hay que correr esto como root para enrolar"
    command -v mokutil >/dev/null || die "hace falta mokutil (pacman -S mokutil)"

    if ! mokutil --sb-state 2>/dev/null | grep -qi enabled; then
        echo
        echo "Secure Boot está desactivado en esta máquina."
        echo "Enrolar la MOK no cambia nada acá: sin Secure Boot el keyring"
        echo ".machine queda vacío y el kernel ignora la lista de MOK."
        echo "Los módulos firmados por UnsyncLabs ya cargan igual, porque el"
        echo "cert va horneado en .builtin_trusted_keys."
        exit 0
    fi

    cat <<EOF

Vas a enrolar a UnsyncLabs como Machine Owner Key.

Esto significa que, con Secure Boot activo, tu máquina va a cargar cualquier
módulo de kernel firmado con la llave privada de UnsyncLabs, sin preguntarte.
Es una delegación de confianza real: si esa llave privada se filtra o el que
la tiene decide abusarla, el código firmado corre en tu kernel con Secure Boot
prendido y sin aviso.

Enrolá esto solo si confiás en quien opera el repo. Verificá el fingerprint de
arriba por un canal aparte, no solo contra este script — si alguien te sirvió
un repo falso, también te sirvió este archivo.

EOF
    read -rp "Continuar? [escribí 'si' para enrolar] " ans
    [ "$ans" = "si" ] || { echo "cancelado"; exit 0; }

    echo
    echo "==> mokutil te va a pedir una contraseña de un solo uso."
    echo "    Anotala: en el próximo reboot, MokManager (pantalla azul) te la"
    echo "    va a pedir para confirmar. Si no confirmás ahí, no se enrola nada."
    echo
    mokutil --import "$cert"

    echo
    echo "Listo. Reiniciá y elegí 'Enroll MOK' -> 'Continue' en MokManager."
    echo "Después verificá con:"
    echo "    mokutil --list-enrolled | grep -i unsynclabs"
    echo "    keyctl show %:.machine"
}

main "$@"
