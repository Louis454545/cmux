// Client-side tree state: mirrors `list-workspaces` snapshots and applies
// tree-changed refresh. Kept deliberately thin — the server is authoritative.

export type Listener = () => void;

export class Store<T> {
  private listeners = new Set<Listener>();
  constructor(private value: T) {}
  get(): T {
    return this.value;
  }
  set(v: T) {
    this.value = v;
    this.listeners.forEach((l) => l());
  }
  subscribe(l: Listener): () => void {
    this.listeners.add(l);
    return () => this.listeners.delete(l);
  }
}
