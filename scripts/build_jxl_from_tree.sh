#!/usr/bin/env bash
# Build jxl_from_tree from the libjxl source tree and place:
#   - the binary at ./jxl_from_tree (project root)
#   - its runtime .so files (libjxl_threads, libjxl_cms) in ./lib/
# The binary's RPATH is patched to $ORIGIN/lib so it is self-contained
# relative to the project root — no system-wide install needed.
#
# Uses system highway + brotli + lcms2 when available, so no git
# submodule downloads are required.
#
# Usage: ./scripts/build_jxl_from_tree.sh [<dest>]
#   <dest>  where to write the binary (default: ./jxl_from_tree).
#           Bundled .so files go into <dest's directory>/lib/.
#
# Override the libjxl revision with `LIBJXL_REV=<sha> …`.

set -euo pipefail

DEST="${1:-./jxl_from_tree}"
NPROC=$(nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 4)

# ── Dependency checks ─────────────────────────────────────────────────────────

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' is required but not found" >&2; exit 1; }; }
need cmake
need git
if ! command -v patchelf >/dev/null 2>&1; then
    echo "error: patchelf not found." >&2
    echo "  Arch/CachyOS: pacman -S patchelf" >&2
    echo "  Debian/Ubuntu: apt install patchelf" >&2
    exit 1
fi

# Check for a build-time dep. Tries pkg-config first (when available), then
# falls back to a header-file probe so the script still works on systems
# without pkg-config installed.
#   $1: pkg-config name      (e.g. libhwy)
#   $2: canonical header path (e.g. /usr/include/hwy/highway.h)
#   $3: human-readable name  (e.g. "libhwy (highway)")
#   $4: Arch package name    (e.g. highway)
#   $5: Debian package name  (e.g. libhwy-dev)
require_lib() {
    local pc_name="$1" header="$2" name="$3" arch_pkg="$4" deb_pkg="$5"
    if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists "$pc_name" 2>/dev/null; then
        return 0
    fi
    if [ -f "$header" ]; then
        return 0
    fi
    echo "error: $name not found (looked for pkg-config '$pc_name' and '$header')." >&2
    echo "  Arch/CachyOS: pacman -S $arch_pkg" >&2
    echo "  Debian/Ubuntu: apt install $deb_pkg" >&2
    exit 1
}

require_lib libhwy        /usr/include/hwy/highway.h  "libhwy (highway)" highway libhwy-dev
require_lib libbrotlienc  /usr/include/brotli/encode.h "libbrotli"        brotli  libbrotli-dev
# lcms2 is needed when skcms is disabled (we set JPEGXL_ENABLE_SKCMS=OFF below).
require_lib lcms2         /usr/include/lcms2.h        "lcms2"            lcms2   liblcms2-dev

# ── Clone ─────────────────────────────────────────────────────────────────────

TMP_SRC=$(mktemp -d)
TMP_BUILD=$(mktemp -d)
trap 'rm -rf "$TMP_SRC" "$TMP_BUILD"' EXIT

# Pin libjxl to v0.11.2. We previously tracked `main` for upscaling-header
# rendering fixes, but the SHA we'd pinned (05baa5ee, April 2026) had an
# encoder regression where `DeltaPalette + Squeeze + tree-branching-on-c`
# programs got silently encoded to a 22-byte all-black JXL — affected
# gallery entries `bg-167` through `bg-173`. v0.11.2 encodes them correctly.
# Override with `LIBJXL_REV=<sha> ./scripts/build_jxl_from_tree.sh` to test
# a different revision.
LIBJXL_REV="${LIBJXL_REV:-332feb17d17311c748445f7ee75c4fb55cc38530}" # v0.11.2

echo "Cloning libjxl @ ${LIBJXL_REV:0:12} (no submodules)..."
git clone https://github.com/libjxl/libjxl.git "$TMP_SRC" --quiet 2>&1
git -C "$TMP_SRC" checkout --quiet "$LIBJXL_REV"

# ── Configure ────────────────────────────────────────────────────────────────

echo "Configuring..."
cmake -S "$TMP_SRC" -B "$TMP_BUILD" \
    -DCMAKE_BUILD_TYPE=Release \
    -DJPEGXL_ENABLE_TOOLS=ON \
    -DJPEGXL_ENABLE_DEVTOOLS=ON \
    -DJPEGXL_FORCE_SYSTEM_HWY=ON \
    -DJPEGXL_FORCE_SYSTEM_BROTLI=ON \
    -DJPEGXL_ENABLE_SKCMS=OFF \
    -DJPEGXL_FORCE_SYSTEM_LCMS2=ON \
    -DJPEGXL_ENABLE_BENCHMARK=OFF \
    -DJPEGXL_ENABLE_EXAMPLES=OFF \
    -DJPEGXL_ENABLE_MANPAGES=OFF \
    -DJPEGXL_ENABLE_SJPEG=OFF \
    -DJPEGXL_ENABLE_OPENEXR=OFF \
    -DJPEGXL_ENABLE_VIEWERS=OFF \
    -DJPEGXL_ENABLE_PLUGINS=OFF \
    -DJPEGXL_ENABLE_DOXYGEN=OFF \
    -DBUILD_TESTING=OFF \
    -Wno-dev \
    >/dev/null

# ── Build ─────────────────────────────────────────────────────────────────────

echo "Building jxl_from_tree (using $NPROC cores)..."
cmake --build "$TMP_BUILD" --parallel "$NPROC"

# ── Install ───────────────────────────────────────────────────────────────────
#
# We don't `cmake --install`: that would splatter libjxl into /usr/local/lib
# (overriding the system libjxl on machines that already have it from their
# package manager). Instead, we bundle the runtime .so files into a project-
# local ./lib/ and set the binary's RPATH to $ORIGIN/lib — so the binary is
# self-contained relative to the project root, which is exactly the CWD the
# Rust server invokes it from.

cp "$TMP_BUILD/tools/jxl_from_tree" "$DEST"
chmod +x "$DEST"

LIB_DEST="$(dirname "$DEST")/lib"
mkdir -p "$LIB_DEST"
# `cp -P` preserves the SONAME symlink chain
# (libjxl_threads.so → .so.0 → .so.0.12.x).
cp -P "$TMP_BUILD"/lib/libjxl_threads.so* "$LIB_DEST/"
cp -P "$TMP_BUILD"/lib/libjxl_cms.so*     "$LIB_DEST/"

# Single-quoted: $ORIGIN is an ld.so token — must reach the linker literally.
patchelf --set-rpath '$ORIGIN/lib' "$DEST"

echo "Done: $DEST ($(wc -c < "$DEST" | tr -d ' ') bytes), libs in $LIB_DEST"
