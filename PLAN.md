# Gambiit — Working Plan

**Status:** decisions confirmed 2026-11-05  
**Project location:** `~/projektid/gambiit`  
**Working name:** Gambiit (Estonian for “gambit”)  
**Relationship to agentchess:** separate project. `~/projektid/agentchess` stays untouched as the original browser prototype. We reuse its `AGENTS.md` playing philosophy, not its code.

> This is the editable working plan. When the repository is created, copy it to
> `~/projektid/gambiit/PLAN.md` (or `docs/PLAN.md`) and keep it in sync.
>
> Estonian working version: [`PLAN.et.md`](PLAN.et.md).

---

## 0. Confirmed decisions

1. **New project** in `~/projektid/gambiit`; agentchess remains a separate experiment.
2. **Stack:** Rust core + Tauri 2 + Svelte 5 + TypeScript. All chess logic, AI integration, persistence and the local API are Rust. The UI is web tech inside a native Tauri window because that gives the best design/theming/animation flexibility.
3. **Agent mode:** prompt mode first. The agent should play like a real opponent: after the human moves, the agent automatically sees the position and makes its move. **No typing in a terminal is required.** Settings offer three modes:
   - **Auto** (default): the agent responds automatically after the human move.
   - **Button:** the agent moves only when the user clicks “Agent move”.
   - **Manual chat:** the app waits while the user talks to the agent in OpenCode (MCP mode, later).
4. **Stockfish is included** as a first-class dependency for hybrid play and post-game analysis. It is an external binary (GPL); the app detects it, and if missing shows a clear install prompt. Personas provide the theater; difficulty levels control Stockfish strength.
5. **Default language: English.** Estonian is an optional UI/agent language. The project is intended for GitHub.
6. **Settings are a major feature:** every game property and “theater” behavior can be toggled on/off, not buried in code.

---

## Phase 0 results (2026-11-05)

Completed:

- Workspace created at `~/projektid/gambiit`; `PLAN.md` and `README.md` are in place.
- Crates: `gambiit-core` (rules/state), `gambiit-engine` (Stockfish UCI), `gambiit-agent` (OpenCode prompt transport), `gambiit-cli` (terminal prototype).
- `cargo test`: 13 tests pass (core rules, perft, threefold, castling, JSON parsing, Stockfish info parsing).
- `cargo clippy --all-targets -- -D warnings`: clean.
- `perft(4)` = 197,281.
- Stockfish 19 detected at `/usr/bin/stockfish`; UCI MultiPV candidate generation works.
- OpenCode Desktop 2.2.5 CLI (`~/.config/ai.opencode.desktop/cli/2.2.5-x64/opencode-cli`) is auto-discovered and works with `opencode-go/deepseek-v4.1-flash` through the running desktop service.
  - `probe --prompt` returned valid JSON.
  - 2-ply game played with the default model: legal `...e5`, about 6 s, about 16,700 tokens, strong theatrical commentary.
  - Session reuse with local Ollama `ollama/gemma4` also works; about 20–24 s per move, about 3,000 tokens.
- Commentary and candidate rejection reasons are parsed and displayed; illegal moves are rejected and retried.

Notes / blockers:

- The standalone `/home/rk/.opencode/bin/opencode` 1.18.35 has a separate expired `opencode-go` API key. It is now a fallback; Gambiit prefers the desktop CLI, and `OPENCODE_BIN` can override discovery. Re-authenticate the standalone CLI with `opencode auth login` if it is needed directly.
- MCP automatic play is not implemented yet; prompt mode is the default. MCP remains a later enhancement.
- The CLI is a development tool. The final UI uses buttons and automatic turns; the human will not type moves in a terminal.

Next:

- Tauri 2 + Svelte 5 UI shell, local two-player board, then wire the prompt transport to the UI.
- Keep the CLI as a fast test harness and fallback.

---

## Desktop UI milestone (2026-11-05)

Completed:

- Tauri 2 + Svelte 5 desktop app in `src-tauri/` and `src/`.
- Click-to-move board with Cburnett SVG pieces, legal-move dots, capture rings,
  last-move highlights, check marks and a promotion picker. Board cells are
  reactive, so pieces update immediately after both human and agent moves.
- Player strips, material/captured pieces, status, thinking ribbon and
  theater-style commentary panel.
- Three themes: Salon, Tournament, Midnight; live preview in Settings.
- Settings: model, illustrated persona, response mode (Auto / Button), colour,
  theme, font size (90–150%, default 120%), separate UI and agent languages,
  sound, animations and Stockfish difficulty.
- Localized UI + agent language: `src/locales/*.json` auto-discovery, English
  and Estonian catalogs, language selection only in Settings, independent UI
  and agent languages.
- Searchable model picker grouped by provider, with model-list refresh.
- Optional provider API keys stored in `~/.config/gambiit/credentials.json`
  (0600), passed as environment variables to a private OpenCode child server;
  empty key falls back to OpenCode's own authentication.
- Automatic game persistence after every move, resume of the latest game,
  saved-game library, PGN export/import with comments and tolerant PGN parsing.
- Five Stockfish difficulty levels — Beginner, Casual, Intermediate, Strong and
  Master — plus Custom. Selecting a level sets the depth/MultiPV sliders; moving
  a slider selects Custom. Stockfish `Skill Level` is part of each preset.
- Illustrated theater persona cards: Theater Master, Cold Calculator, Trickster,
  Panicker, Coach and Chaotic. Personas affect commentary only; the move must
  come from the Stockfish candidate pool. The avatar is also used in the player
  strip and commentary cards.
- Rust commands: `new_game`, `get_state`, `make_move`, `agent_move`, `takeback`,
  `resign`, `list_models`; all moves validated by `gambiit-core`.
- Takeback policy enforced: only the human's own last move can be undone.
- Backend tests: 5 fast tests plus an ignored real-model `agent_smoke` test.
  The smoke test played `e4` → `...c5` in 8.9 s with 522 tokens.
- A real UI game played `e4` → `...e5`; commentary, candidates, plan and token
  usage rendered in the Theater panel. Screenshot: `docs/ui-preview.png`.

Not yet:

- local Axum API and MCP bridge;
- clocks, post-game Stockfish analysis;
- theme editor and custom piece-set loading;
- chat, personas beyond the prompt presets, Truth Lens, Director's Cut,
  leaderboard and broadcast mode.

Next:

- Add post-game Stockfish analysis and the evaluation graph.
- Add the theme editor and custom piece-set folders.
- Then MCP mode, chat and Truth Lens.

---

## 1. Vision and product principles

Gambiit is a local desktop chess game where the opponent is an OpenCode 2 model with a personality. It should feel like a real game — the board is always live, the opponent reacts immediately, and the theater is part of the experience.

Core principle from `agentchess/AGENTS.md`:

> **The board is truth; the voice is free.**

- Move legality, checkmate, stalemate, draw rules, clocks and results are server-authoritative and always visible.
- The agent may bluff about evaluations, plans, emotions and psychology.
- The agent must never lie about mechanics. The game validates every move and the UI never hides the real state.
- The agent’s future plans are secret. Only public post-move commentary is shown. Chain-of-thought is never displayed.
- The game should be fun to watch, not only to play: commentary cards, personas, sounds, animations, post-game theater.

Design tagline: **“Every move is a performance.”**

---

## 2. Technical stack

| Layer | Choice | Notes |
|---|---|---|
| Language | Rust (edition 2021/2024 as supported) | core, API, agent, storage |
| App shell | Tauri 2 | native window, small binary, Rust backend |
| Frontend | Svelte 5 + TypeScript + Vite | CSS themes, SVG pieces, animations |
| Chess rules | `shakmaty` | legal moves, FEN, SAN, UCI, outcomes |
| PGN | `pgn-reader` + custom writer | import/export, comments, clocks |
| Local API | `axum` + `tokio` + `tower-http` | HTTP + SSE for agents, token auth |
| AI | OpenCode CLI (`opencode models`, `opencode run`) | uses the user’s existing auth/quota |
| MCP (later) | stdio bridge (`gambiit-mcp` or `gambiit mcp`) | exposes chess tools to OpenCode |
| Engine | Stockfish via UCI | hybrid opponent, analysis |
| Storage | JSON + PGN; SQLite only if needed later | simple and portable |
| Logging | `tracing` + rolling file | debug agent failures |
| Tests | `cargo test`, Vitest, Playwright, fake agent | correctness + UI confidence |
| Packaging | `tauri build`, PKGBUILD, `.desktop` | Arch/CachyOS first |

**Environment checked:** Rust 1.99, Node 26, pnpm, webkit2gtk 2.54, OpenCode Desktop 2.2.5 (working `opencode-go` auth via the desktop service) and standalone CLI 1.18.35, Stockfish 19 at `/usr/bin/stockfish`.

**Why not pure egui:** possible, but themes, SVG piece sets, motion, accessibility and rapid visual iteration are much faster in CSS/Svelte. The Rust core stays reusable if the UI is ever swapped to egui.

---

## 3. Architecture

```
Svelte UI ──Tauri IPC──┐
                       ├── GameService (Rust)  ← single source of truth
Local HTTP API ────────┤      ├── shakmaty
   ▲        ▲          │      ├── GameStore (JSON/PGN)
   │        └─ MCP ────┘      └── OpponentOrchestrator
   │                                ├── OpenCode prompt transport
OpenCode 2 ────────────────────────┼── OpenCode MCP transport
                                   └── Stockfish transport
```

- `GameService` owns position, move history, clocks, result, commentary, chat and settings relevant to the game.
- All mutations go through it: UI commands, HTTP API, MCP bridge. No adapter mutates state directly.
- Every agent move is validated against `shakmaty`’s legal move list before it is applied.
- The UI displays the authoritative state; it never computes its own rules.
- The local API binds to `127.0.0.1` only. A random bearer token is written to `$XDG_RUNTIME_DIR/gambiit/session.json` (mode 0600).
- Tauri IPC is for the UI; the HTTP API is for agents. Both call the same service.

**Runtime modes:**

- **Native:** Tauri window (primary).
- **Browser:** `gambiit serve` serves the same frontend over localhost. Useful for development and as a fallback if WebKitGTK has issues on Wayland.

---

## 4. Agent integration

### 4.1 Auto prompt mode (default, first implementation)

Flow:

1. Human makes a move.
2. `GameService` sets turn to the agent and emits `agent_turn`.
3. UI immediately shows “Agent is thinking…” with elapsed time and a subtle animation.
4. Orchestrator builds a compact prompt: FEN, recent PGN, legal moves (UCI + SAN), status, human color, persona, protocol, language, clock state.
5. It spawns `opencode run -m <provider/model> --format json -s <session> "<prompt>"`.
6. The model returns strict JSON:

```json
{
  "move": "e7e5",
  "comment": "The position is roughly balanced...",
  "eval": "-0.2",
  "candidates": [
    { "move": "e7e5", "why": "develops and keeps tension" },
    { "move": "c7c5", "why_rejected": "too sharp here" }
  ],
  "plan": "Public plan; may be misleading"
}
```

7. The app validates the move. On malformed JSON or an illegal move, it retries **once** with the error and legal move list. If it still fails, the game pauses with clear options: retry, switch model, use a Stockfish move, or resign/abort.
8. A valid move is applied; commentary appears in the “Theater” panel.

**Prompt rules:**

- Prefer **UCI** (`e2e4`, `e7e8q`) as the move format; accept SAN as a fallback.
- Include FEN + last ~20 plies, not the whole game, to keep prompts small.
- Include draw status (threefold/50-move) computed by the app; the model does not adjudicate.
- Ask for public commentary only; no hidden chain-of-thought.
- Require the mandatory AGENTS.md structure: evaluation, 3–4 candidates with rejection reasons, chosen move + plan.
- Language: English by default; Estonian when selected.

**Session handling:** capture the OpenCode session id from the first run and reuse it with `-s`. Store it in the game record. A session can be reused across a rematch for continuity, or reset for a fresh start.

**No terminal typing:** the UI triggers everything. A “Cancel thinking” button is available. In Button mode the trigger is an explicit “Agent move” button; in Auto mode it happens after every human move.

### 4.2 Manual MCP mode (later, for the agentchess experience)

- A small `gambiit-mcp` stdio bridge exposes:
  - `chess_state()`
  - `chess_play_move(move, comment?)`
  - `chess_comment(text, structured?)`
  - `chess_chat(text)`
  - `chess_new_game(human_color?)`
  - `chess_offer_takeback()` / `chess_accept_takeback()` / `chess_decline_takeback()`
  - `chess_offer_draw()` / `chess_accept_draw()` / `chess_resign()`
  - `chess_get_pgn()`
- The bridge reads the app’s discovery file (port + token) and forwards calls to the HTTP API.
- OpenCode gets a dedicated workspace with `AGENTS.md` and `opencode.json`; the agent is allowed only `gambiit_*` tools (bash/edit/webfetch denied).
- The user can talk to the agent in OpenCode and say “your move”; the game updates. This is the closest to the original agentchess flow.
- If automatic MCP play works reliably (tested in Phase 0), it can become an alternative default; otherwise prompt mode stays primary.

### 4.3 Stockfish hybrid (required for a decent game)

Pure LLM chess is weak. Difficulty levels use Stockfish as a candidate generator:

- **Beginner:** Skill 0, depth 5, top 6 candidates.
- **Casual:** Skill 5, depth 8, top 5.
- **Intermediate (default):** Skill 10, depth 11, top 4.
- **Strong:** Skill 15, depth 15, top 3.
- **Master:** Skill 20, depth 19, top 2.
- **Custom:** user-chosen depth/MultiPV, Skill 20.

Selecting a level sets the depth/MultiPV sliders; moving a slider selects Custom.
The LLM must choose from the Stockfish candidate pool. The persona affects the
commentary only, never the move choice.

Stockfish integration:

- Detect `stockfish` in `PATH`; allow a custom binary path in Settings.
- Spawn a persistent UCI process per session/analysis; `setoption name Skill Level value N`, `setoption name MultiPV value N`, `go depth D` / `movetime`.
- Parse `info` lines and `bestmove`; show candidate evals only where intended.
- If missing: show an install banner (`sudo pacman -S stockfish` on Arch/CachyOS) and fall back to legal-moves-only mode.
- Do not bundle Stockfish by default (GPL). Keep it an external binary; document licensing.

Post-game analysis:

- Depth/time configurable; run in a background task.
- Accuracy, blunder/mistake/inaccuracy list, eval graph, best-move hints.
- Optional PGN annotation with evals.
- Eval bar during the game is **off by default** in agent games; optional in Settings.

### 4.4 AGENTS.md principles → product features

| Principle | Implementation |
|---|---|
| Board is truth, voice is free | server-side validation; eval bar hidden by default; truth lens after the game |
| Read before move | agent must query state; prompt includes FEN + legal moves; stale/illegal moves rejected |
| Mandatory commentary | structured cards: eval, candidates, chosen move, plan |
| Future plans are secret | no chain-of-thought in UI; only public commentary; chat may bluff |
| Takebacks | human moves never undone; agent takeback only after human offer + agent accept; enforced in code |
| Drama / tone modes | persona presets and dynamic register; post-game tribute lines |
| Never lie about mechanics | board/status/result authoritative; agent errors visible |
| Endgame words | result overlay with one of 40 tribute/consolation lines + rematch |
| Always a performance | theater panel, sounds, animations, broadcast mode |

---

## 5. UI / UX

### Art direction

“Theater program”, not a generic chess app:

- **Palette:** charcoal background, warm walnut/cream board, brass accent, deep green legal moves, wine-red check.
- **Type:** `Fraunces`/`Playfair Display` for display names and headings, `Inter` for UI, `JetBrains Mono` for moves/PGN. Bundle fonts locally (OFL).
- **Layout:** board is the stage; right panel holds the playbill: Moves / Theater / Chat / Analysis.
- **Player strips:** agent avatar + persona + model + “thinking…” status above; human + captured pieces + material below.
- **Commentary cards:** brass left rule, move number, eval badge, candidate chips with rejection reasons, plan quote. This is the signature feature.
- **Result overlay:** result, reason, agent’s tribute line, Rematch, Analyze, Export PGN.

### Interaction

- Click-click and drag-and-drop; legal dots, capture rings, last-move highlight, check glow, selected lift.
- Promotion picker; right-click arrows for planning; clear on move.
- Keyboard: arrows navigate, Enter selects, Esc cancels, `F` flip, `U` undo (human only), `N` new game, `?` shortcuts.
- SAN/UCI input field for accessibility/quick entry.
- Animations 180–220 ms; respect `prefers-reduced-motion`.
- Sounds: move, capture, check, castle, promote, game end; volume/mute; sound packs.

### Accessibility

- Board squares are real buttons with ARIA labels (“e4, white pawn”); live region announces moves.
- High-contrast theme, colorblind-friendly board (lightness, not only hue), focus rings.
- Font scale 90–125%; responsive layout; keyboard-only play.
- English default, Estonian optional; all strings in locale files.

### First-run wizard

1. Language (English default / Estonian).
2. Side (White / Black / Random).
3. Opponent: model + persona + difficulty.
4. Board: 3 theme cards.
5. Start game + short note that the agent may bluff but the board is always true.

---

## 6. Themes and pieces

### Built-in board themes (7)

1. **Salon** (default): walnut/cream, brass, dark UI.
2. **Tournament**: classic green/cream, light UI.
3. **Marble**: white/grey marble, light UI.
4. **Midnight**: charcoal/steel blue, neon accent.
5. **Paper**: minimal ink/paper, light UI.
6. **Neon**: dark cyan/magenta.
7. **High Contrast**: accessibility-first.

### Piece sets (4–6)

- Cburnett (classic; document license/attribution).
- Merida (traditional; verify license).
- Alpha (clean; verify license).
- Staunton (custom or CC0).
- **Minimal** — monochrome, fully recolorable via `currentColor`.
- Unicode fallback.

### Theme editor

- Board: light/dark squares, border, coordinates, highlights, texture (wood/marble/paper/noise).
- Pieces: set, scale, shadow, style; colors for the Minimal set.
- UI: dark/light/auto, accent, background, panel, text, font, density.
- Effects: animation speed, sound pack.
- Live mini-board preview; contrast warnings; randomize; save/duplicate/export/import.
- Themes are versioned JSON. Built-ins are read-only; custom themes live in `~/.local/share/gambiit/themes/`.
- Custom piece sets: folder with `manifest.json` + SVGs; app scans and validates; license metadata in the manifest.

---

## 7. Settings (first-class, everything toggleable)

Settings must expose game properties and theater features without code edits.

### Opponent

- Model (`provider/model`, searchable list from `opencode models --verbose`; manual entry allowed).
- Persona preset (Theater Master, Cold Calculator, Trickster, Panicker, Coach, Chaotic, custom).
- Response mode: **Auto after human move** (default) / Button “Agent move” / Manual chat (MCP).
- Theatrical delay before showing the agent move (0–2 s, default 0).
- Thinking budget/timeout (30–300 s).
- Retry count on invalid move (0–2; default 1).
- Fallback: pause / Stockfish move / random legal move.
- Session reuse across games (on/off).

### Game

- Human color; board flip.
- Clock: none (default) / human only / both with agent thinking budget.
- Time control presets: relaxed, 10+5, 15+10, 30+0, custom.
- Takebacks: off / human only / offer-and-accept (default per AGENTS.md).
- Draw offers: on/off.
- Resign confirmation: on/off.
- Show legal moves, last move, coordinates, captured pieces, material advantage.
- PGN export/import; auto-save; resume last game.

### Theater

- Commentary: on/off.
- Verbosity: short / standard / full.
- Show candidate moves and rejection reasons: on/off.
- Show eval claims: on/off.
- Chat with opponent: on/off.
- Deception level: off / light / full (AGENTS.md).
- Drama level: calm / theatrical / maximum.
- Taunts and psychological warfare: on/off.
- Post-game tribute lines: on/off.
- Post-game press conference: on/off (stretch).
- Director’s Cut narrative: on/off (stretch).
- Truth Lens (claimed eval vs engine): on/off (post-game only).
- Agent language: English (default) / Estonian.

### Appearance

- Board theme, piece set, UI theme (dark/light/auto).
- Board size, sidebar density, font scale.
- High contrast, reduced motion.
- Sound pack, volume, mute.

### Engine

- Stockfish path (auto-detect + custom).
- Difficulty: beginner / casual / intermediate / strong / master / custom.
- Depth/MultiPV sliders; presets set them, slider changes select Custom.
- MultiPV candidates, depth/time limits.
- Post-game analysis depth/time.
- Show eval bar during game (off by default).
- Show engine lines in analysis only.

### Advanced

- OpenCode binary path.
- MCP mode and agent workspace path.
- Logs folder, open logs, copy diagnostics.
- Data folder, backup/export games, reset settings.
- Experimental features (MCP auto-play, broadcast mode).

---

## 8. Feature scope

### MVP — “play one game to the end”

- All chess rules: castling, en passant, promotion, checkmate, stalemate, 50-move, threefold, insufficient material.
- Local two-player mode.
- Board, moves, status, flip.
- One agent transport (prompt JSON) + model selector.
- Auto/button response modes; “Agent is thinking…” state.
- Agent commentary feed.
- New game, side selection, save/resume.
- One dark theme + one piece set.
- Stockfish detection + hybrid candidate mode.

### v1 — “real game”

- Theme system: 7 themes, 4+ piece sets, theme editor.
- Animations, sounds, accessibility, English/Estonian i18n.
- PGN export/import, clocks, takeback/draw/resign rules.
- Personas, structured commentary, chat.
- MCP manual mode.
- Model leaderboard, post-game analysis, eval bar toggle.
- Tauri packaging, PKGBUILD, `.desktop`.

### Stretch — “full theater”

- Truth Lens, Director’s Cut, broadcast mode.
- Opening roulette, handicaps, press conference, grudge-match memory.
- Coach mode, local TTS, puzzles, online play (separate large project).

---

## 9. Roadmap

| Phase | Content | Done when |
|---|---|---|
| **0. Risk spike** | CLI prototype: `shakmaty` + `opencode run` + JSON parser + terminal board. Test auto MCP. Install/detect Stockfish. Short model bake-off (3 models: legality, latency, cost, English commentary). | A full game vs a model in the terminal; decision on prompt vs MCP default. |
| **1. Core + API** | `gambiit-core`, `GameService`, axum API, discovery file, fake agent, perft tests. | All rules/API tests green; fake agent plays 100 games. |
| **2. UI shell** | Tauri + Svelte, board, moves, local two-player, save/resume. | Legal game in a native window; flip; restart resumes. |
| **3. Agent in UI** | Prompt transport, Auto/Button modes, model/persona selection, commentary, error handling. | Full game vs OpenCode model without terminal typing; illegal move recovers. |
| **4. Design system** | 7 themes, piece sets, theme editor, animations, sounds, accessibility. | Theme switches live and persists; contrast warnings work. |
| **5. Game completeness** | PGN ✅, persistence ✅; clocks, takebacks, draw/resign, analysis, result screen, tribute lines. | PGN round-trip; takeback rules enforced; analysis works. |
| **6. Theater extras** | Personas, chat, Truth Lens, Director’s Cut, leaderboard, MCP, broadcast. | Toggleable and non-intrusive. |
| **7. Packaging** | PKGBUILD, `.desktop`, icon, docs, tests, performance. | Installable on CachyOS; starts <2 s; README. |

Start with Phase 0. The biggest risk is the agent loop, not the board or themes.

---

## 10. Project structure

```
gambiit/
├─ Cargo.toml                 # workspace
├─ crates/
│  ├─ gambiit-core/           # rules, game state, clocks, PGN, takeback policy
│  ├─ gambiit-agent/          # transports, prompts, orchestration
│  └─ gambiit-mcp/            # stdio MCP bridge -> local HTTP API
├─ src-tauri/                 # Tauri shell, axum API, commands
├─ src/                       # Svelte UI
│  ├─ App.svelte
│  ├─ lib/*.svelte, *.ts
│  ├─ locales/en.json         # UI + agent language catalogs
│  ├─ locales/et.json
│  └─ assets/pieces/cburnett/
├─ themes/                    # built-in theme JSON
├─ docs/                      # LOCALES, ARCHITECTURE, AGENTS, API, THEMES, TESTING
├─ scripts/                   # dev, fake-agent, agent-workspace
└─ packaging/                 # PKGBUILD, .desktop
```

Data:

```
~/.config/gambiit/settings.json
~/.local/share/gambiit/games/<uuid>.json
~/.local/share/gambiit/themes/
~/.local/share/gambiit/agent-workspace/
$XDG_RUNTIME_DIR/gambiit/session.json   # port + token, 0600
```

Atomic writes (temp + rename), PGN export always available.

---

## 11. Testing, risks, security

### Tests

- **Core:** perft 1–5 from start and Kiwipete; SAN/FEN/UCI round-trip; castling, en passant, promotion; 50-move, threefold, insufficient material; PGN.
- **API:** wrong turn, illegal move, duplicate request, bad token, SSE events.
- **Prompt parser:** broken JSON, JSON in prose, illegal move, promotion, timeout; one retry and clear error.
- **Fake agent:** 100 random legal games; no panics; correct termination.
- **UI:** Vitest + Playwright in browser mode with mocked API; visual regressions for 3 sizes × 7 themes; keyboard and ARIA.
- **Real models:** short smoke test with 3 models; measure legality, latency, commentary quality, token cost. Ask before spending quota.

### Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Tauri/WebKitGTK Wayland issues | UI fails | test early; browser fallback `gambiit serve` |
| OpenCode CLI/MCP changes | agent breaks | transport abstraction; version check; prompt fallback |
| LLM illegal move | game stuck | server validation; one retry; pause options; Stockfish fallback |
| Slow/expensive models | bad UX | async UI; timeout; model selector; local models; usage display |
| MCP permissions/config | MCP unavailable | Phase 0 spike; prompt mode primary |
| Stockfish missing | no hybrid/analysis | detect; clear install prompt; legal-only fallback |
| Asset licenses | legal risk | prefer CC0/CC BY; license files; attribution |
| Local API abuse | security | localhost + token + CORS + CSP; chess endpoints only |
| Clock unfairness | bad game feel | no clock default; human-only clock; agent thinking budget |
| Scope creep | delays | vertical slice; MVP before themes |

### Security

- API only on `127.0.0.1`, random token, 0600, CORS restricted to Tauri/browser origin.
- CSP; sanitize commentary; render untrusted text as text/markdown, never `{@html}`.
- Validate PGN/theme imports; size limits; no executable content.
- Agent workspace denies bash/edit/webfetch; only `gambiit_*` tools.
- The app never reads or stores API keys; OpenCode handles auth.
- Logs redact tokens; “Open logs” button in settings.

---

## 12. Additional ideas

1. **Truth Lens** — after the game, compare the agent’s claimed evaluations with Stockfish; flag large discrepancies. Only objective claims; plans are allowed to be lies. Fun and instructive.
2. **Director’s Cut** — generate a narrative recap with key moments, board snapshots, and the agent’s tribute; export Markdown/HTML.
3. **Model leaderboard** — local W/L/D, accuracy, average thinking time, “trash-talk score” per model/persona; weekly/season view.
4. **Persona mixer** — sliders for aggression, deception, verbosity, drama, humor, language; presets with avatars/accents.
5. **Broadcast mode** — large board, commentary ticker, hidden side panels; OBS-friendly.
6. **Opening roulette** — curated random openings or thematic matches.
7. **Handicaps** — agent without a queen, human extra time, etc.
8. **Press conference** — post-game Q&A in character.
9. **Grudge match** — same model/persona remembers the previous game; rivalry dossier.
10. **Coach mode** — honest Stockfish + LLM analysis.
11. **Local TTS** — Piper reads commentary; optional.
12. **Theme sharing** — `.gambiit-theme` files, import/export, eventual gallery.

Top 5: Truth Lens, Director’s Cut, Persona Mixer, Model Leaderboard, Broadcast Mode.

---

## 13. Next step

Phase 0:

1. Create the workspace and `PLAN.md` (copy this file).
2. Build `gambiit-core` with `shakmaty`; CLI board and legal move loop.
3. Install/detect Stockfish; test UCI candidate generation.
4. Spike OpenCode prompt transport with a strict JSON schema; measure 3 models on a small set of positions.
5. Spike automatic MCP with a stdio bridge; decide whether it can be the default.
6. Record results; then start the Tauri UI shell.

**Do not start with themes.** First make a full game against a model work end-to-end. Then make it beautiful.
