# Gambiit

A Rust-based desktop chess game where your opponent is an OpenCode model — with a
personality, a plan, and a taste for theater.

> **The board is truth; the voice is free.**
> Move legality, checkmate and results are always verified by the game. The agent
> may bluff about evaluations and plans, but never about the mechanics.

![Gambiit desktop UI](docs/ui-preview.png)

**Eestikeelne tööplaan:** [`PLAN.et.md`](PLAN.et.md)

## Status

**Desktop UI milestone complete** — a playable Tauri app with a real OpenCode
opponent. The terminal prototype remains as a fast test harness. See
[`PLAN.md`](PLAN.md) for the full architecture and roadmap.

What works now:

- complete chess rules via `shakmaty` (castling, en passant, promotion, mate,
  stalemate, threefold repetition, 50-move rule, insufficient material);
- Tauri 2 + Svelte 5 desktop window;
- click-to-move board with legal-move dots, capture rings, last-move highlights,
  check marks, promotion picker and a theme editor;
- five built-in SVG piece sets — Cburnett, Livius, Meridian, Meridian Shaded
  and Letter — plus custom piece-set folders;
- Stockfish hybrid play: a near-best candidate (within 40 centipawns) is
  recommended for variety, and the model plays it and explains;
- OpenCode prompt transport with strict JSON validation and retries;
- automatic agent replies after the human move, or an “Agent move” button;
- session reuse across moves, so the opponent remembers the game;
- commentary cards with evaluations, candidate moves, rejection reasons, plan,
  elapsed time and token usage;
- chat with the opponent between moves, in character and in the agent language;
- draw offers decided by the opponent, with the result shown on the board;
- an optional chess clock with bullet, blitz, rapid, classical and custom time
  controls, increments, flag fall and PGN `TimeControl` tags;
- takeback policy enforced per `AGENTS.md`: only your own last move is undoable;
- automatic save after every move, resume of the latest game and a saved-game
  library (open, rename, delete);
- PGN export with tags/comments and tolerant PGN import;
- post-game Stockfish analysis: accuracy for both sides, move classification,
  an evaluation graph, selected-move detail with a board, and the Truth Lens
  comparing the agent's claimed evaluations with the engine;
- settings for model, persona, response mode, colour, theme, font size
  (90–150%, default 120%), language, sound, animations, Stockfish depth and
  candidate count;
- searchable model picker grouped by provider, with a model-list refresh;
- optional provider API keys stored locally with 0600 permissions and passed to
  OpenCode as environment variables; no key means OpenCode's own auth is used;
- separate UI and agent languages, both selected in Settings; the agent can
  answer in English while the UI is Estonian;
- illustrated theater personas: Theater Master, Cold Calculator, Trickster,
  Panicker, Coach and Chaotic; the persona changes commentary only;
- five Stockfish difficulty levels — Beginner, Casual, Intermediate, Strong and
  Master — plus Custom, with depth and MultiPV sliders;
- localized UI and agent commentary from `src/locales/*.json`; English and
  Estonian included, new languages are added by dropping in a JSON catalog;
- desktop and terminal builds share the same Rust core, engine and agent layers.

Not yet: MCP mode, Director's Cut, leaderboards and broadcast mode. Those are
the next phases in `PLAN.md`.

## Run the desktop app

```bash
pnpm install
pnpm tauri dev
```

For a standalone release binary:

```bash
pnpm tauri build --no-bundle
# binary: target/release/gambiit-app
```

`pnpm tauri build` additionally produces a Debian package (the configured bundle
target is `deb`). A PKGBUILD/AppImage is planned for the packaging phase.

`pnpm tauri dev` starts Vite and the Tauri window. The first move may take a few
seconds while the model thinks; the UI stays responsive and shows a thinking
indicator.

## Customize

Settings are grouped into four tabs: **Game** (opponent, engine, clock,
features), **Appearance** (themes, colours, pieces, interface), **Language**
(UI and agent language) and **Connection** (model and provider credentials). The Game tab
holds the settings that shape the match; the other tabs stay out of the way.

The Settings panel controls the opponent model, persona, Auto/Button response
mode, playing colour, board theme, piece set, language, sound, animations and
Stockfish candidate depth. Four themes ship now: Salon, Tournament, Midnight and
Custom.

### Theme editor

Select **Custom** in **Settings → Appearance** to open the theme studio.
Nine colours can be changed: background, panel, border, text, muted text, accent,
light square, dark square and board border. Changes preview live on the whole
app. **Save changes** stores the palette in the profile and in saved games
without restarting the current game.

### How settings are saved

- Appearance and interface changes (theme, colours, piece set, UI language,
  sound, animations, font size, chat and draw-offer toggles) are saved
  immediately and never restart the game.
- Clock changes apply to a new game, because they change the time both sides
  start with.
- Model, persona, difficulty or agent-language changes show a warning because
  they affect the current opponent from the next move. Choose **Save changes**
  to keep playing, or **Save & start new game**.
- Changing colour always requires a new game, so the panel says so and starts it
  when you save.
- **Save changes** keeps the settings panel open so you can keep editing. Only
  **Save & start new game** closes it.

### Piece sets

Five sets are built in: **Cburnett** (the default), **Livius**, **Meridian**,
**Meridian Shaded** and **Letter**. Choose one under **Settings → Appearance → Piece set**. Livius, Meridian and Meridian Shaded are CC0 sets by
Martin Sedlák; Letter is a CC0 typographic set made for Gambiit. Cburnett is
CC BY-SA 3.0.

To add your own set, drop a folder into:

```
~/.local/share/gambiit/pieces/
```

Name the files `wK`, `wQ`, `wR`, `wB`, `wN`, `wP`, `bK`, `bQ`, `bR`, `bB`, `bN`
and `bP`, with an `.svg`, `.png`, `.webp`, `.jpg` or `.jpeg` extension. For
example `~/.local/share/gambiit/pieces/classic/wN.svg`. Missing pieces fall back
to the built-in Cburnett set, so a partial set is fine. Use **Open pieces folder**
in Settings to find the directory and **Refresh sets** after adding files.

Piece images are loaded as data URLs and capped at 512 KB per file. The SVG is
rendered inside an `<img>`, so scripts in a piece file do not run.

See [`docs/APPEARANCE.md`](docs/APPEARANCE.md) for the full palette and piece-set
reference.

## Languages

The interface language and the agent’s response language are independent and
are selected in **Settings → Language**. This is useful when a model
writes better in English than in the UI language. Locale catalogs live in
`src/locales/*.json`; adding a new JSON file adds a new language without touching
code.

See [`docs/LOCALES.md`](docs/LOCALES.md) for the format and translation workflow.
English and Estonian are included.

## Saving and PGN

Every game is saved after each move. On the next launch Gambiit resumes the most
recent game automatically. The **▤ Games** button in the top bar opens the saved
game library, where you can open, rename or delete games.

Saved games live in:

```
~/.local/share/gambiit/games/
```

The **PGN** tab exports the current game (tags, moves and comments) and imports
standard PGN text. “Save .pgn” writes a copy to:

```
~/.local/share/gambiit/exports/
```

PGN import accepts standard SAN, comments in braces, `FEN`/`HumanColor` tags and
ignores variations. OpenCode settings are stored with each saved game.

## Post-game analysis

Open the **Analysis** tab in the game sidebar. Choose a depth (12 by default) and
press **Analyze game**. Stockfish re-reads every position once and reports:

- accuracy for White and Black;
- inaccuracies, mistakes and blunders;
- an evaluation graph from White's point of view;
- the best move and centipawn loss for each move, with a small board;
- the **Truth Lens**: the agent's claimed evaluation is compared with the
  engine, and claims that differ by 1.50 pawns or more are flagged.

The analysis runs on a separate Stockfish process, so it does not disturb the
opponent engine. A short game takes a few seconds; a long game at depth 16 can
take a minute. The board remains the source of truth: a flag says the agent's
claim did not match the engine, not that the move was illegal.

## Chat and draw offers

Both features are on by default and can be switched off independently in
**Settings → Game → Features**.

The **Chat** tab is a conversation with the opponent, separate from move
commentary. It uses the same persona and agent language, can bluff and needle,
and never changes the board. Chat history is saved with the game.

**Offer draw** sends a formal draw offer to the model. The opponent accepts or
declines based on the position and its persona. An accepted offer ends the game
as a draw; a declined offer leaves a dismissible note and, when chat is enabled,
the opponent's reply appears in the chat. Draw offers work even when chat is
disabled.

## Clocks

Open **Settings → Game → Clock** and enable **Use a chess clock**. Choose Bullet,
Blitz, Rapid, Classical or Custom, then set minutes and increment. The clock
starts when the game starts and runs for the side to move. The opponent's model
thinking time counts against its own clock, so long time controls are kinder to
slow models.

The clocks appear in both player bars. Under 20 seconds the display switches to
tenths and turns red. A flag ends the game immediately with a time-out result.
Clock values are saved with the game and restored on resume; time spent with
the app closed is not counted. PGN exports include a standard
`TimeControl "600+5"` tag when a clock is enabled.

Changing clock settings while a game is in progress asks for a new game.

## Models and API keys

The model selector lives in **Settings → Connection** and is searchable and
grouped by provider. In the Game tab the opponent card shows the selected model;
click it to jump straight to the picker. If a provider needs an API key, manage
it in the same tab. Keys are kept in
`~/.config/gambiit/credentials.json` with `0600` permissions and are never sent
back to the UI. Leaving the key empty uses OpenCode’s existing authentication.

See [`docs/CREDENTIALS.md`](docs/CREDENTIALS.md) for the environment-variable
mapping, storage details and OpenCode Go/Copilot caveats.

## Requirements

- Rust (stable, 1.85+; tested on 1.99)
- Node.js 20+ and pnpm 10+ for the Svelte frontend
- [OpenCode](https://opencode.ai) for AI opponents:
  - **OpenCode Desktop 2.x is recommended.** Gambiit auto-discovers its
    `opencode-cli` and uses the running desktop service, so it shares the same
    working authentication as the app.
  - A standalone `opencode` CLI also works; set `OPENCODE_BIN` if discovery
    picks the wrong binary.
- **Stockfish** for hybrid play and analysis — strongly recommended
  - Arch/CachyOS: `sudo pacman -S --needed stockfish`
- A model provider. The default is `opencode-go/deepseek-v4.1-flash`. A local
  Ollama model works offline:
  - `ollama/gemma4`
  - `ollama/qwen3.8:27b`
  - `localai/qwen3.8-9b-q8`

## Quick start

```bash
cargo build --release

# Check OpenCode and Stockfish
cargo run -p gambiit-cli -- probe

# Prove the OpenCode transport works (small prompt, no game)
cargo run -p gambiit-cli -- probe --prompt

# Play as White against the default OpenCode model
cargo run -p gambiit-cli -- play --agent opencode --color w

# Or use a local model
cargo run -p gambiit-cli -- play --agent opencode --model ollama/gemma4 --color w

# Play against Stockfish only, or against a mock opponent
cargo run -p gambiit-cli -- play --agent stockfish --color w
cargo run -p gambiit-cli -- play --agent mock --color w

# Perft sanity check
cargo run -p gambiit-cli -- perft 4
```

Useful options:

```bash
cargo run -p gambiit-cli -- play --help
# --depth 12 --multipv 3      Stockfish search settings
# --no-stockfish              legal-moves-only mode
# --max-plies 20              stop after N plies
# --persona "Trickster"       persona for the prompt
# --language en|et            agent commentary language
# --timeout 180               OpenCode request timeout
```

In the terminal, type a move as SAN (`e4`, `Nf3`, `O-O`) or UCI (`e2e4`,
`g1f3`). Type `moves` to list legal moves, `fen`, `pgn`, `resign`, or `q`.

`probe` prints which OpenCode binary Gambiit chose. The desktop `opencode-cli`
is preferred because it talks to the running OpenCode Desktop service and uses
the app's working authentication. If only the standalone `opencode` CLI is
available and its provider key has expired (`api_key_expired`), re-authenticate
with `opencode auth login`, choose another provider, or use a local Ollama
model. Gambiit never stores provider keys; OpenCode owns authentication.

## How it works

```
human move
   ↓
GameService (shakmaty) ── legal moves + FEN + history
   ↓
Stockfish (MultiPV 3) ── candidate moves + evaluations
   ↓
OpenCode prompt ── strict JSON: move, comment, candidates, plan
   ↓
validation (shakmaty) ── illegal/malformed → retry once → pause
   ↓
board + commentary
```

The agent is allowed to be theatrical. The game is not: every move returned by
the model is validated against the legal move list before it reaches the board.

## Development

```bash
cargo fmt --all
cargo test
cargo clippy --all-targets -- -D warnings
```

The terminal prototype is deliberately small. It is the risk spike that proves
the rules, Stockfish integration and OpenCode contract before the UI is built.

## Roadmap

- **Phase 1–2:** Tauri 2 + Svelte 5 shell, local two-player board, save/resume.
- **Phase 3:** agent in the UI: Auto / Button / Manual modes, model and persona
  selection, commentary panel, error recovery.
- **Phase 4:** themes, piece sets, theme editor, animations, sounds,
  accessibility, English/Estonian UI. (Theme editor, five built-in piece sets
  and custom piece-set folders are done.)
- **Phase 5:** takeback rules, draw/resign, result screen.
  (PGN export/import, the saved-game library, post-game analysis and clocks are
  done.)
- **Phase 6:** MCP mode, personas/chat, Truth Lens, Director's Cut, leaderboard,
  broadcast mode.
- **Phase 7:** packaging (PKGBUILD, `.desktop`), docs, performance.

## License

TBD. Code and bundled assets are licensed separately. Chess piece sets carry
their own license files under `src/assets/pieces/`: Cburnett is CC BY-SA 3.0;
Livius, Meridian, Meridian Shaded and Letter are CC0 1.0. Sounds must carry
their own license files.
