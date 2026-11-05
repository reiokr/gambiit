import type { CustomTheme } from "./types";

export interface ThemeOption {
  id: string;
  name: string;
  description: string;
}

export const THEMES: ThemeOption[] = [
  {
    id: "salon",
    name: "Salon",
    description: "Walnut, cream and brass. The default stage.",
  },
  {
    id: "tournament",
    name: "Tournament",
    description: "Classic green and cream, made for long games.",
  },
  {
    id: "midnight",
    name: "Midnight",
    description: "Cold steel and blue light. Play after dark.",
  },
  {
    id: "custom",
    name: "Custom",
    description: "Your own palette, saved with the profile.",
  },
];

export const DEFAULT_CUSTOM_THEME: CustomTheme = {
  bg: "#14110e",
  panel: "#201a15",
  border: "#3d3126",
  text: "#efe6d5",
  muted: "#a89a86",
  accent: "#c9a227",
  boardLight: "#e8d3b0",
  boardDark: "#8b5a2b",
  boardBorder: "#3b2a1a",
};

const CUSTOM_PROPERTIES = [
  "--bg",
  "--bg-glow",
  "--panel",
  "--panel-2",
  "--panel-3",
  "--border",
  "--text",
  "--muted",
  "--faint",
  "--accent",
  "--accent-strong",
  "--accent-soft",
  "--board-light",
  "--board-dark",
  "--board-border",
  "--last",
  "--selected",
  "--legal",
  "--danger",
  "--success",
  "--shadow",
];

const HEX = /^#[0-9a-f]{6}$/i;

function color(value: unknown, fallback: string): string {
  return typeof value === "string" && HEX.test(value.trim())
    ? value.trim().toLowerCase()
    : fallback;
}

export function sanitizeCustomTheme(
  value: Partial<CustomTheme> | null | undefined,
): CustomTheme {
  const source = value ?? {};
  return {
    bg: color(source.bg, DEFAULT_CUSTOM_THEME.bg),
    panel: color(source.panel, DEFAULT_CUSTOM_THEME.panel),
    border: color(source.border, DEFAULT_CUSTOM_THEME.border),
    text: color(source.text, DEFAULT_CUSTOM_THEME.text),
    muted: color(source.muted, DEFAULT_CUSTOM_THEME.muted),
    accent: color(source.accent, DEFAULT_CUSTOM_THEME.accent),
    boardLight: color(source.boardLight, DEFAULT_CUSTOM_THEME.boardLight),
    boardDark: color(source.boardDark, DEFAULT_CUSTOM_THEME.boardDark),
    boardBorder: color(
      source.boardBorder,
      DEFAULT_CUSTOM_THEME.boardBorder,
    ),
  };
}

function clearCustom(root: HTMLElement): void {
  for (const property of CUSTOM_PROPERTIES) {
    root.style.removeProperty(property);
  }
}

function applyCustom(root: HTMLElement, palette: CustomTheme): void {
  const set = (name: string, value: string) =>
    root.style.setProperty(name, value);
  set("--bg", palette.bg);
  set(
    "--bg-glow",
    "color-mix(in srgb, var(--bg) 76%, var(--accent) 24%)",
  );
  set("--panel", palette.panel);
  set(
    "--panel-2",
    "color-mix(in srgb, var(--panel) 93%, var(--text) 7%)",
  );
  set(
    "--panel-3",
    "color-mix(in srgb, var(--panel) 85%, var(--text) 15%)",
  );
  set("--border", palette.border);
  set("--text", palette.text);
  set("--muted", palette.muted);
  set(
    "--faint",
    "color-mix(in srgb, var(--muted) 74%, var(--bg) 26%)",
  );
  set("--accent", palette.accent);
  set(
    "--accent-strong",
    "color-mix(in srgb, var(--accent) 78%, white 22%)",
  );
  set(
    "--accent-soft",
    "color-mix(in srgb, var(--accent) 15%, transparent)",
  );
  set("--board-light", palette.boardLight);
  set("--board-dark", palette.boardDark);
  set("--board-border", palette.boardBorder);
  set(
    "--last",
    "color-mix(in srgb, var(--accent) 34%, transparent)",
  );
  set(
    "--selected",
    "color-mix(in srgb, var(--accent) 46%, transparent)",
  );
  set("--legal", "#2f6b45");
  set("--danger", "#c94a3f");
  set("--success", "#5f9e6a");
  set("--shadow", "0 24px 60px rgba(0, 0, 0, 0.45)");
}

export function applyTheme(theme: string, custom?: CustomTheme): void {
  const root = document.documentElement;
  const id = theme || "salon";
  root.dataset.theme = id;
  if (id === "custom") {
    applyCustom(root, sanitizeCustomTheme(custom));
  } else {
    clearCustom(root);
  }
}
