export interface TimeControlOption {
  id: string;
  key: string;
  initialMs: number;
  incrementMs: number;
}

export const TIME_CONTROLS: TimeControlOption[] = [
  { id: "bullet", key: "bullet", initialMs: 60_000, incrementMs: 0 },
  { id: "blitz", key: "blitz", initialMs: 180_000, incrementMs: 2_000 },
  { id: "rapid", key: "rapid", initialMs: 600_000, incrementMs: 5_000 },
  {
    id: "classical",
    key: "classical",
    initialMs: 1_800_000,
    incrementMs: 0,
  },
  { id: "custom", key: "custom", initialMs: 600_000, incrementMs: 5_000 },
];

export function timeControlById(id: string): TimeControlOption {
  return TIME_CONTROLS.find((option) => option.id === id) ?? TIME_CONTROLS[2];
}

export function formatClock(ms: number): string {
  const clamped = Math.max(0, ms);
  if (clamped < 20_000) {
    const tenths = Math.ceil(clamped / 100) / 10;
    const minutes = Math.floor(tenths / 60);
    const seconds = tenths - minutes * 60;
    return `${minutes}:${seconds.toFixed(1).padStart(4, "0")}`;
  }
  const totalSeconds = Math.ceil(clamped / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${String(seconds).padStart(2, "0")}`;
}

export function clockLabel(
  initialMs: number,
  incrementMs: number,
): string {
  const totalSeconds = Math.max(0, Math.round(initialMs / 1000));
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  const base =
    seconds === 0
      ? `${minutes}`
      : `${minutes}:${String(seconds).padStart(2, "0")}`;
  return `${base} + ${Math.round(incrementMs / 1000)}`;
}
