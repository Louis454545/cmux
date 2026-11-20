import { app, BrowserWindow, ipcMain, shell } from "electron";
import { join } from "path";
import { spawn, execFile } from "child_process";
import { promisify } from "util";
import { MuxSession, MuxConnection, socketPathForSession } from "./mux";

const execFileP = promisify(execFile);
const sessionName = process.env.CMUX_SESSION || "main";

let win: BrowserWindow | null = null;
let mux: MuxSession | null = null;
let ownedServer = false;

// Locate a cmux-tui binary: explicit env, then PATH.
async function cmuxBin(): Promise<string> {
  if (process.env.CMUX_BIN) return process.env.CMUX_BIN;
  for (const name of ["cmux", "cmux-tui"]) {
    try {
      const { stdout } = await execFileP("which", [name]);
      const p = stdout.trim();
      if (p) return p;
    } catch {
      /* keep looking */
    }
  }
  throw new Error(
    "cmux binary not found — install cmux-tui (see docs/arch-linux.md) or set CMUX_BIN"
  );
}

async function connectOrStart(): Promise<void> {
  const socketPath = socketPathForSession(sessionName);
  mux = new MuxSession(sessionName, socketPath);
  try {
    await mux.init();
    return;
  } catch (e) {
    mux.close();
    mux = null;
  }
  // No live server — start a detached owner.
  const bin = await cmuxBin();
  await new Promise<void>((resolve, reject) => {
    const child = spawn(bin, ["server", "start", "--session", sessionName], {
      detached: true,
      stdio: "ignore",
    });
    child.unref();
    child.on("error", reject);
    child.on("exit", (code) =>
      code === 0 ? undefined : reject(new Error(`server start exited ${code}`))
    );
    setTimeout(resolve, 400); // detached owner keeps running in background
  });
  ownedServer = true;
  // Wait for the socket to come up.
  const deadline = Date.now() + 10000;
  while (Date.now() < deadline) {
    mux = new MuxSession(sessionName, socketPath);
    try {
      await mux.init();
      return;
    } catch {
      mux.close();
      mux = null;
      await new Promise((r) => setTimeout(r, 200));
    }
  }
  throw new Error("cmux server did not come up");
}

function broadcast(channel: string, payload: unknown) {
  win?.webContents.send(channel, payload);
}

async function main() {
  await app.whenReady();

  win = new BrowserWindow({
    width: 1440,
    height: 900,
    minWidth: 640,
    minHeight: 400,
    frame: false,
    backgroundColor: "#1d1f21",
    title: "cmux",
    webPreferences: {
      preload: join(__dirname, "../preload/preload.js"),
      contextIsolation: true,
      nodeIntegration: false,
      webviewTag: true, // browser panes embed real pages
    },
  });

  if (process.env.ELECTRON_RENDERER_URL) {
    win.loadURL(process.env.ELECTRON_RENDERER_URL);
  } else {
    win.loadFile(join(__dirname, "../renderer/index.html"));
  }

  try {
    await connectOrStart();
  } catch (e) {
    broadcast("mux:fatal", String(e));
    return;
  }

  broadcast("mux:identify", mux!.identity);
  mux!.on("event", (e) => broadcast("mux:event", e));
  mux!.on("surface-event", (e) => broadcast("mux:surface-event", e));

  // ---- IPC: generic command bridge (validated names only) ----
  const COMMANDS = new Set([
    "list-workspaces", "create-workspace", "new-workspace", "new-pane", "new-pane-right",
    "split", "new-tab", "new-browser-tab", "create-terminal", "new-screen",
    "select-workspace", "select-screen", "select-tab", "focus-pane", "focus-direction",
    "close-surface", "close-pane", "close-screen", "close-workspace",
    "rename-workspace", "rename-screen", "rename-pane", "rename-surface",
    "send", "send-key", "resize-surface", "scroll-surface", "set-client-sizing",
    "read-scrollback", "copy", "notify", "ack-tab-notifications", "list-agents",
    "undo-layout", "zoom-pane", "swap-pane", "move-tab", "move-tab-to-workspace",
    "set-ratio", "set-split-ratio", "export-layout", "apply-layout", "ping",
    "read-screen", "set-window-title", "wait-for", "run",
  ]);
  ipcMain.handle("mux:call", (_e, cmd: string, params: Record<string, unknown>) => {
    if (!COMMANDS.has(cmd)) return Promise.reject(new Error(`disallowed command ${cmd}`));
    return mux!.control.call(cmd, params);
  });

  // ---- IPC: render attach per surface ----
  ipcMain.handle("mux:attach", (_e, surface: number, cols?: number, rows?: number) => {
    mux!.detach(surface);
    mux!.attachRender(surface, cols, rows);
  });
  ipcMain.handle("mux:detach", (_e, surface: number) => mux!.detach(surface));

  ipcMain.on("window:minimize", () => win?.minimize());
  ipcMain.on("window:maximize", () =>
    win?.isMaximized() ? win?.unmaximize() : win?.maximize()
  );
  ipcMain.on("window:close", () => win?.close());
  ipcMain.on("open-external", (_e, url: string) => {
    if (/^https?:\/\//.test(url)) shell.openExternal(url);
  });

  app.on("activate", () => {
    if (BrowserWindow.getAllWindows().length === 0) main();
  });
}

app.on("window-all-closed", () => {
  mux?.close();
  mux = null;
  app.quit();
});

main().catch((e) => {
  console.error(e);
  app.quit();
});
