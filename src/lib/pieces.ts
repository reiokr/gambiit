import wK from "../assets/pieces/cburnett/wK.svg";
import wQ from "../assets/pieces/cburnett/wQ.svg";
import wR from "../assets/pieces/cburnett/wR.svg";
import wB from "../assets/pieces/cburnett/wB.svg";
import wN from "../assets/pieces/cburnett/wN.svg";
import wP from "../assets/pieces/cburnett/wP.svg";
import bK from "../assets/pieces/cburnett/bK.svg";
import bQ from "../assets/pieces/cburnett/bQ.svg";
import bR from "../assets/pieces/cburnett/bR.svg";
import bB from "../assets/pieces/cburnett/bB.svg";
import bN from "../assets/pieces/cburnett/bN.svg";
import bP from "../assets/pieces/cburnett/bP.svg";

const URLS: Record<string, string> = {
  wK,
  wQ,
  wR,
  wB,
  wN,
  wP,
  bK,
  bQ,
  bR,
  bB,
  bN,
  bP,
};

export function pieceUrl(color: string, role: string): string {
  const side = color === "white" || color === "w" ? "w" : "b";
  return URLS[side + role.toUpperCase()] ?? "";
}

export function pieceAlt(color: string, role: string): string {
  const names: Record<string, string> = {
    p: "pawn",
    n: "knight",
    b: "bishop",
    r: "rook",
    q: "queen",
    k: "king",
  };
  return `${color} ${names[role] ?? "piece"}`;
}
