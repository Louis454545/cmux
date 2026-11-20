// Wire types for cmux.protocol/2 as consumed by the renderer.
export interface Size {
  cols: number;
  rows: number;
}
export interface Tab {
  surface: number;
  kind: "pty" | "browser" | string;
  browser_source: string | null;
  name: string | null;
  title: string;
  size?: Size;
  dead?: boolean;
  unread?: number;
  notification?: boolean;
}
export interface SplitNode {
  type: "leaf" | "split" | "stack";
  pane?: number;
  split?: number;
  dir?: "right" | "down";
  ratio?: number;
  a?: SplitNode;
  b?: SplitNode;
  panes?: number[];
  expanded?: number;
}
export interface Pane {
  id: number;
  name: string | null;
  active_tab: number;
  focused_at: number;
  tabs: Tab[];
}
export interface Screen {
  id: number;
  name: string | null;
  active: boolean;
  active_pane: number;
  layout: SplitNode;
  panes: Pane[];
}
export interface Workspace {
  id: number;
  key: string;
  name: string;
  active: boolean;
  unread?: number;
  screens: Screen[];
}
export interface Tree {
  workspace_revision: number;
  workspaces: Workspace[];
}
export interface Run {
  text: string;
  fg: string | null;
  bg: string | null;
  attrs: number;
  underline?: string;
  width_hint?: number;
}
export interface Row {
  row: number;
  runs: Run[];
}
export interface Cursor {
  x: number;
  y: number;
  style: "block" | "bar" | "underline";
  blink: boolean;
  visible: boolean;
  color: string | null;
}
export interface RenderState {
  event: "render-state";
  surface: number;
  size: Size;
  cursor: Cursor;
  default_fg: string;
  default_bg: string;
  scrollback_rows: number;
  history_epoch: number;
  rows: Row[];
}
export interface RenderDelta {
  event: "render-delta";
  surface: number;
  cursor: Cursor;
  full: boolean;
  size?: Size;
  default_fg?: string;
  default_bg?: string;
  scrollback_rows?: number;
  history_epoch?: number;
  rows: Row[];
}

declare global {
  interface Window {
    cmux: {
      call: (cmd: string, params?: Record<string, unknown>) => Promise<Record<string, unknown>>;
      attach: (surface: number, cols?: number, rows?: number) => Promise<void>;
      detach: (surface: number) => Promise<void>;
      onIdentify: (cb: (v: unknown) => void) => () => void;
      onEvent: (cb: (v: any) => void) => () => void;
      onSurfaceEvent: (cb: (v: any) => void) => () => void;
      onFatal: (cb: (v: string) => void) => () => void;
      window: { minimize(): void; maximize(): void; close(): void };
      openExternal: (url: string) => void;
    };
  }
}
export {};
