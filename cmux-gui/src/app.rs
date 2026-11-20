// Root GPUI view: sidebar + split-tree panes + tab strips + palette.
use crate::model::*;
use crate::mux::Mux;
use crate::terminal;
use crate::theme;
use gpui::*;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

actions!(
    cmux,
    [
        TogglePalette,
        SplitDown,
        SplitRight,
        NewTab,
        NewBrowserTab,
        CloseTab,
        ZoomPane,
        NewWorkspace,
    ]
);

type EventTx = smol::channel::Sender<Value>;
type Job = Box<dyn FnOnce(&Mux, &EventTx) + Send>;

#[derive(Clone)]
pub enum Cmd {
    Call { name: &'static str, params: Value },
    NewWorkspace,
    SelectWorkspace(usize),
    OpenBrowser(String),
}

pub struct RootView {
    pub(crate) fatal: Option<String>,
    pub(crate) tree: Tree,
    pub(crate) surfaces: HashMap<u64, SurfaceModel>,
    pub(crate) attached: HashSet<u64>,
    pub(crate) focused_pane: Option<u64>,
    pub(crate) sidebar_w: f32,
    pub(crate) dragging_sidebar: bool,
    pub(crate) sidebar_drag_x: Pixels,
    pub(crate) dragging_split: Option<(u64, Bounds<Pixels>, bool)>, // split id, parent rect, horiz
    pub(crate) palette_open: bool,
    pub(crate) palette_query: String,
    pub(crate) palette_sel: usize,
    pub(crate) focus: FocusHandle,
    pub(crate) cell_w: f32,
    event_rx: smol::channel::Receiver<Value>,
    pub(crate) event_tx: EventTx,
    call_tx: std::sync::mpsc::Sender<Job>,
}

impl RootView {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let (event_tx, event_rx) = smol::channel::unbounded::<Value>();
        let (call_tx, call_rx) = std::sync::mpsc::channel::<Job>();
        let session = std::env::var("CMUX_SESSION").unwrap_or_else(|_| "main".into());
        {
            let event_tx = event_tx.clone();
            std::thread::spawn(move || {
                eprintln!("[cmux-gui] worker: connecting session={session}");
                let mux = match Mux::connect_or_start(&session, event_tx.clone()) {
                    Ok(m) => {
                        eprintln!("[cmux-gui] worker: connected");
                        m
                    }
                    Err(e) => {
                        let _ = event_tx
                            .send_blocking(json!({"event":"fatal","error":e}));
                        return;
                    }
                };
                if let Ok(t) = mux.control.call("list-workspaces", serde_json::json!({})) {
                    let _ = event_tx.send_blocking(serde_json::json!({"event":"tree","tree":t}));
                }
                eprintln!("[cmux-gui] worker: job loop start");
                while let Ok(job) = call_rx.recv() {
                    job(&mux, &event_tx);
                }
            });
        }
        // Pump wire events into the view on the foreground executor.
        cx.spawn(async move |this: WeakEntity<RootView>, cx: &mut AsyncApp| {
            let rx = match this.update(cx, |v, _cx| v.event_rx.clone()) {
                Ok(rx) => rx,
                Err(_) => return,
            };
            while let Ok(v) = rx.recv().await {
                if this.update(cx, |v2, cx| v2.on_wire(v, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();

        let view = Self {
            fatal: None,
            tree: Tree::default(),
            surfaces: HashMap::new(),
            attached: HashSet::new(),
            focused_pane: None,
            sidebar_w: theme::SIDEBAR_W,
            sidebar_drag_x: px(0.),
            dragging_sidebar: false,
            dragging_split: None,
            palette_open: false,
            palette_query: String::new(),
            palette_sel: 0,
            focus: cx.focus_handle(),
            cell_w: 7.8,
            event_rx,
            event_tx,
            call_tx,
        };
        window.focus(&view.focus);
        view
    }

    pub(crate) fn enqueue(&self, job: Job) {
        let _ = self.call_tx.send(job);
    }

    /// Run a protocol call on the worker thread; it pushes the refreshed
    /// tree back onto the event channel.
    pub(crate) fn call(&self, cmd: &'static str, params: Value) {
        self.enqueue(Box::new(move |mux: &Mux, ev: &EventTx| {
            let _ = mux.control.call(cmd, params);
            if let Ok(t) = mux.control.call("list-workspaces", json!({})) {
                let _ = ev.send_blocking(json!({"event": "tree", "tree": t}));
            }
        }));
    }

    pub(crate) fn run_cmd(&mut self, cmd: Cmd, cx: &mut Context<Self>) {
        match cmd {
            Cmd::Call { name, params } => self.call(name, params),
            Cmd::SelectWorkspace(i) => self.call("select-workspace", json!({"index": i})),
            Cmd::NewWorkspace => {
                self.enqueue(Box::new(|mux: &Mux, ev: &EventTx| {
                    if let Ok(r) = mux.control.call("create-workspace", json!({})) {
                        if let Some(id) = r.get("workspace").and_then(|w| w.as_u64()) {
                            let _ = mux
                                .control
                                .call("create-terminal", json!({"workspace": id}));
                        }
                    }
                    if let Ok(t) = mux.control.call("list-workspaces", json!({})) {
                        let _ = ev.send_blocking(json!({"event": "tree", "tree": t}));
                    }
                }));
            }
            Cmd::OpenBrowser(url) => cx.open_url(&url),
        }
    }

    pub(crate) fn on_wire(&mut self, v: Value, cx: &mut Context<Self>) {
        let ev = v.get("event").and_then(|e| e.as_str()).unwrap_or("");
        match ev {
            "fatal" => {
                self.fatal = v.get("error").and_then(|e| e.as_str()).map(String::from)
            }
            "tree" => {
                if let Some(t) = v.get("tree") {
                    match serde_json::from_value::<Tree>(t.clone()) {
                        Err(e) => {
                            eprintln!("[cmux-gui] tree parse error: {e}");
                            eprintln!("[cmux-gui] tree head: {}", &t.to_string()[..400.min(t.to_string().len())]);
                        }
                        Ok(tree) => {
                            eprintln!("[cmux-gui] tree ok: {} workspaces", tree.workspaces.len());
                            self.tree = tree;
                            if self
                                .focused_pane
                                .map(|p| !self.all_panes().iter().any(|x| x.id == p))
                                .unwrap_or(true)
                            {
                                self.focused_pane = self
                                    .active_screen()
                                    .map(|s| s.active_pane)
                                    .filter(|p| *p != 0)
                                    .or_else(|| {
                                        self.active_screen()
                                            .and_then(|s| s.panes.first().map(|p| p.id))
                                    });
                            }
                        }
                    }
                }
            }
            "tree-changed" | "layout-changed" | "title-changed" | "notification"
            | "surface-exited" | "workspace-changed" | "pane-changed" => self.refresh(),
            "render-state" | "render-delta" => {
                if let Some(surface) = v.get("surface").and_then(|s| s.as_u64()) {
                    let model = self.surfaces.entry(surface).or_default();
                    apply_render_event(model, &v);
                }
            }
            "scroll-changed" => {
                if let Some(surface) = v.get("surface").and_then(|s| s.as_u64()) {
                    if let Some(m) = self.surfaces.get_mut(&surface) {
                        m.offset = v.get("offset").and_then(|o| o.as_i64()).unwrap_or(0);
                    }
                }
            }
            "detached" => {
                if let Some(surface) = v.get("surface").and_then(|s| s.as_u64()) {
                    self.surfaces.remove(&surface);
                }
            }
            _ => {}
        }
        cx.notify();
    }

    pub(crate) fn refresh(&self) {
        self.enqueue(Box::new(|mux: &Mux, ev: &EventTx| {
            if let Ok(t) = mux.control.call("list-workspaces", json!({})) {
                let _ = ev.send_blocking(json!({"event": "tree", "tree": t}));
            }
        }));
    }

    pub(crate) fn all_panes(&self) -> Vec<&Pane> {
        self.tree
            .workspaces
            .iter()
            .flat_map(|w| w.screens.iter().flat_map(|s| s.panes.iter()))
            .collect()
    }

    pub(crate) fn active_workspace(&self) -> Option<&Workspace> {
        self.tree
            .workspaces
            .iter()
            .find(|w| w.active)
            .or(self.tree.workspaces.first())
    }

    pub(crate) fn active_screen(&self) -> Option<&Screen> {
        let ws = self.active_workspace()?;
        ws.screens.iter().find(|s| s.active).or(ws.screens.first())
    }

    pub(crate) fn focused_terminal_surface(&self) -> Option<u64> {
        let pane_id = self.focused_pane?;
        let pane = self.all_panes().into_iter().find(|p| p.id == pane_id)?;
        let tab = pane.tabs.get(pane.active_tab)?;
        if tab.dead {
            return None;
        }
        (tab.kind.is_empty() || tab.kind == "pty").then_some(tab.surface)
    }

    pub(crate) fn with_pane(&mut self, name: &'static str) {
        if let Some(p) = self.focused_pane {
            self.call(name, json!({"pane": p}));
        }
    }

    // ---- keyboard ----
    pub(crate) fn on_key_down(&mut self, e: &KeyDownEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.palette_open {
            self.palette_key(e, cx);
            return;
        }
        let ks = &e.keystroke;
        let mods = &ks.modifiers;
        // Bound actions (cmd/ctrl chords) are handled by the keymap;
        // platform chords that aren't bound get ignored rather than typed.
        if mods.platform {
            return;
        }
        if let Some(surface) = self.focused_terminal_surface() {
            if let Some(named) = named_key(ks) {
                self.call("send-key", json!({"surface": surface, "keys": [named]}));
            } else if mods.control {
                if let Some(ch) = &ks.key_char {
                    if let Some(c) = ch.chars().next() {
                        let text = if c == ' ' {
                            "\x00".to_string()
                        } else {
                            let b = c.to_ascii_uppercase() as u8;
                            String::from_utf8(vec![b.saturating_sub(64)])
                                .unwrap_or_default()
                        };
                        if !text.is_empty() {
                            self.call(
                                "send",
                                json!({"surface": surface, "text": text, "paste": false}),
                            );
                        }
                    }
                }
            } else if let Some(ch) = &ks.key_char {
                if !ch.is_empty() {
                    let text = if mods.alt {
                        format!("\x1b{}", ch)
                    } else {
                        ch.clone()
                    };
                    self.call(
                        "send",
                        json!({"surface": surface, "text": text, "paste": false}),
                    );
                }
            }
        }
        cx.notify();
    }

    fn palette_items(&self) -> Vec<(String, Cmd)> {
        let mut items: Vec<(String, Cmd)> = vec![
            ("New workspace".into(), Cmd::NewWorkspace),
            ("Split pane down".into(), Cmd::Call {
                name: "new-pane",
                params: json!({"pane": self.focused_pane.unwrap_or(0)}),
            }),
            ("Split pane right".into(), Cmd::Call {
                name: "new-pane-right",
                params: json!({"pane": self.focused_pane.unwrap_or(0)}),
            }),
            ("New tab".into(), Cmd::Call {
                name: "new-tab",
                params: json!({"pane": self.focused_pane.unwrap_or(0)}),
            }),
            ("New browser tab".into(), Cmd::Call {
                name: "new-browser-tab",
                params: json!({"pane": self.focused_pane.unwrap_or(0), "url": "about:blank"}),
            }),
            ("Zoom/unzoom pane".into(), Cmd::Call {
                name: "zoom-pane",
                params: json!({"pane": self.focused_pane.unwrap_or(0)}),
            }),
        ];
        for (i, ws) in self.tree.workspaces.iter().enumerate() {
            items.push((format!("Go to workspace: {}", ws.name), Cmd::SelectWorkspace(i)));
        }
        items
    }

    pub(crate) fn filtered(&self) -> Vec<(String, Cmd)> {
        let q = self.palette_query.to_lowercase();
        self.palette_items()
            .into_iter()
            .filter(|(t, _)| t.to_lowercase().contains(&q))
            .collect()
    }

    fn palette_key(&mut self, e: &KeyDownEvent, cx: &mut Context<Self>) {
        match e.keystroke.key.as_str() {
            "escape" => self.palette_open = false,
            "enter" => {
                let items = self.filtered();
                if let Some((_, cmd)) = items.get(self.palette_sel) {
                    let cmd = cmd.clone();
                    self.run_cmd(cmd, cx);
                }
                self.palette_open = false;
            }
            "up" => self.palette_sel = self.palette_sel.saturating_sub(1),
            "down" => {
                let n = self.filtered().len();
                self.palette_sel = (self.palette_sel + 1).min(n.saturating_sub(1));
            }
            "backspace" => {
                self.palette_query.pop();
                self.palette_sel = 0;
            }
            _ => {
                if let Some(ch) = &e.keystroke.key_char {
                    if ch.len() == 1 && !e.keystroke.modifiers.control && !e.keystroke.modifiers.alt && !e.keystroke.modifiers.platform {
                        self.palette_query.push_str(ch);
                        self.palette_sel = 0;
                    }
                }
            }
        }
        cx.notify();
    }

    // ---- mouse ----
    pub(crate) fn on_mouse_move(&mut self, e: &MouseMoveEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.dragging_sidebar {
            let x: f32 = e.position.x.into();
            let w = x
                .clamp(theme::SIDEBAR_MIN, theme::SIDEBAR_MAX);
            self.sidebar_w = w;
            cx.notify();
        } else if let Some((split, rect, horiz)) = self.dragging_split {
            let ratio = if horiz {
                (f32::from(e.position.x) - f32::from(rect.origin.x)) / f32::from(rect.size.width)
            } else {
                (f32::from(e.position.y) - f32::from(rect.origin.y)) / f32::from(rect.size.height)
            };
            let r = ratio.clamp(0.05, 0.95) as f64;
            self.enqueue(Box::new(move |mux: &Mux, _ev: &EventTx| {
                let _ = mux
                    .control
                    .call("set-split-ratio", json!({"split": split, "ratio": r}));
            }));
        }
    }

    pub(crate) fn on_mouse_up(&mut self, _e: &MouseUpEvent, _w: &mut Window, cx: &mut Context<Self>) {
        if self.dragging_sidebar || self.dragging_split.is_some() {
            self.dragging_sidebar = false;
            self.dragging_split = None;
            self.refresh();
            cx.notify();
        }
    }
}


impl RootView {
    pub(crate) fn on_scroll(&mut self, e: &ScrollWheelEvent, _w: &mut Window, _cx: &mut Context<Self>) {
        let delta = match e.delta {
            ScrollDelta::Pixels(p) => f32::from(p.y),
            ScrollDelta::Lines(l) => l.y,
        };
        if let Some(surface) = self.focused_terminal_surface() {
            let d = if delta > 0.0 { -3 } else { 3 };
            self.enqueue(Box::new(move |mux: &Mux, _ev| {
                let _ = mux
                    .control
                    .call("scroll-surface", serde_json::json!({"surface": surface, "delta": d}));
            }));
        }
    }
}

fn named_key(ks: &Keystroke) -> Option<String> {
    if ks.key == "tab" && ks.modifiers.shift {
        let rest = [
            ks.modifiers.control.then_some("ctrl"),
            ks.modifiers.alt.then_some("alt"),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join("+");
        return Some(if rest.is_empty() {
            "backtab".into()
        } else {
            format!("{rest}+backtab")
        });
    }
    let base: &str = match ks.key.as_str() {
        "enter" => "enter",
        "backspace" => "backspace",
        "tab" => "tab",
        "escape" => "escape",
        "up" => "up",
        "down" => "down",
        "left" => "left",
        "right" => "right",
        "home" => "home",
        "end" => "end",
        "pageup" => "pageup",
        "pagedown" => "pagedown",
        "delete" => "delete",
        "insert" => "insert",
        k if k.starts_with('f')
            && k[1..]
                .parse::<u32>()
                .map(|n| (1..=24).contains(&n))
                .unwrap_or(false) =>
        {
            k
        }
        _ => return None,
    };
    let mods = [
        ks.modifiers.control.then_some("ctrl"),
        ks.modifiers.alt.then_some("alt"),
        ks.modifiers.shift.then_some("shift"),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("+");
    Some(if mods.is_empty() {
        base.to_string()
    } else {
        format!("{mods}+{base}")
    })
}
