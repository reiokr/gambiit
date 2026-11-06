# Gambiit MCP transport

This document describes the MCP (Model Context Protocol) transport used by the
Gambiit opponent. It covers the architecture, the tools, the turn lifecycle,
security, testing and troubleshooting.

MCP is optional. Prompt mode remains the default and the fallback.

## 1. What MCP is used for

In MCP mode, Gambiit does not ask the model for one JSON answer. It starts the
same OpenCode model with a temporary project that registers a local Gambiit MCP
server. The model can inspect the board, request analysis and submit exactly one
typed response for the current turn.

There are three turn kinds:

| Turn kind | Submission tool | Used for |
|---|---|---|
| `move` | `submit_move` | The opponent's chess move and Theater commentary |
| `chat` | `submit_chat` | One in-character chat reply |
| `draw` | `submit_draw` | Accepting or declining a draw offer |

The bridge rejects a submission for the wrong turn type. For example, a
`submit_move` call during a chat turn returns:

```text
This is a chat turn. Call submit_chat instead.
```

## 2. Architecture

```text
Gambiit UI
   │
   ▼
Tauri Session ── prepares TurnContext ──► local TCP bridge (127.0.0.1, random port)
   │                                          ▲
   │ writes temporary opencode.json           │ token-checked tool calls
   ▼                                          │
OpenCode CLI ── starts the app binary ──► MCP stdio server (--mcp-server)
   │
   └── model calls tools such as get_state or submit_move
```

Key points:

- The bridge is owned by the running Gambiit process.
- The MCP server is the same Gambiit binary, launched with `--mcp-server`.
- The MCP server speaks JSON-RPC over stdio and forwards tool calls to the TCP
  bridge.
- The bridge stores only the current turn and one pending submission. It never
  applies a move itself.
- The Tauri session validates and applies the submission after the model exits.

## 3. Requirements

- OpenCode Desktop or the OpenCode CLI, discoverable by Gambiit, or set
  `OPENCODE_BIN=/path/to/opencode`.
- A model that supports tool calls.
- Stockfish for candidate moves, the `get_analysis` current-position read, the
  coach and post-game analysis. Without Stockfish the game can start, but the
  full opponent/analysis experience is reduced.

Enable it in **Settings → Connection → Agent mode → MCP**. Prompt mode remains
available in the same place. Gambiit launches OpenCode automatically for each
turn; you do not need to run a separate OpenCode session or type moves in a
terminal.

## 4. Turn lifecycle

A move turn:

1. The human moves. The session prepares Stockfish candidates and applies the
   persona's bounded style ordering.
2. The session calls `bridge.begin(TurnContext { kind: Move, ... })`.
3. The session writes a temporary `opencode.json` and runs the model with the
   MCP prompt.
4. OpenCode starts `gambiit --mcp-server` with `GAMBIIT_MCP_URL` and
   `GAMBIIT_MCP_TOKEN`.
5. The model calls tools. `submit_move` validates the move against
   `gambiit-core` and the Stockfish candidate pool, then stores a
   `MoveSubmission`.
6. The session calls `bridge.take_submission()`, clears the bridge and applies
   the move. Illegal moves and moves outside the candidate pool are rejected.

A chat turn:

1. The human sends a chat message.
2. The session prepares a `TurnContext { kind: Chat, ... }` with the current
   game, legal moves, candidates, clock and analysis summary.
3. The model calls `submit_chat` with a non-empty `message`.
4. The session stores the reply as an agent chat message.

A draw turn:

1. The human offers a draw.
2. The session prepares a `TurnContext { kind: Draw, ... }`.
3. The model calls `submit_draw` with `accept` and a non-empty `comment`.
4. The session accepts the draw or records the decline and adds the comment to
   chat when chat is enabled.

Only one turn is active at a time. Starting a new turn clears any pending
submission. A second submission in the same turn is rejected.

## 5. Tool reference

The MCP server advertises all tools; the current turn kind decides which
submission tool is valid.

### `get_state`

Read-only. Returns the current board and game state.

```json
{
  "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
  "turn": "black",
  "humanColor": "white",
  "agentColor": "black",
  "turnKind": "move",
  "status": "Black to move.",
  "gameOver": false,
  "history": "1. e4",
  "clock": {
    "whiteMs": 600000,
    "blackMs": 600000,
    "incrementMs": 5000,
    "running": "black"
  }
}
```

`clock` is `null` when the game has no clock.

### `get_legal_moves`

Returns every legal move for the side to move:

```json
[
  {
    "uci": "e7e5",
    "san": "e5",
    "from": "e7",
    "to": "e5",
    "promotion": null,
    "piece": "p",
    "captured": null,
    "isCastle": false,
    "isEnPassant": false
  }
]
```

### `get_candidates`

Returns the Stockfish candidates prepared for this turn. The first candidate is
the persona's bounded style preference.

```json
[
  { "uci": "c7c5", "san": "c5", "score": "-0.24", "depth": 11 },
  { "uci": "e7e5", "san": "e5", "score": "-0.28", "depth": 11 }
]
```

Scores are from the side-to-move perspective. The list can be empty when
Stockfish is unavailable.

### `get_clock`

Returns the clock object shown in `get_state`, or `null`.

### `get_analysis`

Read-only. Returns the latest saved post-game analysis summary, plus the current
position read.

```json
{
  "available": true,
  "summary": {
    "depth": 12,
    "analyzedPlies": 42,
    "agentAccuracy": 78.4,
    "agentMoves": 21,
    "agentBlunders": 1,
    "agentMistakes": 2,
    "agentInaccuracies": 3,
    "truthLensFlags": 1
  },
  "position": {
    "fen": "...",
    "turn": "white",
    "status": "White to move.",
    "score": "+0.31",
    "perspective": "white",
    "best": { "uci": "g1f3", "san": "Nf3", "score": "+0.31", "depth": 11 },
    "candidates": [
      { "uci": "g1f3", "san": "Nf3", "score": "+0.31", "depth": 11 }
    ]
  }
}
```

`available` is `false` and `summary` is `null` until post-game analysis has been
run. `best` and `candidates` can be `null`/empty when Stockfish is unavailable.
This tool does **not** run a new analysis and does not change the board.

### `submit_move`

Valid only during a move turn.

```json
{
  "move": "c7c5",
  "comment": "The centre is a stage; I enter from the wings.",
  "eval": "-0.2",
  "plan": "Invite White to overreach.",
  "candidates": [
    { "move": "c7c5", "why": "asymmetry and counterplay" },
    { "move": "e7e5", "why_rejected": "too symmetrical" }
  ]
}
```

Required: `move` and non-empty `comment`.

Returns `{ "uci": "c7c5", "san": "c5" }` on success.

Validation:

- the move must be legal;
- if a candidate pool is present, the move must be one of its UCI moves;
- the response is stored, not applied directly by the bridge.

### `submit_chat`

Valid only during a chat turn.

```json
{ "message": "The position is quiet, but the evening is young." }
```

Required: a non-empty `message`, truncated to 4000 characters by the bridge.
Returns `{ "submitted": true }`.

### `submit_draw`

Valid only during a draw-offer turn.

```json
{
  "accept": false,
  "comment": "A draw? The curtain has barely risen."
}
```

Required: boolean `accept` and a non-empty `comment`, truncated to 800
characters. Returns `{ "accepted": false }`.

## 6. Prompts and fast path

The MCP prompts include the board, move history, game status and, for move
turns, the candidate list. The model is told to use the relevant submission tool
exactly once and then stop.

The move prompt fast path is:

1. Use the board and candidates already in the prompt; do not call `get_state`
   mechanically.
2. Optionally call `get_state`, `get_candidates`, `get_clock` or
   `get_analysis`.
3. Call `submit_move` once with the chosen candidate, commentary and optional
   eval/plan/candidates.
4. Stop.

Chat and draw prompts follow the same pattern with `submit_chat` and
`submit_draw`. Chat and draw prompts may call `get_analysis` for an honest engine
read.

## 7. Security model

- The bridge binds `127.0.0.1` on a random port.
- Every request carries a random token; invalid tokens are rejected.
- Socket reads and writes have a 30-second timeout.
- The temporary OpenCode project directory is created with mode `0700`; the
  `opencode.json` file is created with mode `0600`.
- The bridge cannot mutate the game. It stores at most one typed submission,
  and the Tauri session validates it before applying anything.
- Move legality is always checked by `gambiit-core`; candidate-pool membership
  is checked when Stockfish candidates exist.
- Wrong-turn submissions and duplicate submissions are rejected.

The MCP transport does not grant shell or file access to the model on its own.
The temporary project config registers only the Gambiit MCP server. OpenCode's
own built-in tools and global plugins remain governed by the user's OpenCode
configuration; Gambiit does not disable them for you. If you need strict
isolation, run Gambiit with a dedicated OpenCode profile or agent that disables
the tools you do not want.

## 8. Files and environment variables

Temporary project config:

```text
$TMPDIR/gambiit-mcp-<pid>/opencode.json
```

The file has this shape:

```json
{
  "mcp": {
    "gambiit": {
      "type": "local",
      "command": ["/path/to/gambiit", "--mcp-server"],
      "environment": {
        "GAMBIIT_MCP_URL": "127.0.0.1:PORT",
        "GAMBIIT_MCP_TOKEN": "random-token"
      },
      "enabled": true
    }
  }
}
```

Environment variables:

| Variable | Purpose |
|---|---|
| `GAMBIIT_MCP_URL` | Bridge address, set by Gambiit for the MCP server |
| `GAMBIIT_MCP_TOKEN` | Bridge token, set by Gambiit for the MCP server |
| `GAMBIIT_MCP_EXE` | Override the MCP server binary; used by smoke tests |
| `OPENCODE_BIN` | Override OpenCode binary discovery |

The model is started in the temporary project directory so OpenCode picks up
that project config. The desktop CLI ignored `OPENCODE_CONFIG` in testing, which
is why Gambiit uses a temporary project directory.

## 9. Troubleshooting

### “MCP bridge unavailable” at startup

The app could not bind a local TCP port. Check that localhost networking is
available and that another process is not interfering. Prompt mode still works.

### “No active Gambiit turn”

A tool was called outside an active turn or after the bridge was cleared. The
model should retry inside the current turn; if it persists, switch to prompt mode
and report the model and prompt.

### “This is a chat turn. Call submit_chat instead.”

The model used the wrong submission tool. The error names the correct tool.
Repeated failures usually mean the model is not following tool instructions.

### “A response has already been submitted for this turn.”

The model called a submission tool twice. The first submission is kept; the
second is rejected. The model should stop after a successful submission.

### “outside the Stockfish candidate pool”

The move was legal but weaker than the allowed near-best pool. The model must
choose a candidate. This protects the difficulty ceiling and the bounded persona
style.

### “The agent did not call submit_move / submit_chat / submit_draw.”

The model replied in text or called no submission tool. Check that the model
supports tool calls. Prompt mode is the fallback.

### OpenCode not found

Install `opencode` / `opencode-desktop`, or set `OPENCODE_BIN` to the CLI path.

### MCP is slow

Tool-calling models can make several calls before submitting. The fast path
includes the board and candidates in the first prompt, so usually only the
submission tool is needed. Prompt mode is faster and more predictable.

### Stale temporary directories

`gambiit-mcp-<pid>` directories under the system temp directory are safe to
remove after the app exits. They contain only the temporary OpenCode project
config.

## 10. Testing

Bridge unit tests:

```bash
cargo test -p gambiit-app mcp::tests
```

They cover state and candidate reads, move validation, chat/draw/analysis
submissions, wrong-turn rejection and the advertised tool list.

Real-model smoke tests (ignored by default because they spend provider tokens):

```bash
cargo test -p gambiit-app -- --ignored mcp_two_turns_smoke --nocapture
cargo test -p gambiit-app -- --ignored mcp_chat_draw_smoke --nocapture
```

The first plays two consecutive MCP moves. The second plays one move, sends a
chat message and makes a draw offer through MCP. Both use the built app binary
as the MCP server and `GAMBIIT_MCP_EXE` to point at it.

## 11. Prompt mode fallback

Prompt mode sends one structured JSON prompt per turn and validates the answer.
It is the default because it is faster and works with models that do not support
tool calls well.

Use MCP when you want the model to reason with tools and, in particular, when
you want it to consult `get_analysis` or choose between candidate moves through
the MCP interface. Both transports enforce the same board, rules, candidate pool
and persona bounds.
