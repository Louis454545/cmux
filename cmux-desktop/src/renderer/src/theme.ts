// Design tokens extracted from the macOS app (Sources/, WindowChromeMetrics,
// CmuxAccentColor) and Ghostty defaults.
export const theme = {
  // Ghostty default terminal theme
  termBg: "#282c34",
  termFg: "#ffffff",
  palette: [
    "#1d1f21", "#cc6666", "#b5bd68", "#f0c674",
    "#81a2be", "#b294bb", "#8abeb7", "#c5c8c6",
    "#666666", "#d54e53", "#b9ca4a", "#e7c547",
    "#7aa6da", "#c397d8", "#70c0b1", "#eaeaea",
  ],
  // Chrome — from screenshots + sidebar bg (~#1d1f21 tinted by titlebar blur)
  windowBg: "#1d1f21",
  titlebarBg: "#1d1f21",
  sidebarBg: "#1f2023",
  paneHeaderBg: "#25262b",
  paneHeaderActiveBg: "#2b2c31",
  divider: "#101114",
  hover: "rgba(255,255,255,0.06)",
  active: "rgba(255,255,255,0.10)",
  text: "#e6e6e6",
  textDim: "#9a9a9a",
  textFaint: "#6e6e6e",
  // cmux accent (CmuxAccentColor.cmuxBlue, dark)
  accent: "#0091ff",
  accentDim: "rgba(0,145,255,0.35)",
  // Metrics (WindowChromeMetrics)
  chromeBar: 28,
  sidebarWidth: 240,
  sidebarMin: 120,
  sidebarMax: 260,
  rowHeight: 28,
  headerBtn: 20,
  headerIcon: 12,
  radius: 6,
  fontUI:
    '-apple-system, "Inter", "SF Pro Text", "Segoe UI", system-ui, sans-serif',
  fontMono: '"JetBrains Mono", "SF Mono", Menlo, Consolas, monospace',
} as const;
