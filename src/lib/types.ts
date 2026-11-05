export type ColorKey = "white" | "black";

export interface PieceView {
  color: ColorKey;
  role: string;
  glyph: string;
}

export interface SquareView {
  square: string;
  piece: PieceView | null;
}

export interface MoveInfo {
  uci: string;
  san: string;
  from: string;
  to: string;
  promotion: string | null;
  piece: string;
  captured: string | null;
  isCastle: boolean;
  isEnPassant: boolean;
}

export interface MoveRecord {
  ply: number;
  color: ColorKey;
  san: string;
  uci: string;
  from: string;
  to: string;
  captured: string | null;
  fenAfter: string;
  comment: string | null;
  eval: string | null;
}

export interface CandidateView {
  move: string;
  why: string | null;
  whyRejected: string | null;
}

export interface CommentaryCard {
  san: string;
  uci: string;
  comment: string;
  eval: string | null;
  candidates: CandidateView[];
  plan: string | null;
  tokens: number | null;
  cost: number | null;
  elapsedMs: number;
}

export interface Settings {
  model: string;
  persona: string;
  uiLanguage: string;
  agentLanguage: string;
  agentLanguageName: string;
  responseMode: "auto" | "button";
  stockfishDepth: number;
  multipv: number;
  difficulty: string;
  theme: string;
  sound: boolean;
  animations: boolean;
  fontScale: number;
}

export interface GameSummary {
  id: string;
  name: string;
  updatedAt: number;
  humanColor: ColorKey;
  model: string;
  moveCount: number;
  result: string;
}

export interface GameSnapshot {
  gameId: string;
  gameName: string;
  fen: string;
  turn: ColorKey;
  humanColor: ColorKey;
  agentColor: ColorKey;
  statusText: string;
  statusKey: string;
  statusSide: ColorKey | null;
  gameOver: boolean;
  winner: ColorKey | null;
  check: boolean;
  checkSquare: string | null;
  legalMoves: MoveInfo[];
  history: MoveRecord[];
  lastMove: MoveRecord | null;
  board: SquareView[];
  commentary: CommentaryCard[];
  settings: Settings;
  agentError: string | null;
  thinking: boolean;
}

export type AnalysisClass =
  | "best"
  | "good"
  | "inaccuracy"
  | "mistake"
  | "blunder";

export interface AnalysisPly {
  ply: number;
  san: string;
  uci: string;
  color: ColorKey;
  fenBefore: string;
  fenAfter: string;
  evalBeforeCp: number;
  evalAfterCp: number;
  bestUci: string | null;
  bestSan: string | null;
  lossCp: number;
  classification: AnalysisClass;
  claimedEval: string | null;
  claimGapCp: number | null;
  claimMismatch: boolean;
}

export interface AnalysisReport {
  depth: number;
  initialEvalCp: number;
  positions: AnalysisPly[];
  whiteAccuracy: number;
  blackAccuracy: number;
  whiteBlunders: number;
  blackBlunders: number;
  whiteMistakes: number;
  blackMistakes: number;
  whiteInaccuracies: number;
  blackInaccuracies: number;
  truthLensFlags: number;
}
