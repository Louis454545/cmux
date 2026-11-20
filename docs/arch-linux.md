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
