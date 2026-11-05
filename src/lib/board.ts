import type { ColorKey, PieceView, SquareView } from "./types";

export const FILES = ["a", "b", "c", "d", "e", "f", "g", "h"];
export const RANKS = [8, 7, 6, 5, 4, 3, 2, 1];

const VALUES: Record<string, number> = {
  p: 1,
  n: 3,
  b: 3,
  r: 5,
  q: 9,
  k: 0,
};

const INITIAL: Record<string, number> = {
  p: 8,
  n: 2,
  b: 2,
  r: 2,
  q: 1,
  k: 1,
};

export function orderedSquares(flipped: boolean): string[] {
  const files = flipped ? [...FILES].reverse() : FILES;
  const ranks = flipped ? [...RANKS].reverse() : RANKS;
  const squares: string[] = [];
  for (const rank of ranks) {
    for (const file of files) {
      squares.push(`${file}${rank}`);
    }
  }
  return squares;
}

export function boardMap(
  board: SquareView[],
): Map<string, PieceView | null> {
  return new Map(board.map((square) => [square.square, square.piece]));
}

export function isLightSquare(square: string): boolean {
  const file = FILES.indexOf(square[0]);
  const rank = Number.parseInt(square[1], 10) - 1;
  return (file + rank) % 2 === 1;
}

export function materialScore(board: SquareView[]): {
  white: number;
  black: number;
  diff: number;
} {
  let white = 0;
  let black = 0;
  for (const square of board) {
    if (!square.piece) continue;
    const value = VALUES[square.piece.role] ?? 0;
    if (square.piece.color === "white") white += value;
    else black += value;
  }
  return { white, black, diff: white - black };
}

function missingPieces(board: SquareView[], color: ColorKey): PieceView[] {
  const counts: Record<string, number> = {
    p: 0,
    n: 0,
    b: 0,
    r: 0,
    q: 0,
    k: 0,
  };
  for (const square of board) {
    if (square.piece?.color === color) {
      counts[square.piece.role] = (counts[square.piece.role] ?? 0) + 1;
    }
  }
  const missing: PieceView[] = [];
  for (const role of ["q", "r", "b", "n", "p"] as const) {
    const count = Math.max(0, (INITIAL[role] ?? 0) - (counts[role] ?? 0));
    for (let i = 0; i < count; i += 1) {
      missing.push({ color, role, glyph: "" });
    }
  }
  return missing;
}

export function capturedPieces(board: SquareView[]): {
  byWhite: PieceView[];
  byBlack: PieceView[];
} {
  return {
    byWhite: missingPieces(board, "black"),
    byBlack: missingPieces(board, "white"),
  };
}

export function sideLabel(color: ColorKey): string {
  return color === "white" ? "White" : "Black";
}
