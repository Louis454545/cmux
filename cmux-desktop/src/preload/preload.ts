import { contextBridge, ipcRenderer } from "electron";

// One ipcRenderer listener per channel; fan out to renderer callbacks with
// real unsubscribe so React mounts don't accumulate stale listeners.
function channel<T>(name: string) {
  const subs = new Set<(v: T) => void>();
  ipcRenderer.on(name, (_e, v) => subs.forEach((cb) => cb(v)));
  return (cb: (v: T) => void) => {
    subs.add(cb);
    return () => subs.delete(cb);
  };
}

const api = {
  call: (cmd: string, params: Record<string, unknown> = {}) =>
    ipcRenderer.invoke("mux:call", cmd, params),
  attach: (surface: number, cols?: number, rows?: number) =>
    ipcRenderer.invoke("mux:attach", surface, cols, rows),
  detach: (surface: number) => ipcRenderer.invoke("mux:detach", surface),
  onIdentify: channel<unknown>("mux:identify"),
  onEvent: channel<any>("mux:event"),
  onSurfaceEvent: channel<any>("mux:surface-event"),
  onFatal: channel<string>("mux:fatal"),
  window: {
    minimize: () => ipcRenderer.send("window:minimize"),
    maximize: () => ipcRenderer.send("window:maximize"),
    close: () => ipcRenderer.send("window:close"),
  },
  openExternal: (url: string) => ipcRenderer.send("open-external", url),
};

contextBridge.exposeInMainWorld("cmux", api);
export type CmuxApi = typeof api;
