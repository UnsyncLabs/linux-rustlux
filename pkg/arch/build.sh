#!/bin/bash
# build.sh — entry point used by arch-builder's build-wrapper.sh.
#
# The wrapper runs this as the unprivileged "builder" user instead of calling
# makepkg directly, which is what pins the build flags: without them makepkg
# would default to _hardened=yes and a native -march, producing "linux-rustlux"
# rather than the "linux-rustlux-zen4" package this repo publishes.
set -euo pipefail

# The kernel tarball is verified against validpgpkeys in the PKGBUILD, so the
# signing keys have to be in the builder's keyring. A fresh container has an
# empty one; import them, and only fall back to skipping the check if the
# keyserver is unreachable (the sources still come from cdn.kernel.org over TLS).
PGP_FLAGS=()
if ! gpg --keyserver keyserver.ubuntu.com --recv-keys \
        ABAF11C65A2970B130ABE3C479BE3E4300411886 \
        647F28654894E3BD457199BE38DBBDC86092693E 2>/dev/null; then
    echo "[warn] could not import kernel signing keys; skipping PGP check"
    PGP_FLAGS+=(--skippgpcheck)
fi

_hardened=no \
_processor_opt=zen4 \
_bigscreen_beyond=yes \
    makepkg -s --noconfirm "${PGP_FLAGS[@]}"
