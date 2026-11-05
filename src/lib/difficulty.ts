export interface DifficultyOption {
  id: string;
  key: string;
  depth: number;
  multipv: number;
  skill: number;
}

export const DIFFICULTIES: DifficultyOption[] = [
  { id: "beginner", key: "beginner", depth: 5, multipv: 6, skill: 0 },
  { id: "casual", key: "casual", depth: 8, multipv: 5, skill: 5 },
  {
    id: "intermediate",
    key: "intermediate",
    depth: 11,
    multipv: 4,
    skill: 10,
  },
  { id: "strong", key: "strong", depth: 15, multipv: 3, skill: 15 },
  { id: "master", key: "master", depth: 19, multipv: 2, skill: 20 },
  { id: "custom", key: "custom", depth: 11, multipv: 4, skill: 20 },
];

export const DIFFICULTY_ALIASES: Record<string, string> = {
  theater: "beginner",
  standard: "intermediate",
  coach: "master",
};

export function difficultyFor(id: string): DifficultyOption {
  return DIFFICULTIES.find((item) => item.id === id) ?? DIFFICULTIES[2];
}
