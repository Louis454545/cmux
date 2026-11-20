import { EventEmitter } from "events";
import { Socket } from "net";
import { createHash } from "crypto";
import { join } from "path";
import { homedir } from "os";

// cmux.protocol/2 client over the unix socket JSON-lines transport.
// One MuxConnection per (session, stream kind): a "control" connection issues
// commands and carries the subscribe stream; each "attach" connection carries
// one render-mode surface stream, so render events never interleave with
// unrelated command traffic.

export type Json = Record<string, unknown>;

export interface Identify {
  app: string;
  version: string;
  protocol: number;
  capabilities: string[];
  session: string;
  pid: number;
  registry_id?: string;
  generation?: string;
  workspace_revision?: number;
}

function runtimeBase(): string {
  return process.env.XDG_RUNTIME_DIR || process.env.TMPDIR || "/tmp";
}

export function socketPathForSession(session: string): string {
  if (process.env.CMUX_TUI_SOCKET) return process.env.CMUX_TUI_SOCKET;
  const uid = process.getuid?.() ?? 0;
  const base = `${runtimeBase()}/cmux-tui-${uid}`;
  const candidate = join(base, `${session}.sock`);
  // Darwin/BSD limit is ~103 bytes, Linux 107; mirror the server fallback.
  if (Buffer.byteLength(candidate) < 100) return candidate;
  const sha = createHash("sha256").update(session, "utf8").digest("hex");
  return join(`${runtimeBase()}/cmux-tui-hashed-${uid}`, `${sha}.sock`);
}

export class MuxConnection extends EventEmitter {
  private sock: Socket;
  private buf = Buffer.alloc(0);
  private nextId = 1;
  private pending = new Map<number, { resolve: (v: Json) => void; reject: (e: Error) => void }>();
  readonly label: string;

  constructor(socketPath: string, label = "control") {
    super();
    this.label = label;
    this.sock = new Socket();
    this.sock.connect(socketPath);
    this.sock.on("connect", () => this.emit("connect"));
    this.sock.on("data", (d) => this.onData(d));
    this.sock.on("error", (e) => this.emit("error", e));
    this.sock.on("close", () => this.emit("close"));
  }

  private onData(d: Buffer) {
    this.buf = Buffer.concat([this.buf, d]);
    let idx: number;
    while ((idx = this.buf.indexOf(0x0a)) >= 0) {
      const line = this.buf.subarray(0, idx).toString("utf8");
      this.buf = this.buf.subarray(idx + 1);
      const trimmed = line.trim();
      if (!trimmed) continue;
      let msg: Json;
      try {
        msg = JSON.parse(trimmed);
      } catch {
        this.emit("protocol-error", trimmed);
        continue;
      }
      if (typeof msg.id === "number" && (msg.ok === true || msg.ok === false)) {
        const p = this.pending.get(msg.id);
        if (p) {
          this.pending.delete(msg.id);
          if (msg.ok) p.resolve((msg.data as Json) ?? {});
          else p.reject(new Error((msg.error as string) ?? "command failed"));
        }
      } else if (typeof msg.event === "string") {
        this.emit("event", msg);
      }
    }
  }

  call(cmd: string, params: Json = {}): Promise<Json> {
    const id = this.nextId++;
    const line = JSON.stringify({ id, cmd, ...params }) + "\n";
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.sock.write(line, (err) => {
        if (err) {
          this.pending.delete(id);
          reject(err);
        }
      });
    });
  }

  close() {
    this.sock.destroy();
  }
}

// High-level session driver used by the Electron main process.
export class MuxSession extends EventEmitter {
  control: MuxConnection;
  attaches = new Map<number, MuxConnection>();
  identity: Identify | null = null;
  constructor(public session: string, socketPath?: string) {
    super();
    this.control = new MuxConnection(socketPath ?? socketPathForSession(session));
    this.control.on("event", (e) => this.emit("event", e));
  }

  async init(): Promise<Identify> {
    this.identity = (await this.control.call("identify")) as unknown as Identify;
    await this.control.call("set-client-info", {
      name: "cmux-desktop",
      capabilities: ["shared-sizing-v1"],
    });
    await this.control.call("subscribe", { tree_events: "deltas" });
    return this.identity;
  }

  listWorkspaces() {
    return this.control.call("list-workspaces");
  }

  attachRender(surface: number, cols?: number, rows?: number): MuxConnection {
    const conn = new MuxConnection(socketPathForSession(this.session), `attach-${surface}`);
    const params: Json = { surface, mode: "render" };
    if (cols && rows && this.identity?.capabilities.includes("attach-initial-size")) {
      params.cols = cols;
      params.rows = rows;
    }
    conn.call("attach-surface", params).catch((e) => conn.emit("attach-error", e));
    this.attaches.set(surface, conn);
    conn.on("event", (e) => this.emit("surface-event", e));
    conn.on("close", () => this.attaches.delete(surface));
    return conn;
  }

  detach(surface: number) {
    this.attaches.get(surface)?.close();
    this.attaches.delete(surface);
  }

  close() {
    for (const c of this.attaches.values()) c.close();
    this.control.close();
  }
}

export function stateDir(): string {
  return join(
    process.env.XDG_STATE_HOME || join(homedir(), ".local", "state"),
    "cmux-desktop"
  );
}
