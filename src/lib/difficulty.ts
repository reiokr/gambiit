export interface DifficultyOption {
  id: string;
  key: string;
  depth: number;
  multipv: number;
}

export const DIFFICULTIES: DifficultyOption[] = [
  { id: "theater", key: "theater", depth: 0, multipv: 0 },
  { id: "casual", key: "casual", depth: 8, multipv: 5 },
  { id: "standard", key: "standard", depth: 12, multipv: 3 },
  { id: "strong", key: "strong", depth: 16, multipv: 2 },
  { id: "coach", key: "coach", depth: 18, multipv: 1 },
  { id: "custom", key: "custom", depth: 12, multipv: 3 },
];

export function difficultyFor(id: string): DifficultyOption {
  return DIFFICULTIES.find((item) => item.id === id) ?? DIFFICULTIES[2];
}
