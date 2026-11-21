import React, { useEffect, useRef, useState } from "react";
import { theme } from "./theme";
import type { Tab } from "./protocol";

// Browser panes are real web pages served by the cmux-browser CDP provider.
// In the desktop app we embed them in a <webview> tag (enabled in main).
export default function BrowserTab({ tab }: { tab: Tab }) {
  const ref = useRef<any>(null);
  const [url, setUrl] = useState(tab.browser_source ?? "about:blank");
  const [input, setInput] = useState(url);

  useEffect(() => {
    const wv = ref.current;
    if (!wv) return;
    const onNav = (e: any) => {
      if (e.url) {
        setUrl(e.url);
        setInput(e.url);
      }
    };
    wv.addEventListener("did-navigate", onNav);
    wv.addEventListener("did-navigate-in-page", onNav);
    return () => {
      wv.removeEventListener("did-navigate", onNav);
      wv.removeEventListener("did-navigate-in-page", onNav);
    };
  }, []);

  const go = (e: React.FormEvent) => {
    e.preventDefault();
    let u = input.trim();
    if (!u) return;
    if (!/^https?:\/\//.test(u) && !u.startsWith("about:")) u = `https://${u}`;
    setUrl(u);
  };

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <form
        onSubmit={go}
        style={{
          height: 28,
          display: "flex",
          alignItems: "center",
          padding: "0 8px",
          background: theme.paneHeaderBg,
          borderBottom: `1px solid ${theme.divider}`,
          gap: 6,
        }}
      >
        <input
          value={input}
          onChange={(e) => setInput(e.target.value)}
          style={{
            flex: 1,
            height: 20,
            background: "rgba(255,255,255,0.06)",
            border: "none",
            borderRadius: 5,
            color: theme.text,
            fontSize: 12,
            padding: "0 8px",
            outline: "none",
            fontFamily: theme.fontUI,
          }}
        />
      </form>
      {/* webview is an Electron element */}
      <webview
        ref={ref}
        src={url}
        style={{ flex: 1, border: "none" }}
        // @ts-expect-error webview attribute
        allowpopups="true"
      />
    </div>
  );
}
