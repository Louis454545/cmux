# cmux on Arch Linux

`cmux-tui` is the cross-platform cmux multiplexer (the macOS app in `Sources/`
is native-only). On Arch Linux you get the full feature set through it:
sessions, workspaces, screens, panes, tabs, terminals, browser panes, agent
hooks and screen detection, durable servers, and remote machines.

## Option 1 — install script (prebuilt binary)

```bash
curl -fsSL https://cmux.com/tui/install.sh | sh
```

Installs `cmux` to `~/.cmux/bin` for `x86_64` and `aarch64` Linux. To also run
a persistent session as a systemd user service (with linger):

```bash
curl -fsSL https://cmux.com/tui/install.sh | sh -s -- --server
```

## Option 2 — build a package with makepkg

```bash
sudo pacman -S --needed base-devel git rust zig
cd packaging/arch
makepkg -si
```

The PKGBUILD builds `cmux-tui`, `cmux-tui-hook`, `chatmux-relay`, and
`cmux-relay` from this checkout and installs `cmux` to `/usr/bin`.

Zig version note: `ghostty-vt-sys` builds `libghostty-vt.a` from the `ghostty`
submodule and requires the version in `ghostty/build.zig.zon`'s
`minimum_zig_version`. If the repo `zig` package is older, use the AUR
`zig-bin` package or download the matching tarball from ziglang.org and point
`ZIG` at it:

```bash
ZIG=/path/to/zig makepkg -si
```

## Option 3 — build from source

```bash
sudo pacman -S --needed base-devel git rustup zig
rustup toolchain install 1.95.0 -c clippy rustfmt
git submodule update --init ghostty
cd cmux-tui
cargo run -p cmux-tui
```

## First run

```bash
cmux                    # create or attach the `main` session
cmux --session agents   # named session
```

`Ctrl-b ?` lists every shortcut. `cmux server install` registers a systemd
user unit (`--user` starts at login instead of using linger), `cmux attach`
reconnects to a detached server.

## Feature notes on Arch

- **Browser panes** use the Chrome DevTools Protocol and need a Chromium or
  Chrome binary: `sudo pacman -S chromium`.
- **Clipboard** needs `wl-clipboard` (Wayland) or `xclip`/`xsel` (X11).
- **Notifications/dark-mode detection** use the freedesktop portal when a
  desktop portal is running; the TUI works fine without one.
- **Remote machines** (SSH pairing, WireGuard relay) work as on macOS;
  `machine-agent` is Unix-only and runs on Arch.
- **Terminal**: run `cmux` inside Ghostty, foot, alacritty, kitty, WezTerm, or
  any modern terminal — the multiplexer renders inside it.

## The GUI: cmux-gui (native Rust/GPUI)

`cmux-gui/` is the graphical cmux app — a native Rust build on the GPUI
toolkit that speaks `cmux.protocol/2` to the `cmux-tui` daemon over its
unix socket: workspaces, split panes with draggable dividers, per-pane tabs,
real terminal surfaces (render-mode attach), notification badges, the
command palette (`Ctrl/Cmd-K`), pane zoom, and `Ctrl/Cmd-D`/`Ctrl/Cmd-T`/
`Ctrl/Cmd-W` chords.

```bash
cd cmux-gui
cargo build --release
CMUX_SESSION=main ./target/release/cmux-gui
```

Package it for pacman:

```bash
cd packaging/arch/cmux-gui
makepkg -si    # produces /usr/bin/cmux-gui + a .desktop entry
```

It needs a `cmux`/`cmux-tui` binary on `PATH` (any of the options above) or
`CMUX_BIN=/path/to/cmux-tui`, and it auto-starts
`cmux server start --session $CMUX_SESSION` (default `main`) when no socket
exists. GPUI has no embedded webview, so browser panes render as a link card
that opens the URL in your system browser. Wayland and X11 both work.

## The GUI: cmux-desktop (Electron, deprecated)

`cmux-desktop/` is the earlier Electron + React shell over the same
`cmux.protocol/2` socket. It is superseded by `cmux-gui`; it remains as a
reference implementation. Its one advantage is that browser panes embed a
real webview.
