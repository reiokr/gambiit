# Local read-only API

Gambiit runs a small HTTP API on `127.0.0.1` while the desktop app is open. The
first version is deliberately **read-only**: it exposes the game state and a
live SSE stream, but no endpoint can move a piece, send chat, resign or change
settings.

- API version: `1`
- Bind address: `127.0.0.1` only
- Authentication: random bearer token
- Senior endpoint: `GET /api/health`
- Protected endpoints: `GET /api/state`, `GET /api/events`
- CORS: only the Tauri webview origins
- No mutation endpoints

## Discovery file

At startup Gambiit writes:

```text
$XDG_RUNTIME_DIR/gambiit/session.json
```

If `XDG_RUNTIME_DIR` is not set, it falls back to
`$TMPDIR/gambiit-runtime/gambiit/session.json`.

The directory is created with mode `0700`; the file is written atomically with
mode `0600`.

```json
{
  "url": "http://127.0.0.1:42619",
  "port": 42619,
  "token": "64-hex-character-token",
  "pid": 293035,
  "version": "0.1.0",
  "apiVersion": 1,
  "startedAt": 1793995481
}
```

Read the URL and token from this file. The file is overwritten on the next
launch. A crash or forced kill can leave a stale file behind; treat connection
refused as “the app is not running” and do not reuse the token.

## Authentication

Send the token as a bearer header:

```bash
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:$PORT/api/state
```

Browser `EventSource` cannot set headers, so the same token may be passed as a
query parameter:

```text
http://127.0.0.1:42619/api/events?token=64-hex-character-token
```

The token is a local capability. Do not put it in a public web page, a shared
stream overlay URL or a screenshot. The API is read-only, but the state includes
chat, coach notes and settings.

## Endpoints

### `GET /api/health`

Unauthenticated. Useful for checking whether the app and API are up.

```json
{
  "ok": true,
  "name": "gambiit",
  "version": "0.1.0",
  "apiVersion": 1,
  "readOnly": true
}
```

### `GET /api/state`

Bearer token required. Returns the same snapshot the Tauri UI receives:
FEN, turn, board, legal moves, history, last move, commentary, chat, coach notes,
clock, settings and agent status.

```bash
curl -s -H "Authorization: Bearer $TOKEN" \
  "http://127.0.0.1:$PORT/api/state"
```

The response is `application/json` and is not cached. The full snapshot is
intentionally returned so overlays and tools can render the same authoritative
state as the app.

### `GET /api/events`

Bearer token required. Returns a Server-Sent Events stream.

- Event name: `state`
- Data: the same JSON snapshot as `/api/state`
- A new event is sent when the game state changes (checked every 500 ms).
  Clock values are quantized to display precision, so a running clock does not
  force a full snapshot on every poll.
- A `keepalive` comment is sent every 15 seconds.
- A new client immediately receives the current state.
- No historical replay or `Last-Event-ID` resume in v1.

```bash
curl -N -H "Authorization: Bearer $TOKEN" \
  "http://127.0.0.1:$PORT/api/events"
```

```js
const source = new EventSource(
  `http://127.0.0.1:${port}/api/events?token=${encodeURIComponent(token)}`,
);
source.addEventListener("state", (event) => {
  const state = JSON.parse(event.data);
  console.log(state.fen, state.statusText);
});
```

## Security model

- The server binds only to `127.0.0.1`, never `0.0.0.0`.
- A random 64-character token is generated per launch with UUID v4 material.
- Every state/SSE request requires the token; missing or wrong tokens get `401`.
- Token comparison is constant-time.
- The discovery directory is `0700` and the file is `0600`.
- CORS allows only the Tauri webview origins:
  `tauri://localhost`, `http://tauri.localhost`, `https://tauri.localhost`.
  There is no wildcard origin.
- Responses are `Cache-Control: no-store` and
  `X-Content-Type-Options: nosniff`.
- There are no mutation endpoints, no shell, no file access and no provider API
  keys in the API.
- The API does not bypass game validation because it cannot submit moves at all.

The API is local, not private from other processes running as the same user. Any
local process that can read the discovery file can read the game state. That is
the intended trust model: the token is a local capability, not a remote secret.

## Errors

| Status | Meaning |
|---|---|
| `200` | Success |
| `401` | Missing or invalid bearer token |
| `404` | Unknown endpoint |
| `500` | Snapshot serialization failed |

## Deliberate limitations

- **No mutations.** New game, move, agent move, takeback, resign, draw, chat,
  coach, analysis and settings are not exposed. They remain Tauri-only.
- **No browser shell yet.** `gambiit serve` and a browser-hosted UI are future
  work.
- **Full snapshots only.** There is no compact overlay-specific payload yet.
- **No API version negotiation.** The discovery file advertises `apiVersion: 1`;
  future breaking changes can use a new version and path.
- **Stale discovery files.** A crash can leave `session.json` behind. The next
  launch overwrites it; clients should treat connection refused as unavailable.

## Why read-only first

Read-only endpoints are useful for OBS overlays, stream widgets, local scripts
and automated checks without creating a second way to mutate a live game. The
mutation surface needs a much larger test matrix: turn ownership, clocks,
takeback policy, draw/resign, chat, agent sessions, persistence and concurrent
UI/API writes. Those endpoints can be added later on top of the same snapshot
and validation layer.
