#!/usr/bin/env bash
# Build cmux-gui + cmux-tui release binaries and package them as an AppImage.
# Run on Linux (Arch: pacman -S rust clang cmake pkgconf wayland libxcb
# fontconfig freetype2 vulkan-icd-loader). Self-contained:
# the AppImage bundles the `cmux` daemon binary, so nothing else is needed
# besides a Vulkan/Wayland or X11-capable system.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${OUT:-$ROOT/dist}"
APPDIR="$OUT/cmux-gui.AppDir"
ARCH="$(uname -m)"

cd "$ROOT/cmux-tui"
cargo build --release --locked
cd "$ROOT/cmux-gui"
cargo build --release --locked

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin"
install -m755 "$ROOT/cmux-tui/target/release/cmux-tui" "$APPDIR/usr/bin/cmux"
install -m755 "$ROOT/cmux-gui/target/release/cmux-gui" "$APPDIR/usr/bin/cmux-gui"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
export PATH="$HERE/usr/bin:$PATH"
export LD_LIBRARY_PATH="$HERE/usr/lib${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
exec "$HERE/usr/bin/cmux-gui" "$@"
EOF
chmod +x "$APPDIR/AppRun"

cat > "$APPDIR/cmux-gui.desktop" <<'EOF'
[Desktop Entry]
Name=cmux
Comment=Terminal multiplexer for AI coding agents
Exec=cmux-gui
Icon=cmux-gui
Type=Application
Categories=TerminalEmulator;Development;
Terminal=false
EOF

install -m644 "$ROOT/Assets.xcassets/AppIcon.appiconset/256.png" "$APPDIR/cmux-gui.png"

# Bundle the shared libraries both binaries link against, minus the
# core runtime every glibc system already provides.
mkdir -p "$APPDIR/usr/lib"
EXCLUDE='^linux-vdso|^ld-linux|^ld-musl|^libc\.so|^libm\.so|^libdl\.so|^librt\.so|^libpthread\.so|^libresolv|^libnss_|^libutil\.so|^libgcc_s\.so|^libstdc\+\+\.so'
ldd "$APPDIR/usr/bin/cmux-gui" "$APPDIR/usr/bin/cmux" \
    | awk '/=> \// {print $3}' | sort -u | while read -r lib; do
    base="$(basename "$lib")"
    echo "$base" | grep -qE "$EXCLUDE" && continue
    install -m644 "$lib" "$APPDIR/usr/lib/$base"
done
# dlopened dependencies that ldd cannot see (GPUI loads Vulkan and the
# X11 xkbcommon extension at runtime).
for name in libvulkan.so.1 libxkbcommon-x11.so.0; do
    path="$(ldconfig -p | grep -F "$name" | awk 'NR == 1 {print $NF}')"
    [ -n "$path" ] && install -m644 "$path" "$APPDIR/usr/lib/$name"
done

if ! command -v appimagetool >/dev/null 2>&1; then
    TOOL="$OUT/appimagetool"
    curl -fsSL -o "$TOOL" \
        "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-${ARCH}.AppImage"
    chmod +x "$TOOL"
else
    TOOL="$(command -v appimagetool)"
fi

APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" "$APPDIR" "$OUT/cmux-gui-${ARCH}.AppImage"
echo "Built $OUT/cmux-gui-${ARCH}.AppImage"
