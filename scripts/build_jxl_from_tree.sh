#!/usr/bin/env bash
# Build jxl_from_tree from the libjxl source tree and place the binary at
# ./jxl_from_tree (project root).  Uses system highway + brotli when available
# so no git submodule downloads are required.
#
# Usage: ./scripts/build_jxl_from_tree.sh [--dest <path>]
#   --dest  where to write the binary (default: ./jxl_from_tree)

set -euo pipefail

DEST="${1:-./jxl_from_tree}"
LIBJXL_TAG="v0.11.2"
NPROC=$(nproc 2>/dev/null || sysctl -n hw.logicalcpu 2>/dev/null || echo 4)

# ── Dependency checks ─────────────────────────────────────────────────────────

need() { command -v "$1" >/dev/null 2>&1 || { echo "error: '$1' is required but not found" >&2; exit 1; }; }
need cmake
need git

# Check for system highway (required; avoids large submodule download)
if ! pkg-config --exists libhwy 2>/dev/null && ! [ -f /usr/include/hwy/highway.h ]; then
    echo "error: libhwy (highway) headers not found." >&2
    echo "  Arch/CachyOS: pacman -S highway" >&2
    echo "  Debian/Ubuntu: apt install libhwy-dev" >&2
    exit 1
fi

# Check for system brotli (required; avoids submodule download)
if ! pkg-config --exists libbrotlienc 2>/dev/null; then
    echo "error: libbrotli not found." >&2
    echo "  Arch/CachyOS: pacman -S brotli" >&2
    echo "  Debian/Ubuntu: apt install libbrotli-dev" >&2
    exit 1
fi

# lcms2 is needed when skcms is disabled
if ! pkg-config --exists lcms2 2>/dev/null && ! [ -f /usr/include/lcms2.h ]; then
    echo "error: lcms2 headers not found." >&2
    echo "  Arch/CachyOS: pacman -S lcms2" >&2
    echo "  Debian/Ubuntu: apt install liblcms2-dev" >&2
    exit 1
fi

# ── Clone ─────────────────────────────────────────────────────────────────────

TMP_SRC=$(mktemp -d)
TMP_BUILD=$(mktemp -d)
cleanup() { rm -rf "$TMP_SRC" "$TMP_BUILD"; }
trap cleanup EXIT

echo "Cloning libjxl $LIBJXL_TAG (shallow, no submodules)..."
git clone --depth=1 --branch "$LIBJXL_TAG" https://github.com/libjxl/libjxl.git "$TMP_SRC" \
    --quiet 2>&1

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
cmake --build "$TMP_BUILD" --target jxl_from_tree --parallel "$NPROC"

# ── Install ───────────────────────────────────────────────────────────────────

cp "$TMP_BUILD/tools/jxl_from_tree" "$DEST"
chmod +x "$DEST"

echo "Done: $DEST ($(wc -c < "$DEST" | tr -d ' ') bytes)"
