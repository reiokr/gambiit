import type { Settings } from "./types";
import {
  DIFFICULTIES,
  DIFFICULTY_ALIASES,
  difficultyFor,
} from "./difficulty";

const STORAGE_KEY = "gambiit.settings.v1";
const COLOR_KEY = "gambiit.humanColor.v1";

export const DEFAULT_SETTINGS: Settings = {
  model: "opencode-go/deepseek-v4.1-flash",
  persona: "Theater Master",
  uiLanguage: "en",
  agentLanguage: "en",
  agentLanguageName: "English",
  responseMode: "auto",
  stockfishDepth: 11,
  multipv: 4,
  difficulty: "intermediate",
  theme: "salon",
  sound: true,
  animations: true,
  fontScale: 1.2,
};

export function loadSettings(): Settings {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULT_SETTINGS };
    const stored = JSON.parse(raw) as Partial<Settings> & {
      language?: string;
      languageName?: string;
    };
    const merged: Settings = { ...DEFAULT_SETTINGS, ...stored };
    // Migrate the pre-split single-language setting.
    if (!stored.uiLanguage && stored.language) {
      merged.uiLanguage = stored.language;
    }
    if (!stored.agentLanguage && stored.language) {
      merged.agentLanguage = stored.language;
    }
    if (!stored.agentLanguageName && stored.languageName) {
      merged.agentLanguageName = stored.languageName;
    }
    const alias = DIFFICULTY_ALIASES[merged.difficulty];
    if (alias) {
      merged.difficulty = alias;
    }
    if (!DIFFICULTIES.some((level) => level.id === merged.difficulty)) {
      merged.difficulty = "intermediate";
    }
    const preset = difficultyFor(merged.difficulty);
    if (preset.id !== "custom") {
      merged.stockfishDepth = preset.depth;
      merged.multipv = preset.multipv;
    }
    return merged;
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
