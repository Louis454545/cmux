import React, { useEffect, useMemo, useRef, useState } from "react";
import { theme } from "./theme";
import { useApp } from "./App";
import type { Workspace } from "./protocol";

interface Cmd {
  id: string;
  title: string;
  run: () => Promise<unknown> | void;
}

export default function CommandPalette() {
  const { state, dispatch } = useApp();
  const [q, setQ] = useState("");
  const [sel, setSel] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<Cmd[]>(() => {
    const close = () => dispatch({ type: "palette", open: false });
    const after = async (p: Promise<unknown>) => {
      await p;
      const t = await window.cmux.call("list-workspaces");
      dispatch({ type: "tree", tree: t as any });
      close();
    };
    const withPane = (fn: (pane: number) => Promise<unknown>) => () => {
      if (state.focusedPane != null) return after(fn(state.focusedPane));
    };
    const list: Cmd[] = [
      { id: "new-ws", title: "New workspace", run: () => after(window.cmux.call("create-workspace", {})) },
      { id: "new-pane-right", title: "Split pane right", run: withPane((pane) => window.cmux.call("new-pane-right", { pane })) },
      { id: "new-pane", title: "Split pane down", run: withPane((pane) => window.cmux.call("new-pane", { pane })) },
      { id: "new-tab", title: "New tab", run: withPane((pane) => window.cmux.call("new-tab", { pane })) },
      { id: "new-browser-tab", title: "New browser tab", run: withPane((pane) => window.cmux.call("new-browser-tab", { pane })) },
      { id: "zoom-pane", title: "Zoom/unzoom pane", run: withPane((pane) => window.cmux.call("zoom-pane", { pane })) },
      { id: "undo-layout", title: "Undo layout change", run: () => after(window.cmux.call("undo-layout", {})) },
    ];
    for (const ws of state.tree?.workspaces ?? []) {
      list.push({
        id: `goto-ws-${ws.id}`,
        title: `Go to workspace: ${ws.name}`,
        run: () => after(window.cmux.call("select-workspace", { workspace: ws.id })),
      });
    }
    return list;
  }, [state.tree, state.focusedPane, dispatch]);

  const filtered = useMemo(() => {
    const l = q.toLowerCase();
    return commands.filter((c) => c.title.toLowerCase().includes(l));
  }, [commands, q]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === "Escape") dispatch({ type: "palette", open: false });
    else if (e.key === "ArrowDown") setSel((s) => Math.min(s + 1, filtered.length - 1));
    else if (e.key === "ArrowUp") setSel((s) => Math.max(s - 1, 0));
    else if (e.key === "Enter" && filtered[sel]) void filtered[sel].run();
  };

  return (
    <div
      onClick={() => dispatch({ type: "palette", open: false })}
      style={{
        position: "fixed",
        inset: 0,
        background: "rgba(0,0,0,0.4)",
        display: "flex",
        justifyContent: "center",
        paddingTop: 120,
        zIndex: 100,
      }}
    >
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          width: 480,
          height: "fit-content",
          maxHeight: 320,
          background: theme.sidebarBg,
          border: `1px solid ${theme.divider}`,
          borderRadius: 8,
          overflow: "hidden",
          boxShadow: "0 12px 40px rgba(0,0,0,0.5)",
        }}
      >
        <input
          ref={inputRef}
          value={q}
          onChange={(e) => {
            setQ(e.target.value);
            setSel(0);
          }}
          onKeyDown={onKey}
          placeholder="Type a command…"
          style={{
            width: "100%",
            height: 36,
            background: "transparent",
            border: "none",
            borderBottom: `1px solid ${theme.divider}`,
            color: theme.text,
            fontSize: 14,
            padding: "0 14px",
            outline: "none",
            fontFamily: theme.fontUI,
          }}
        />
        <div style={{ maxHeight: 280, overflowY: "auto", padding: 4 }}>
          {filtered.map((c, i) => (
            <div
              key={c.id}
              onClick={() => void c.run()}
              onMouseEnter={() => setSel(i)}
              style={{
                height: 28,
                display: "flex",
                alignItems: "center",
                padding: "0 10px",
                borderRadius: 5,
                background: i === sel ? theme.accent : "transparent",
                color: i === sel ? "#fff" : theme.text,
                fontSize: 13,
                cursor: "default",
              }}
            >
              {c.title}
            </div>
          ))}
          {filtered.length === 0 && (
            <div style={{ padding: 12, color: theme.textFaint, fontSize: 12 }}>
              No matching commands
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
