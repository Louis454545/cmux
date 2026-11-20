import React, { useEffect, useRef } from "react";
import { theme } from "./theme";
import type { Cursor, RenderDelta, RenderState, Row, Run } from "./protocol";

// Canvas renderer for protocol-v7 `attach-surface mode:"render"` styled runs.
// The server resolves palette/OSC colors and damage; we paint cells.

const ATTR_BOLD = 0x1;
const ATTR_ITALIC = 0x2;
const ATTR_STRIKE = 0x4;
const ATTR_INVERSE = 0x8;
const ATTR_DIM = 0x10;
const ATTR_INVISIBLE = 0x20;
const ATTR_BLINK = 0x40;

export interface SurfaceModel {
  cols: number;
  rows: number;
  cells: Row[];
  cursor: Cursor;
  defaultFg: string;
  defaultBg: string;
  scrollbackRows: number;
  offset: number; // scroll viewport offset from bottom
}

export function applyRenderEvent(model: SurfaceModel, e: RenderState | RenderDelta): SurfaceModel {
  if (e.event === "render-state") {
    return {
      ...model,
      cols: e.size.cols,
      rows: e.size.rows,
      cells: e.rows,
      cursor: e.cursor,
      defaultFg: e.default_fg,
      defaultBg: e.default_bg,
      scrollbackRows: e.scrollback_rows,
    };
  }
  let cells: Row[];
  if (e.full) {
    cells = e.rows;
  } else {
    // Merge delta rows into the model by row index — safe even if the
    // initial render-state was missed (cells may be sparse).
    const byRow = new Map(model.cells.map((r) => [r.row, r]));
    for (const upd of e.rows) byRow.set(upd.row, upd);
    const n = e.size?.rows ?? model.rows;
    cells = [];
    for (let i = 0; i < n; i++) {
      const r = byRow.get(i);
      if (r) cells.push(r);
    }
  }
  return {
    ...model,
    cols: e.size?.cols ?? model.cols,
    rows: e.size?.rows ?? model.rows,
    cells,
    cursor: e.cursor,
    defaultFg: e.default_fg ?? model.defaultFg,
    defaultBg: e.default_bg ?? model.defaultBg,
    scrollbackRows: e.scrollback_rows ?? model.scrollbackRows,
  };
}

export function emptyModel(): SurfaceModel {
  return {
    cols: 80,
    rows: 24,
    cells: [],
    cursor: { x: 0, y: 0, style: "block", blink: true, visible: true, color: null },
    defaultFg: theme.termFg,
    defaultBg: theme.termBg,
    scrollbackRows: 0,
    offset: 0,
  };
}

interface Props {
  surface: number;
  focused: boolean;
  onFocus: () => void;
}

// Measure a cell from the loaded font once per font-size.
const FONT_SIZE = 13;
function useCellMetrics() {
  const ref = useRef({ w: 8, h: 17, measured: false });
  if (!ref.current.measured) {
    const c = document.createElement("canvas");
    const ctx = c.getContext("2d")!;
    ctx.font = `${FONT_SIZE}px ${theme.fontMono}`;
    const m = ctx.measureText("M");
    ref.current = {
      w: Math.max(1, m.width),
      h: Math.ceil(FONT_SIZE * 1.3),
      measured: true,
    };
  }
  return ref.current;
}

export default function TerminalSurface({ surface, focused, onFocus }: Props) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const modelRef = useRef<SurfaceModel>(emptyModel());
  const cell = useCellMetrics();

  useEffect(() => {
    const onEvent = (e: any) => {
      if (e.surface !== surface) return;
      if (e.event === "render-state" || e.event === "render-delta") {
        modelRef.current = applyRenderEvent(modelRef.current, e);
        draw();
      } else if (e.event === "scroll-changed") {
        modelRef.current.offset = e.offset;
        draw();
      } else if (e.event === "detached") {
        draw();
      }
    };
    const off = window.cmux.onSurfaceEvent(onEvent);
    // Attach only while visible: a display:none surface (e.g. under a
    // zoomed sibling) has no size and must not resize the shared PTY.
    let attached = false;
    const ensureAttach = () => {
      const el = wrapRef.current;
      if (attached || !el || el.clientWidth === 0 || el.clientHeight === 0) return;
      attached = true;
      const size = fitSize();
      window.cmux.attach(surface, size.cols, size.rows);
    };
    ensureAttach();
    draw();

    // Fit the surface to the pane: resize the PTY when the container
    // changes size (splits, sidebar drag, window resize).
    let resizeTimer: ReturnType<typeof setTimeout> | null = null;
    const ro = new ResizeObserver(() => {
      if (resizeTimer) clearTimeout(resizeTimer);
      resizeTimer = setTimeout(() => {
        const el = wrapRef.current;
        if (!el || el.clientWidth === 0 || el.clientHeight === 0) return;
        ensureAttach();
        const s = fitSize();
        const m = modelRef.current;
        if (s.cols !== m.cols || s.rows !== m.rows) {
          window.cmux
            .call("resize-surface", { surface, cols: s.cols, rows: s.rows })
            .catch(console.error);
        }
      }, 80);
    });
    if (wrapRef.current) ro.observe(wrapRef.current);

    return () => {
      ro.disconnect();
      if (resizeTimer) clearTimeout(resizeTimer);
      off?.();
      window.cmux.detach(surface);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [surface]);

  const fitSize = () => {
    const el = wrapRef.current;
    if (!el) return { cols: modelRef.current.cols, rows: modelRef.current.rows };
    return {
      cols: Math.max(2, Math.floor(el.clientWidth / cell.w)),
      rows: Math.max(1, Math.floor(el.clientHeight / cell.h)),
    };
  };

  const draw = () => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const m = modelRef.current;
    const dpr = window.devicePixelRatio || 1;
    const W = m.cols * cell.w;
    const H = m.rows * cell.h;
    if (canvas.width !== W * dpr || canvas.height !== H * dpr) {
      canvas.width = W * dpr;
      canvas.height = H * dpr;
      canvas.style.width = `${W}px`;
      canvas.style.height = `${H}px`;
    }
    const ctx = canvas.getContext("2d")!;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.fillStyle = m.defaultBg;
    ctx.fillRect(0, 0, W, H);
    ctx.textBaseline = "alphabetic";

    for (const row of m.cells) {
      const y = row.row * cell.h;
      let x = 0;
      for (const run of row.runs) {
        let fg = run.fg ?? m.defaultFg;
        let bg = run.bg ?? m.defaultBg;
        if (run.attrs & ATTR_INVERSE) [fg, bg] = [bg, fg];
        if (run.attrs & ATTR_DIM) fg = dimColor(fg);
        const text = run.attrs & ATTR_INVISIBLE ? " ".repeat(run.text.length) : run.text;
        const w = (run.width_hint ?? run.text.length) * cell.w;
        if (bg !== m.defaultBg) {
          ctx.fillStyle = bg;
          ctx.fillRect(x, y, w, cell.h);
        }
        if (text.trim().length > 0) {
          const weight = run.attrs & ATTR_BOLD ? "bold " : "";
          const style = run.attrs & ATTR_ITALIC ? "italic " : "";
          ctx.font = `${style}${weight}${FONT_SIZE}px ${theme.fontMono}`;
          ctx.fillStyle = fg;
          ctx.fillText(text, x, y + cell.h - 3);
        }
        if (run.underline || run.attrs & ATTR_STRIKE) {
          ctx.strokeStyle = fg;
          ctx.lineWidth = 1;
          const ly = run.underline ? y + cell.h - 1 : y + cell.h / 2;
          ctx.beginPath();
          if (run.underline === "double") {
            ctx.moveTo(x, ly - 2);
            ctx.lineTo(x + w, ly - 2);
          }
          if (run.underline === "curly") {
            // simple wavy approximation
            let cx = x;
            ctx.moveTo(cx, ly);
            while (cx < x + w) {
              ctx.quadraticCurveTo(cx + cell.w / 4, ly - 2, cx + cell.w / 2, ly);
              cx += cell.w / 2;
            }
          } else {
            ctx.moveTo(x, ly);
            ctx.lineTo(x + w, ly);
          }
          ctx.stroke();
        }
        x += w;
      }
    }

    // Cursor
    if (m.cursor.visible && focused !== false) {
      const cx = m.cursor.x * cell.w;
      const cy = m.cursor.y * cell.h;
      ctx.fillStyle = m.cursor.color ?? m.defaultFg;
      if (m.cursor.style === "block") {
        ctx.globalAlpha = 0.75;
        ctx.fillRect(cx, cy, cell.w, cell.h);
        ctx.globalAlpha = 1;
      } else if (m.cursor.style === "bar") {
        ctx.fillRect(cx, cy, 2, cell.h);
      } else {
        ctx.fillRect(cx, cy + cell.h - 2, cell.w, 2);
      }
    }
  };

  const sendText = (text: string, paste = false) => {
    window.cmux.call("send", { surface, text, paste }).catch(console.error);
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    const named = namedKey(e);
    if (e.metaKey && e.key === "v") return; // let paste handler run
    // App chords (palette, splits, tabs, close) stay on the window listener.
    if (
      (e.metaKey || e.ctrlKey) &&
      !e.altKey &&
      ["k", "d", "t", "w"].includes(e.key.toLowerCase())
    )
      return;
    e.preventDefault();
    e.stopPropagation();
    if (named) {
      window.cmux.call("send-key", { surface, keys: [named] }).catch(console.error);
    } else if (e.key.length === 1) {
      let text = e.key;
      if (e.ctrlKey) {
        // Ctrl+Space is NUL; other letters map to their control character.
        text = text === " " ? "\x00" : String.fromCharCode(text.toUpperCase().charCodeAt(0) - 64);
      }
      if (e.altKey) text = "\x1b" + text; // Meta sends ESC prefix.
      sendText(text);
    } else if (e.key === "Enter") {
      sendText("\r");
    }
  };

  const onPaste = (e: React.ClipboardEvent) => {
    e.preventDefault();
    sendText(e.clipboardData.getData("text"), true);
  };

  const onWheel = (e: React.WheelEvent) => {
    const delta = e.deltaY > 0 ? 3 : e.deltaY < 0 ? -3 : 0;
    if (delta) window.cmux.call("scroll-surface", { surface, delta }).catch(() => {});
  };

  return (
    <div
      ref={wrapRef}
      className="terminal-surface"
      tabIndex={0}
      onKeyDown={onKeyDown}
      onPaste={onPaste}
      onWheel={onWheel}
      onMouseDown={onFocus}
      style={{
        outline: "none",
        background: modelRef.current.defaultBg,
        width: "100%",
        height: "100%",
        overflow: "hidden",
        cursor: "text",
      }}
    >
      <canvas ref={canvasRef} />
    </div>
  );
}

function namedKey(e: React.KeyboardEvent): string | null {
  const map: Record<string, string> = {
    Enter: "enter",
    Backspace: "backspace",
    Tab: "tab",
    Escape: "escape",
    ArrowUp: "up",
    ArrowDown: "down",
    ArrowLeft: "left",
    ArrowRight: "right",
    Home: "home",
    End: "end",
    PageUp: "pageup",
    PageDown: "pagedown",
    Delete: "delete",
    Insert: "insert",
  };
  for (let i = 1; i <= 24; i++) map[`F${i}`] = `f${i}`;
  if (e.key === "Tab" && e.shiftKey) {
    const rest = [e.ctrlKey && "ctrl", e.altKey && "alt"].filter(Boolean).join("+");
    return rest ? `${rest}+backtab` : "backtab";
  }
  const base = map[e.key];
  if (!base) return null;
  const mods = [e.ctrlKey && "ctrl", e.altKey && "alt", e.shiftKey && "shift"]
    .filter(Boolean)
    .join("+");
  return mods ? `${mods}+${base}` : base;
}

function dimColor(hex: string): string {
  const m = /^#(..)(..)(..)$/.exec(hex);
  if (!m) return hex;
  const f = (s: string) => Math.round(parseInt(s, 16) * 0.66);
  return `#${f(m[1]).toString(16).padStart(2, "0")}${f(m[2])
    .toString(16)
    .padStart(2, "0")}${f(m[3]).toString(16).padStart(2, "0")}`;
}
