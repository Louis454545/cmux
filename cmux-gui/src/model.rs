// Wire types + render model for cmux.protocol/2.
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Tab {
    pub surface: u64,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub browser_source: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub dead: bool,
    #[serde(default)]
    pub notification: Option<Value>,
}

impl Tab {
    pub fn unread(&self) -> u64 {
        self.notification
            .as_ref()
            .and_then(|n| n.get("notification").and_then(|v| v.as_u64()))
            .unwrap_or(0)
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Pane {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub active_tab: usize,
    #[serde(default)]
    pub tabs: Vec<Tab>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SplitNode {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub pane: Option<u64>,
    #[serde(default)]
    pub split: Option<u64>,
    #[serde(default)]
    pub dir: Option<String>, // "right" | "down"
    #[serde(default)]
    pub ratio: Option<f32>,
    #[serde(default)]
    pub a: Option<Box<SplitNode>>,
    #[serde(default)]
    pub b: Option<Box<SplitNode>>,
    #[serde(default)]
    pub panes: Option<Vec<u64>>,
    #[serde(default)]
    pub expanded: Option<u64>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Screen {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub active_pane: u64,
    #[serde(default)]
    pub zoomed_pane: Option<u64>,
    pub layout: SplitNode,
    #[serde(default)]
    pub panes: Vec<Pane>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Workspace {
    pub id: u64,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub unread: u64,
    #[serde(default)]
    pub screens: Vec<Screen>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Tree {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}

// ---------- render-mode surface model ----------
#[derive(Clone, Debug)]
pub struct Run {
    pub text: String,
    pub fg: Option<String>,
    pub bg: Option<String>,
    pub attrs: u64,
    pub underline: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CellRow {
    pub row: u32,
    pub runs: Vec<Run>,
}

#[derive(Clone, Debug)]
pub struct Cursor {
    pub x: u32,
    pub y: u32,
    pub style: String,
    pub visible: bool,
    pub color: Option<String>,
}

impl Default for Cursor {
    fn default() -> Self {
        Cursor { x: 0, y: 0, style: "block".into(), visible: true, color: None }
    }
}

#[derive(Clone, Debug)]
pub struct SurfaceModel {
    pub cols: u32,
    pub rows: u32,
    pub cells: Vec<CellRow>,
    pub cursor: Cursor,
    pub default_fg: String,
    pub default_bg: String,
    pub scrollback_rows: u64,
    pub offset: i64,
}

impl Default for SurfaceModel {
    fn default() -> Self {
        SurfaceModel {
            cols: 80,
            rows: 24,
            cells: vec![],
            cursor: Cursor::default(),
            default_fg: "#ffffff".into(),
            default_bg: "#282c34".into(),
            scrollback_rows: 0,
            offset: 0,
        }
    }
}

fn parse_runs(rows: &Value) -> Vec<CellRow> {
    rows.as_array()
        .map(|arr| {
            arr.iter()
                .map(|r| CellRow {
                    row: r.get("row").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                    runs: r
                        .get("runs")
                        .and_then(|v| v.as_array())
                        .map(|runs| {
                            runs.iter()
                                .map(|run| Run {
                                    text: run
                                        .get("text")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    fg: run
                                        .get("fg")
                                        .and_then(|v| v.as_str())
                                        .map(String::from),
                                    bg: run
                                        .get("bg")
                                        .and_then(|v| v.as_str())
                                        .map(String::from),
                                    attrs: run.get("attrs").and_then(|v| v.as_u64()).unwrap_or(0),
                                    underline: run
                                        .get("underline")
                                        .and_then(|v| v.as_str())
                                        .map(String::from),
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn parse_cursor(e: &Value) -> Cursor {
    let c = e.get("cursor").cloned().unwrap_or(json_empty());
    Cursor {
        x: c.get("x").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        y: c.get("y").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
        style: c.get("style").and_then(|v| v.as_str()).unwrap_or("block").into(),
        visible: c.get("visible").and_then(|v| v.as_bool()).unwrap_or(false),
        color: c.get("color").and_then(|v| v.as_str()).map(String::from),
    }
}

fn json_empty() -> Value {
    Value::Object(serde_json::Map::new())
}

/// Applies a `render-state` or `render-delta` frame to the model.
pub fn apply_render_event(model: &mut SurfaceModel, e: &Value) {
    let event = e.get("event").and_then(|v| v.as_str()).unwrap_or("");
    if event == "render-state" {
        model.cols = e.pointer("/size/cols").and_then(|v| v.as_u64()).unwrap_or(80) as u32;
        model.rows = e.pointer("/size/rows").and_then(|v| v.as_u64()).unwrap_or(24) as u32;
        model.cells = parse_runs(&e["rows"]);
        model.cursor = parse_cursor(e);
        if let Some(fg) = e.get("default_fg").and_then(|v| v.as_str()) {
            model.default_fg = fg.into();
        }
        if let Some(bg) = e.get("default_bg").and_then(|v| v.as_str()) {
            model.default_bg = bg.into();
        }
        model.scrollback_rows =
            e.get("scrollback_rows").and_then(|v| v.as_u64()).unwrap_or(0);
        return;
    }
    // A resize arrives as a full delta carrying the new `size`; apply it
    // before merging so grown bottom rows are not dropped.
    if let Some(n) = e.pointer("/size/cols").and_then(|v| v.as_u64()) {
        model.cols = n as u32;
    }
    if let Some(n) = e.pointer("/size/rows").and_then(|v| v.as_u64()) {
        model.rows = n as u32;
    }
    let full = e.get("full").and_then(|v| v.as_bool()).unwrap_or(false);
    if full {
        model.cells = parse_runs(&e["rows"]);
    } else {
        // Merge delta rows by index, filling sparse gaps up to `rows`.
        let mut by_row: HashMap<u32, CellRow> =
            model.cells.drain(..).map(|r| (r.row, r)).collect();
        for upd in parse_runs(&e["rows"]) {
            by_row.insert(upd.row, upd);
        }
        model.cells = (0..model.rows)
            .filter_map(|i| by_row.remove(&i))
            .collect();
    }
    model.cursor = parse_cursor(e);
    if let Some(fg) = e.get("default_fg").and_then(|v| v.as_str()) {
        model.default_fg = fg.into();
    }
    if let Some(bg) = e.get("default_bg").and_then(|v| v.as_str()) {
        model.default_bg = bg.into();
    }
    if let Some(n) = e.get("scrollback_rows").and_then(|v| v.as_u64()) {
        model.scrollback_rows = n;
    }
}
