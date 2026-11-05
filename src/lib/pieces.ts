import { invoke } from "@tauri-apps/api/core";
import { writable } from "svelte/store";

type PieceUrls = Record<string, string>;

const modules = import.meta.glob("../assets/pieces/*/*.svg", {
  eager: true,
  import: "default",
}) as Record<string, string>;

const BUILT_IN_SETS: Record<string, PieceUrls> = {};

for (const [path, url] of Object.entries(modules)) {
  const match = path.match(/\/pieces\/([^/]+)\/([wb])([KQRBNP])\.svg$/);
  if (!match) continue;
  const [, set, color, role] = match;
  BUILT_IN_SETS[set] ??= {};
  BUILT_IN_SETS[set][color + role] = url;
}

const FALLBACK = BUILT_IN_SETS.cburnett ?? {};

/** Resolved URLs for the selected piece set; empty means the built-in fallback. */
export const pieceUrls = writable<PieceUrls>({});

const FALLBACK_PREVIEW = FALLBACK.wN ?? "";

let pieceSetToken = 0;

export function piecePreview(id: string, code = "wN"): string {
  return BUILT_IN_SETS[id]?.[code] ?? FALLBACK_PREVIEW;
}

export async function applyPieceSet(id: string): Promise<void> {
  const token = ++pieceSetToken;
  const builtIn = BUILT_IN_SETS[id];
  if (builtIn) {
    pieceUrls.set({ ...builtIn });
    return;
  }
  if (!id) {
    pieceUrls.set({ ...FALLBACK });
    return;
  }
  try {
    const urls = await invoke<PieceUrls>("load_piece_set", { id });
    if (token === pieceSetToken) {
      pieceUrls.set({ ...FALLBACK, ...(urls ?? {}) });
    }
  } catch {
    if (token === pieceSetToken) {
      pieceUrls.set({ ...FALLBACK });
    }
  }
}

export function pieceUrl(
  color: string,
  role: string,
  overrides: PieceUrls = {},
): string {
  const side = color === "white" || color === "w" ? "w" : "b";
  const code = side + role.toUpperCase();
  return overrides[code] ?? FALLBACK[code] ?? "";
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
