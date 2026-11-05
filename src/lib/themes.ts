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
];

export function applyTheme(theme: string): void {
  document.documentElement.dataset.theme = theme || "salon";
}
