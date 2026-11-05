import type { Settings } from "./types";

const STORAGE_KEY = "gambiit.settings.v1";
const COLOR_KEY = "gambiit.humanColor.v1";

export const DEFAULT_SETTINGS: Settings = {
  model: "opencode-go/deepseek-v4.1-flash",
  persona: "Theater Master",
  language: "en",
  responseMode: "auto",
  stockfishDepth: 12,
  multipv: 3,
  theme: "salon",
  sound: true,
  animations: true,
  fontScale: 1.2,
};

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    return { ...DEFAULT_SETTINGS, ...JSON.parse(raw) };
  } catch {
    return { ...DEFAULT_SETTINGS };
  }
}

export function saveSettings(settings: Settings): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Ignore storage failures (private mode, quota).
  }
}

export function applyFontScale(scale: number): void {
  const value = Number.isFinite(scale)
    ? Math.min(1.5, Math.max(0.9, scale))
    : 1.2;
  document.documentElement.style.setProperty("--font-scale", String(value));
}

export function loadHumanColor(): "white" | "black" {
  try {
    const value = localStorage.getItem(COLOR_KEY);
    return value === "black" ? "black" : "white";
  } catch {
    return "white";
  }
}

export function saveHumanColor(color: "white" | "black"): void {
  try {
    localStorage.setItem(COLOR_KEY, color);
  } catch {
    // Ignore.
  }
}
