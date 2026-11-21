# cmux-desktop

Native-feeling Linux (and cross-platform) GUI for cmux, talking to the
`cmux-tui` multiplexer daemon over `cmux.protocol/2` (JSON-lines on a unix
socket). Render-mode surface attach streams resolved styled cells — no local
VT emulator needed.

## Run

```sh
npm install
npm run build
CMUX_SESSION=main npx electron .   # starts `cmux server start --session main` if needed
```

Requires a `cmux`/`cmux-tui` binary on PATH (or `CMUX_BIN=/path/to/cmux-tui`).

## Features

- Workspace sidebar (select/create), resizable 120–260px
- Pane split tree from the server layout (`leaf`/`split`/`stack`), draggable
  split ratio via `set-split-ratio`
- Per-pane tab strip; `+` new tab; terminal tabs render via `attach-surface
  mode:"render"` onto a canvas (bold/italic/inverse/dim/invisible/strike,
  single/double/curly underline, block/bar/underline cursor)
- Browser tabs embed a real `<webview>`
- Keyboard input: text via `send`, named chords via `send-key`, bracketed
  paste, wheel scrolling via `scroll-surface`
- Notification rings: accent outline on panes with unread/bell, badge counts
- Cmd/Ctrl+K command palette: workspace switching, splits, tabs, zoom, undo
- Window chrome: 28px frameless titlebar matching the macOS metrics
