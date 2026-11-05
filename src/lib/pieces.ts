import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";
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

/** Data URLs for the selected custom piece set; empty means built-in. */
export const pieceUrls = writable<Record<string, string>>({});

export const DEFAULT_PIECE_PREVIEW = wN;

let pieceSetToken = 0;

export async function applyPieceSet(id: string): Promise<void> {
  const token = ++pieceSetToken;
  if (!id || id === "cburnett") {
    pieceUrls.set({});
    return;
  }
  try {
    const urls = await invoke<Record<string, string>>("load_piece_set", {
      id,
    });
    if (token === pieceSetToken) {
      pieceUrls.set(urls ?? {});
    }
  } catch {
    if (token === pieceSetToken) {
      pieceUrls.set({});
    }
  }
}

export function pieceUrl(
  color: string,
  role: string,
  overrides: Record<string, string> = {},
): string {
  const side = color === "white" || color === "w" ? "w" : "b";
  const code = side + role.toUpperCase();
  return overrides[code] ?? URLS[code] ?? "";
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
