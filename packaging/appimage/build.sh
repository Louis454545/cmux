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
