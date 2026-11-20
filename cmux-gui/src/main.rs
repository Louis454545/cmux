mod app;
mod model;
mod mux;
mod render;
mod terminal;
mod theme;

use app::*;
use gpui::*;

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("cmd-k", TogglePalette, None),
            KeyBinding::new("ctrl-k", TogglePalette, None),
            KeyBinding::new("cmd-d", SplitDown, None),
            KeyBinding::new("ctrl-d", SplitDown, None),
            KeyBinding::new("cmd-shift-d", SplitRight, None),
            KeyBinding::new("ctrl-shift-d", SplitRight, None),
            KeyBinding::new("cmd-t", NewTab, None),
            KeyBinding::new("ctrl-t", NewTab, None),
            KeyBinding::new("cmd-w", CloseTab, None),
            KeyBinding::new("ctrl-w", CloseTab, None),
        ]);
        let bounds = Bounds::centered(None, size(px(1440.), px(900.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some("cmux".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(point(px(8.), px(6.))),
                }),
                app_id: Some("com.cmuxterm.gui".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| RootView::new(window, cx)),
        )
        .unwrap();
        cx.on_action(|_: &TogglePalette, cx| toggle_palette(cx));
        cx.on_action(|_: &SplitDown, cx| pane_action(cx, "new-pane"));
        cx.on_action(|_: &SplitRight, cx| pane_action(cx, "new-pane-right"));
        cx.on_action(|_: &NewTab, cx| pane_action(cx, "new-tab"));
        cx.on_action(|_: &NewBrowserTab, cx| {
            pane_action_params(cx, "new-browser-tab", "about:blank")
        });
        cx.on_action(|_: &CloseTab, cx| close_tab(cx));
        cx.on_action(|_: &ZoomPane, cx| pane_action(cx, "zoom-pane"));
        cx.on_action(|_: &NewWorkspace, cx| {
            with_view(cx, |v, cx| v.run_cmd(crate::app::Cmd::NewWorkspace, cx));
        });
        cx.activate(true);
    });
}

fn with_view(cx: &mut App, f: impl FnOnce(&mut RootView, &mut Context<RootView>)) {
    if let Some(w) = cx.active_window().and_then(|w| w.downcast::<RootView>()) {
        let _ = w.update(cx, |v, _w, cx| f(v, cx));
    }
}

fn toggle_palette(cx: &mut App) {
    with_view(cx, |v, cx| {
        v.palette_open = !v.palette_open;
        if v.palette_open {
            v.palette_query.clear();
            v.palette_sel = 0;
        }
        cx.notify();
    });
}

fn pane_action(cx: &mut App, name: &'static str) {
    with_view(cx, |v, _cx| v.with_pane(name));
}

fn pane_action_params(cx: &mut App, name: &'static str, url: &str) {
    with_view(cx, |v, _cx| {
        if let Some(p) = v.focused_pane {
            v.call(name, serde_json::json!({"pane": p, "url": url}));
        }
    });
}

fn close_tab(cx: &mut App) {
    with_view(cx, |v, _cx| {
        if let Some(p) = v.focused_pane {
            if let Some(pane) = v.all_panes().into_iter().find(|x| x.id == p) {
                if let Some(tab) = pane.tabs.get(pane.active_tab) {
                    v.call(
                        "close-surface",
                        serde_json::json!({"surface": tab.surface}),
                    );
                }
            }
        }
    });
}
