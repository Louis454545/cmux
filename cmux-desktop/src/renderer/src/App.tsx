import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useReducer,
  useState,
} from "react";
import { theme } from "./theme";
import type { Tree, Workspace, Screen, Pane, Tab, SplitNode } from "./protocol";
import TerminalSurface from "./TerminalSurface";
import BrowserTab from "./BrowserTab";
import CommandPalette from "./CommandPalette";

interface State {
  tree: Tree | null;
  focusedPane: number | null;
  paletteOpen: boolean;
  fatal: string | null;
}
type Action =
  | { type: "tree"; tree: Tree }
  | { type: "focus"; pane: number }
  | { type: "palette"; open: boolean }
  | { type: "fatal"; msg: string };

function reducer(s: State, a: Action): State {
  switch (a.type) {
    case "tree": {
      const activeWs = a.tree.workspaces.find((w) => w.active);
      const activeScreen = activeWs?.screens.find((s) => s.active) ?? activeWs?.screens[0];
      const focusedPane = activeScreen?.active_pane ?? s.focusedPane;
      return { ...s, tree: a.tree, focusedPane };
    }
    case "focus":
      return { ...s, focusedPane: a.pane };
    case "palette":
      return { ...s, paletteOpen: a.open };
    case "fatal":
      return { ...s, fatal: a.msg };
  }
}

const Ctx = createContext<{ state: State; dispatch: React.Dispatch<Action> }>(
  null as any
);
export const useApp = () => useContext(Ctx);

async function refresh(dispatch: React.Dispatch<Action>) {
  try {
    const data = await window.cmux.call("list-workspaces");
    dispatch({ type: "tree", tree: data as unknown as Tree });
  } catch (e) {
    console.error(e);
  }
}

export default function App() {
  const [state, dispatch] = useReducer(reducer, {
    tree: null,
    focusedPane: null,
    paletteOpen: false,
    fatal: null,
  });

  useEffect(() => {
    window.cmux.onEvent((e: any) => {
      if (
        e.event === "tree-changed" ||
        e.event === "layout-changed" ||
        e.event === "title-changed" ||
        e.event === "notification" ||
        e.event === "surface-exited"
      ) {
        refresh(dispatch);
      }
    });
    window.cmux.onFatal((msg) => dispatch({ type: "fatal", msg }));
    refresh(dispatch);
  }, []);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (mod && e.key.toLowerCase() === "k") {
        e.preventDefault();
        dispatch({ type: "palette", open: !state.paletteOpen });
      } else if (mod && e.key.toLowerCase() === "d" && e.shiftKey) {
        e.preventDefault();
        const p = state.focusedPane;
        if (p) window.cmux.call("new-pane-right", { pane: p }).then(() => refresh(dispatch));
      } else if (mod && e.key.toLowerCase() === "d") {
        e.preventDefault();
        const p = state.focusedPane;
        if (p) window.cmux.call("new-pane", { pane: p }).then(() => refresh(dispatch));
      } else if (mod && e.key.toLowerCase() === "t") {
        e.preventDefault();
        const p = state.focusedPane;
        if (p) window.cmux.call("new-tab", { pane: p }).then(() => refresh(dispatch));
      } else if (mod && e.key.toLowerCase() === "w") {
        e.preventDefault();
        closeFocusedTab(state, dispatch);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [state.paletteOpen, state.focusedPane, state.tree]);

  const activeWs = state.tree?.workspaces.find((w) => w.active);
  const activeScreen = activeWs?.screens.find((s) => s.active) ?? activeWs?.screens[0];

  return (
    <Ctx.Provider value={{ state, dispatch }}>
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          width: "100%",
          height: "100%",
          background: theme.windowBg,
          color: theme.text,
          fontFamily: theme.fontUI,
          fontSize: 13,
        }}
      >
        <Titlebar workspace={activeWs} />
        <div style={{ display: "flex", flex: 1, minHeight: 0 }}>
          <Sidebar />
          <div
            style={{
              flex: 1,
              minWidth: 0,
              display: "flex",
              flexDirection: "column",
              background: theme.windowBg,
            }}
          >
            {state.fatal ? (
              <div style={{ padding: 24, color: "#ff8a80" }}>{state.fatal}</div>
            ) : activeScreen ? (
              <ScreenView screen={activeScreen} />
            ) : (
              <div
                style={{
                  flex: 1,
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "center",
                  color: theme.textDim,
                }}
              >
                {state.tree ? "No workspace selected" : "Connecting to cmux…"}
              </div>
            )}
          </div>
        </div>
        {state.paletteOpen && <CommandPalette />}
      </div>
    </Ctx.Provider>
  );
}

function closeFocusedTab(state: State, dispatch: React.Dispatch<Action>) {
  const ws = state.tree?.workspaces.find((w) => w.active);
  const sc = ws?.screens.find((s) => s.active) ?? ws?.screens[0];
  const pane = sc?.panes.find((p) => p.id === state.focusedPane) ?? sc?.panes[0];
  const tab = pane?.tabs[pane.active_tab] ?? pane?.tabs[0];
  if (tab) window.cmux.call("close-surface", { surface: tab.surface }).then(() => refresh(dispatch));
}

// ---------------- Titlebar (28px chrome) ----------------
function Titlebar({ workspace }: { workspace?: Workspace }) {
  return (
    <div
      style={{
        height: theme.chromeBar,
        background: theme.titlebarBg,
        display: "flex",
        alignItems: "center",
        paddingLeft: 4,
        WebkitAppRegion: "drag",
        userSelect: "none",
        borderBottom: `1px solid ${theme.divider}`,
      } as React.CSSProperties}
    >
      <WinBtn title="Close" onClick={() => window.cmux.window.close()} />
      <WinBtn title="Minimize" onClick={() => window.cmux.window.minimize()} />
      <WinBtn title="Zoom" onClick={() => window.cmux.window.maximize()} />
      <div
        style={{
          flex: 1,
          textAlign: "center",
          fontSize: 13,
          fontWeight: 600,
          color: theme.textDim,
        }}
      >
        {workspace?.name ?? "cmux"}
      </div>
      <div style={{ width: 72 }} />
    </div>
  );
}

function WinBtn({ title, onClick }: { title: string; onClick: () => void }) {
  const [hov, setHov] = useState(false);
  const glyph = title === "Close" ? "✕" : title === "Minimize" ? "−" : "+";
  return (
    <button
      title={title}
      onClick={onClick}
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        WebkitAppRegion: "no-drag",
        width: theme.headerBtn,
        height: theme.headerBtn,
        borderRadius: theme.radius,
        border: "none",
        background: hov ? theme.hover : "transparent",
        color: theme.textDim,
        fontSize: theme.headerIcon,
        cursor: "default",
        marginRight: 2,
      } as React.CSSProperties}
    >
      {glyph}
    </button>
  );
}

// ---------------- Sidebar ----------------
function Sidebar() {
  const { state, dispatch } = useApp();
  const [width, setWidth] = useState<number>(theme.sidebarWidth);
  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startW = width;
    const move = (ev: MouseEvent) =>
      setWidth(
        Math.min(theme.sidebarMax, Math.max(theme.sidebarMin, startW + ev.clientX - startX))
      );
    const up = () => {
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };

  const newWorkspace = () =>
    window.cmux
      .call("create-workspace", {})
      .then((r: { workspace?: number }) =>
        r.workspace != null
          ? window.cmux.call("create-terminal", { workspace: r.workspace })
          : undefined
      )
      .then(() => refresh(dispatch));

  return (
    <div
      style={{
        width,
        minWidth: width,
        background: theme.sidebarBg,
        borderRight: `1px solid ${theme.divider}`,
        display: "flex",
        flexDirection: "column",
        position: "relative",
      }}
    >
      <div
        style={{
          height: theme.chromeBar,
          display: "flex",
          alignItems: "center",
          padding: "0 10px",
          color: theme.textFaint,
          fontSize: 11,
          fontWeight: 700,
          letterSpacing: 0.6,
          textTransform: "uppercase",
        }}
      >
        Workspaces
        <div style={{ flex: 1 }} />
        <button
          onClick={newWorkspace}
          title="New workspace"
          style={{
            background: "transparent",
            border: "none",
            color: theme.textDim,
            fontSize: 14,
            cursor: "pointer",
          }}
        >
          +
        </button>
      </div>
      <div style={{ flex: 1, overflowY: "auto", padding: "2px 6px" }}>
        {state.tree?.workspaces.map((ws) => (
          <WorkspaceRow key={ws.id} ws={ws} />
        ))}
      </div>
      <div
        onMouseDown={startDrag}
        style={{
          position: "absolute",
          top: 0,
          right: -3,
          width: 6,
          height: "100%",
          cursor: "col-resize",
        }}
      />
    </div>
  );
}

function WorkspaceRow({ ws }: { ws: Workspace }) {
  const { state, dispatch } = useApp();
  const [hov, setHov] = useState(false);
  const select = () => {
    // select-workspace takes a zero-based index, not the workspace id.
    const index = state.tree?.workspaces.findIndex((w) => w.id === ws.id) ?? -1;
    if (index < 0) return;
    window.cmux.call("select-workspace", { index }).then(() => refresh(dispatch));
  };
  const unread = ws.unread ?? 0;
  return (
    <div
      onClick={select}
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        height: theme.rowHeight,
        borderRadius: theme.radius,
        background: ws.active ? theme.active : hov ? theme.hover : "transparent",
        display: "flex",
        alignItems: "center",
        padding: "0 10px",
        marginBottom: 1,
        cursor: "default",
      }}
    >
      <div
        style={{
          width: 16,
          height: 16,
          borderRadius: "50%",
          border: `1.5px solid ${unread > 0 ? theme.accent : theme.textFaint}`,
          color: unread > 0 ? theme.accent : theme.textFaint,
          fontSize: 9,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          marginRight: 8,
          fontWeight: 700,
        }}
      >
        {unread > 0 ? unread : ws.key.slice(0, 1).toUpperCase()}
      </div>
      <div
        style={{
          fontWeight: ws.active ? 600 : 400,
          color: ws.active ? theme.text : theme.textDim,
          overflow: "hidden",
          textOverflow: "ellipsis",
          whiteSpace: "nowrap",
        }}
      >
        {ws.name}
      </div>
    </div>
  );
}

// ---------------- Screen: layout split tree ----------------
function ScreenView({ screen }: { screen: Screen }) {
  const { state } = useApp();
  const zoomed =
    screen.zoomed_pane != null
      ? screen.panes.find((p) => p.id === screen.zoomed_pane)
      : undefined;
  return (
    <div style={{ flex: 1, display: "flex", minHeight: 0 }}>
      {/* Keep the split tree mounted while zoomed so tabs/attaches survive. */}
      <div style={{ display: zoomed ? "none" : "flex", flex: 1, minHeight: 0, minWidth: 0 }}>
        <SplitNodeView
          node={screen.layout}
          panes={screen.panes}
          focusedPane={state.focusedPane}
        />
      </div>
      {zoomed && (
        <PaneView pane={zoomed} focused={zoomed.id === state.focusedPane} />
      )}
    </div>
  );
}

function SplitNodeView({
  node,
  panes,
  focusedPane,
}: {
  node: SplitNode;
  panes: Pane[];
  focusedPane: number | null;
}) {
  if (node.type === "stack" && node.panes) {
    const pane = panes.find((p) => p.id === node.expanded) ?? panes[0];
    return pane ? <PaneView pane={pane} focused={pane.id === focusedPane} /> : <div style={{ flex: 1 }} />;
  }
  if (node.type !== "split" || !node.a || !node.b) {
    const pane = panes.find((p) => p.id === node.pane) ?? panes[0];
    return pane ? (
      <PaneView pane={pane} focused={pane.id === focusedPane} />
    ) : (
      <div style={{ flex: 1 }} />
    );
  }
  return <SplitView node={node} panes={panes} focusedPane={focusedPane} />;
}

function SplitView({
  node,
  panes,
  focusedPane,
}: {
  node: SplitNode;
  panes: Pane[];
  focusedPane: number | null;
}) {
  // dir:"right" = side-by-side columns, dir:"down" = stacked rows
  const horiz = node.dir !== "down";
  const ratio = node.ratio ?? 0.5;
  const [dragging, setDragging] = useState(false);
  const startDrag = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    const parent = (e.currentTarget as HTMLElement).parentElement!;
    const rect = parent.getBoundingClientRect();
    const move = (ev: MouseEvent) => {
      const r = horiz
        ? (ev.clientX - rect.left) / rect.width
        : (ev.clientY - rect.top) / rect.height;
      window.cmux
        .call("set-split-ratio", { split: node.split, ratio: Math.min(0.95, Math.max(0.05, r)) })
        .catch(() => {});
    };
    const up = () => {
      setDragging(false);
      window.removeEventListener("mousemove", move);
      window.removeEventListener("mouseup", up);
    };
    setDragging(true);
    window.addEventListener("mousemove", move);
    window.addEventListener("mouseup", up);
  };
  return (
    <div
      style={{
        flex: 1,
        display: "flex",
        flexDirection: horiz ? "row" : "column",
        minHeight: 0,
        minWidth: 0,
      }}
    >
      <div
        style={{
          [horiz ? "width" : "height"]: `${ratio * 100}%`,
          display: "flex",
          minWidth: 0,
          minHeight: 0,
        } as React.CSSProperties}
      >
        <SplitNodeView node={node.a!} panes={panes} focusedPane={focusedPane} />
      </div>
      <div
        onMouseDown={startDrag}
        style={{
          [horiz ? "width" : "height"]: 9,
          [horiz ? "height" : "width"]: "100%",
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          cursor: horiz ? "col-resize" : "row-resize",
          flexShrink: 0,
          [horiz ? "marginLeft" : "marginTop"]: -4,
          [horiz ? "marginRight" : "marginBottom"]: -4,
          zIndex: 5,
        } as React.CSSProperties}
      >
        <div
          style={{
            [horiz ? "width" : "height"]: dragging ? 3 : 1,
            [horiz ? "height" : "width"]: "100%",
            background: dragging ? theme.accent : theme.divider,
          } as React.CSSProperties}
        />
      </div>
      <div style={{ flex: 1, display: "flex", minWidth: 0, minHeight: 0 }}>
        <SplitNodeView node={node.b!} panes={panes} focusedPane={focusedPane} />
      </div>
    </div>
  );
}

// ---------------- Pane: tab strip + active tab ----------------
function PaneView({ pane, focused }: { pane: Pane; focused: boolean }) {
  const { dispatch } = useApp();
  const focus = () => {
    window.cmux.call("focus-pane", { pane: pane.id }).then(() => refresh(dispatch));
    dispatch({ type: "focus", pane: pane.id });
  };
  const unread = pane.tabs.reduce((n, t) => n + (t.unread ?? 0) + (t.notification ? 1 : 0), 0);
  return (
    <div
      onMouseDown={focus}
      style={{
        flex: 1,
        display: "flex",
        flexDirection: "column",
        minWidth: 0,
        minHeight: 0,
        boxShadow: unread > 0 && !focused ? `inset 0 0 0 2px ${theme.accent}` : "none",
        background: theme.termBg,
      }}
    >
      <div
        style={{
          height: theme.chromeBar,
          background: focused ? theme.paneHeaderActiveBg : theme.paneHeaderBg,
          display: "flex",
          alignItems: "center",
          padding: "0 6px",
          overflowX: "auto",
          overflowY: "hidden",
        }}
      >
        {pane.tabs.map((t, i) => (
          <TabChip key={t.surface} tab={t} pane={pane} index={i} />
        ))}
        <button
          onClick={(e) => {
            e.stopPropagation();
            window.cmux.call("new-tab", { pane: pane.id }).then(() => refresh(dispatch));
          }}
          title="New tab"
          style={{
            background: "transparent",
            border: "none",
            color: theme.textDim,
            fontSize: 13,
            cursor: "pointer",
            padding: "0 6px",
          }}
        >
          +
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, position: "relative" }}>
        {pane.tabs.map((t, i) =>
          i === pane.active_tab ? (
            <ActiveTab key={t.surface} tab={t} pane={pane} focused={focused} onFocus={focus} />
          ) : null
        )}
      </div>
    </div>
  );
}

function TabChip({ tab, pane, index }: { tab: Tab; pane: Pane; index: number }) {
  const { dispatch } = useApp();
  const active = index === pane.active_tab;
  const [hov, setHov] = useState(false);
  const select = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.cmux
      .call("select-tab", { pane: pane.id, index })
      .then(() => refresh(dispatch));
  };
  const close = (e: React.MouseEvent) => {
    e.stopPropagation();
    window.cmux.call("close-surface", { surface: tab.surface }).then(() => refresh(dispatch));
  };
  const needsAttention = (tab.unread ?? 0) > 0 || tab.notification;
  return (
    <div
      onClick={select}
      onMouseEnter={() => setHov(true)}
      onMouseLeave={() => setHov(false)}
      style={{
        height: 22,
        padding: "0 8px",
        marginRight: 2,
        borderRadius: 5,
        display: "flex",
        alignItems: "center",
        background: active ? theme.active : hov ? theme.hover : "transparent",
        color: active ? theme.text : theme.textDim,
        fontSize: 12,
        whiteSpace: "nowrap",
        cursor: "default",
      }}
    >
      {tab.kind === "browser" ? "🌐 " : ""}
      {tab.title || tab.name || `tab ${tab.surface}`}
      {needsAttention && (
        <span
          style={{
            marginLeft: 6,
            minWidth: 14,
            height: 14,
            borderRadius: 7,
            background: theme.accent,
            color: "#fff",
            fontSize: 9,
            fontWeight: 700,
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            padding: "0 3px",
          }}
        >
          {tab.unread ?? 1}
        </span>
      )}
      {hov && (
        <span
          onClick={close}
          style={{ marginLeft: 6, color: theme.textFaint, fontSize: 11 }}
        >
          ✕
        </span>
      )}
    </div>
  );
}

function ActiveTab({
  tab,
  focused,
  onFocus,
}: {
  tab: Tab;
  pane: Pane;
  focused: boolean;
  onFocus: () => void;
}) {
  if (tab.kind === "browser") {
    return <BrowserTab tab={tab} />;
  }
  return <TerminalSurface surface={tab.surface} focused={focused} onFocus={onFocus} />;
}
