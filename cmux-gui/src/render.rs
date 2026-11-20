// Rendering for RootView: titlebar, sidebar, split tree, panes, palette.
use crate::app::{Cmd, RootView};
use crate::model::*;
use crate::mux::Mux;
use crate::terminal;
use crate::theme;
use gpui::*;
use serde_json::json;
use std::collections::HashMap;

impl RootView {
    fn titlebar(&self, ws_name: &str) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(theme::CHROME_BAR))
            .bg(theme::TITLEBAR_BG)
            .border_b_1()
            .border_color(theme::DIVIDER)
            .text_xs()
            .text_color(theme::TEXT_DIM)
            .child(ws_name.to_string())
    }

    fn palette(&self) -> impl IntoElement {
        let items = self.filtered();
        let rows: Vec<AnyElement> = items
            .iter()
            .enumerate()
            .map(|(i, (title, _))| {
                div()
                    .px_2()
                    .py_1()
                    .text_color(if i == self.palette_sel {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    })
                    .bg(if i == self.palette_sel {
                        theme::HOVER
                    } else {
                        theme::ACTIVE
                    })
                    .child(title.clone())
                    .into_any_element()
            })
            .collect();
        div()
            .absolute()
            .top(px(theme::CHROME_BAR + 8.))
            .left_1_2()
            .ml(px(-220.))
            .w(px(440.))
            .bg(theme::ACTIVE)
            .border_1()
            .border_color(theme::DIVIDER)
            .rounded_md()
            .shadow_lg()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(theme::DIVIDER)
                    .text_color(theme::TEXT)
                    .child(if self.palette_query.is_empty() {
                        "Type a command…".to_string()
                    } else {
                        self.palette_query.clone()
                    }),
            )
            .children(rows)
    }

    fn sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows: Vec<AnyElement> = self
            .tree
            .workspaces
            .iter()
            .enumerate()
            .map(|(i, ws)| self.workspace_row(i, ws, cx).into_any_element())
            .collect();
        div()
            .relative()
            .w(px(self.sidebar_w))
            .flex_shrink()
            .child(
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .h_full()
                    .bg(theme::SIDEBAR_BG)
                    .border_r_1()
                    .border_color(theme::DIVIDER)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .h(px(theme::CHROME_BAR))
                            .px(px(10.))
                            .text_color(theme::TEXT_FAINT)
                            .text_xs()
                            .font_weight(FontWeight::BOLD)
                            .child("WORKSPACES")
                            .child(div().flex_1())
                            .child(
                                div()
                                    .id("new-ws")
                                    .cursor_pointer()
                                    .text_color(theme::TEXT_DIM)
                                    .child("+")
                                    .on_click(cx.listener(|this, _e, _w, cx| {
                                        this.run_cmd(Cmd::NewWorkspace, cx);
                                    })),
                            ),
                    )
                    .child(div().flex_1().p_1().children(rows)),
            )
            .child(
                // 6px drag handle over the sidebar's right edge.
                div()
                    .id("sidebar-drag")
                    .absolute()
                    .top_0()
                    .right(px(-3.))
                    .w(px(6.))
                    .h_full()
                    .cursor(CursorStyle::ResizeLeftRight)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|this, _e, _w, cx| {
                            this.dragging_sidebar = true;
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    ),
            )
    }

    fn workspace_row(
        &self,
        index: usize,
        ws: &Workspace,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let unread = ws.unread;
        let active = ws.active;
        let key_label = ws
            .key
            .chars()
            .next()
            .unwrap_or('?')
            .to_ascii_uppercase()
            .to_string();
        div()
            .id(SharedString::from(format!("ws-{}", ws.id)))
            .h(px(theme::ROW_H))
            .rounded(px(6.))
            .bg(if active {
                theme::ACTIVE
            } else {
                gpui::transparent_black().into()
            })
            .hover(|s| s.bg(theme::HOVER))
            .flex()
            .items_center()
            .px(px(10.))
            .mb(px(1.))
            .on_click(cx.listener(move |this, _e, _w, cx| {
                this.run_cmd(Cmd::SelectWorkspace(index), cx);
            }))
            .child(
                div()
                    .w(px(16.))
                    .h(px(16.))
                    .rounded_full()
                    .border_1()
                    .border_color(if unread > 0 {
                        theme::ACCENT
                    } else {
                        theme::TEXT_FAINT
                    })
                    .text_color(if unread > 0 {
                        theme::ACCENT
                    } else {
                        theme::TEXT_FAINT
                    })
                    .text_xs()
                    .flex()
                    .items_center()
                    .justify_center()
                    .mr_2()
                    .child(if unread > 0 {
                        unread.to_string()
                    } else {
                        key_label
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .text_color(if active {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    })
                    .font_weight(if active {
                        FontWeight::SEMIBOLD
                    } else {
                        FontWeight::NORMAL
                    })
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(ws.name.clone()),
            )
    }

    fn main_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let content: AnyElement = match self.active_screen() {
            Some(s) => self.screen_view(s, cx).into_any_element(),
            None => div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(theme::TEXT_FAINT)
                .child(if self.tree.workspaces.is_empty() {
                    "Connecting to cmux…"
                } else {
                    "No screens"
                })
                .into_any_element(),
        };
        div()
            .flex()
            .flex_1()
            .min_h(px(0.))
            .min_w(px(0.))
            .child(self.sidebar(cx))
            .child(content)
    }

    fn screen_view(&self, screen: &Screen, cx: &mut Context<Self>) -> impl IntoElement {
        let zoomed = screen
            .zoomed_pane
            .and_then(|id| screen.panes.iter().find(|p| p.id == id));
        let body: AnyElement = match zoomed {
            Some(pane) => div()
                .flex()
                .flex_1()
                .min_w(px(0.))
                .min_h(px(0.))
                .child(self.pane_view(pane, cx))
                .into_any_element(),
            None => div()
                .flex()
                .flex_1()
                .min_w(px(0.))
                .min_h(px(0.))
                .child(self.split_node(&screen.layout, &screen.panes, cx))
                .into_any_element(),
        };
        div().flex().flex_1().min_w(px(0.)).min_h(px(0.)).child(body)
    }

    fn split_node(
        &self,
        node: &SplitNode,
        panes: &[Pane],
        cx: &mut Context<Self>,
    ) -> AnyElement {
        if node.kind == "stack" {
            let pane = node
                .expanded
                .and_then(|id| panes.iter().find(|p| p.id == id))
                .or_else(|| panes.first());
            return match pane {
                Some(p) => div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .child(self.pane_view(p, cx))
                    .into_any_element(),
                None => div().flex_1().into_any_element(),
            };
        }
        let (Some(a_node), Some(b_node)) = (node.a.as_deref(), node.b.as_deref()) else {
            let pane = node
                .pane
                .and_then(|id| panes.iter().find(|p| p.id == id))
                .or_else(|| panes.first());
            return match pane {
                Some(p) => div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .child(self.pane_view(p, cx))
                    .into_any_element(),
                None => div().flex_1().into_any_element(),
            };
        };
        if node.kind != "split" {
            return match panes.first() {
                Some(p) => div()
                    .flex()
                    .flex_1()
                    .min_w(px(0.))
                    .min_h(px(0.))
                    .child(self.pane_view(p, cx))
                    .into_any_element(),
                None => div().flex_1().into_any_element(),
            };
        }
        let horiz = node.dir.as_deref() != Some("down");
        let ratio = node.ratio.unwrap_or(0.5);
        let split_id = node.split.unwrap_or(0);
        let a = self.split_node(a_node, panes, cx);
        let b = self.split_node(b_node, panes, cx);

        let first: AnyElement = {
            let d = div().flex().min_w(px(0.)).min_h(px(0.)).child(a);
            if horiz {
                d.w(relative(ratio)).into_any_element()
            } else {
                d.h(relative(ratio)).into_any_element()
            }
        };
        let divider: AnyElement = {
            let d = div()
                .id(SharedString::from(format!("split-{split_id}")))
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _e, w, _cx| {
                        // Approximate the container with the window content
                        // rect; ratio math only needs a bounds box.
                        this.dragging_split = Some((
                            split_id,
                            Bounds {
                                origin: point(px(0.), px(theme::CHROME_BAR)),
                                size: w.viewport_size(),
                            },
                            horiz,
                        ));
                    }),
                );
            let d = if horiz {
                d.w(px(9.))
                    .h_full()
                    .ml(px(-4.))
                    .mr(px(-4.))
                    .cursor(CursorStyle::ResizeLeftRight)
                    .child(div().w(px(1.)).h_full().bg(theme::DIVIDER))
            } else {
                d.h(px(9.))
                    .w_full()
                    .mt(px(-4.))
                    .mb(px(-4.))
                    .cursor(CursorStyle::ResizeUpDown)
                    .child(div().h(px(1.)).w_full().bg(theme::DIVIDER))
            };
            d.into_any_element()
        };
        let second: AnyElement = div()
            .flex()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .child(b)
            .into_any_element();

        let outer = div().flex().flex_1().min_w(px(0.)).min_h(px(0.));
        if horiz {
            outer
                .flex_row()
                .child(first)
                .child(divider)
                .child(second)
                .into_any_element()
        } else {
            outer
                .flex_col()
                .child(first)
                .child(divider)
                .child(second)
                .into_any_element()
        }
    }

    fn pane_view(&self, pane: &Pane, cx: &mut Context<Self>) -> impl IntoElement {
        let focused = self.focused_pane == Some(pane.id);
        let unread: u64 = pane.tabs.iter().map(|t| t.unread()).sum();
        let pane_id = pane.id;
        let body: AnyElement = match pane.tabs.get(pane.active_tab) {
            Some(tab) if tab.kind == "browser" => {
                self.browser_tab(tab).into_any_element()
            }
            Some(tab) => self.terminal_tab(tab).into_any_element(),
            None => div().flex_1().into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.))
            .min_h(px(0.))
            .border_1()
            .border_color(if unread > 0 && !focused {
                theme::ACCENT
            } else {
                theme::DIVIDER
            })
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _e, _w, cx| {
                    if this.focused_pane != Some(pane_id) {
                        this.focused_pane = Some(pane_id);
                        this.call("focus-pane", json!({"pane": pane_id}));
                        cx.notify();
                    }
                }),
            )
            .child(self.tab_strip(pane, cx))
            .child(body)
    }

    fn tab_strip(&self, pane: &Pane, cx: &mut Context<Self>) -> impl IntoElement {
        let pane_id = pane.id;
        let tabs: Vec<AnyElement> = pane
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let active = i == pane.active_tab;
                let unread = tab.unread();
                let mut d = div()
                    .id(SharedString::from(format!("tab-{pane_id}-{i}")))
                    .flex()
                    .items_center()
                    .h_full()
                    .px_2()
                    .border_r_1()
                    .border_color(theme::DIVIDER)
                    .bg(if active {
                        theme::TERM_BG
                    } else {
                        theme::PANE_HEADER_BG
                    })
                    .text_xs()
                    .text_color(if active {
                        theme::TEXT
                    } else {
                        theme::TEXT_DIM
                    })
                    .on_click(cx.listener(move |this, _e, _w, _cx| {
                        this.call("select-tab", json!({"pane": pane_id, "index": i}));
                    }))
                    .child(tab.title.clone());
                if unread > 0 {
                    d = d.child(
                        div()
                            .ml_1()
                            .px_1()
                            .rounded_full()
                            .bg(theme::ACCENT)
                            .text_color(gpui::white())
                            .child(unread.to_string()),
                    );
                }
                d.into_any_element()
            })
            .collect();
        div()
            .flex()
            .items_center()
            .h(px(theme::CHROME_BAR))
            .bg(theme::PANE_HEADER_BG)
            .border_b_1()
            .border_color(theme::DIVIDER)
            .children(tabs)
            .child(
                div()
                    .id(SharedString::from(format!("newtab-{pane_id}")))
                    .cursor_pointer()
                    .px_1()
                    .text_color(theme::TEXT_DIM)
                    .child("+")
                    .on_click(cx.listener(move |this, _e, _w, _cx| {
                        this.call("new-tab", json!({"pane": pane_id}));
                    })),
            )
    }

    fn terminal_tab(&self, tab: &Tab) -> impl IntoElement {
        let surface = tab.surface;
        let sizes = self.reported_sizes.clone();
        let tx = self.call_tx.clone();
        let report = std::rc::Rc::new(move |cols: u32, rows: u32| {
            if sizes.borrow_mut().insert(surface, (cols, rows)) != Some((cols, rows)) {
                let _ = tx.send(Box::new(move |mux: &Mux, _ev| {
                    mux.resize_attached(surface, cols, rows);
                }));
            }
        });
        div().flex().flex_1().bg(theme::TERM_BG).child(
            terminal::element(
                self.surfaces.get(&tab.surface).cloned().unwrap_or_default(),
                self.cell_w,
                tab.surface,
                report,
            ),
        )
    }

    fn browser_tab(&self, tab: &Tab) -> impl IntoElement {
        let url = self
            .browser_urls
            .get(&tab.surface)
            .cloned()
            .unwrap_or_default();
        let label = if url.is_empty() {
            "about:blank".to_string()
        } else {
            url.clone()
        };
        div()
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .bg(theme::TERM_BG)
            .text_color(theme::TEXT_DIM)
            .gap_2()
            .child(format!("Browser pane: {label}"))
            .child(
                div()
                    .id(SharedString::from(format!("open-url-{}", tab.surface)))
                    .px_2()
                    .py_1()
                    .rounded_md()
                    .bg(theme::ACTIVE)
                    .cursor_pointer()
                    .child("Open in browser")
                    .on_click(move |_e, _w, cx| {
                        if !url.is_empty() {
                            cx.open_url(&url);
                        }
                    }),
            )
    }
}

impl Render for RootView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Attach/detach surfaces to match what's visible this frame.
        // PTYs attach in render mode; browser tabs attach in default mode
        // for their browser-state (URL) stream.
        let mut now_attached: HashMap<u64, bool> = HashMap::new();
        if let Some(s) = self.active_screen() {
            for p in &s.panes {
                for t in &p.tabs {
                    if t.kind == "browser" {
                        now_attached.insert(t.surface, true);
                    } else if t.kind == "pty" || t.kind.is_empty() {
                        now_attached.insert(t.surface, false);
                    }
                }
            }
        }
        for (surface, browser) in &now_attached {
            if self.attached.get(surface) == Some(browser) {
                continue;
            }
            let (surface, browser) = (*surface, *browser);
            self.enqueue(Box::new(move |mux: &Mux, _ev| {
                if browser {
                    mux.attach_browser(surface);
                } else {
                    mux.attach_render(surface);
                }
            }));
        }
        for surface in self.attached.keys() {
            if !now_attached.contains_key(surface) {
                let surface = *surface;
                // The new attach connection must re-report its grid even
                // when the remembered size is unchanged.
                self.reported_sizes.borrow_mut().remove(&surface);
                self.enqueue(Box::new(move |mux: &Mux, _ev| mux.detach(surface)));
            }
        }
        self.attached = now_attached;

        let ws_name = self
            .active_workspace()
            .map(|w| w.name.clone())
            .unwrap_or_else(|| "cmux".into());
        let content: AnyElement = if let Some(fatal) = &self.fatal {
            div()
                .flex()
                .size_full()
                .items_center()
                .justify_center()
                .text_color(theme::TEXT_DIM)
                .child(format!("cmux: {fatal}"))
                .into_any_element()
        } else {
            self.main_area(cx).into_any_element()
        };
        let mut root = div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme::WINDOW_BG)
            .text_color(theme::TEXT)
            .text_sm()
            .track_focus(&self.focus)
            .key_context("cmux")
            .on_key_down(cx.listener(Self::on_key_down))
            .on_mouse_move(cx.listener(Self::on_mouse_move))
            .on_mouse_up(MouseButton::Left, cx.listener(Self::on_mouse_up))
            .on_scroll_wheel(cx.listener(Self::on_scroll))
            .child(self.titlebar(&ws_name))
            .child(content);
        if self.palette_open {
            root = root.child(self.palette());
        }
        root
    }
}
