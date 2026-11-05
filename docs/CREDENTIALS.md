# Models and provider credentials

## Model picker

Gambiit reads the model list from the installed OpenCode CLI (`opencode models`)
and shows a searchable picker in **Settings → Game → Opponent → Model**.

- Type to filter; the picker groups matches by provider.
- Click a model to select it, or type a custom `provider/model` string.
- **Refresh** reloads the model list.
- The first matches are shown for performance; narrow the search to reach the rest.

## Optional provider API key

OpenCode already owns normal authentication. A provider key in Gambiit is an
optional override for the current game's child process.

- Open **Settings → Connection** after choosing a model.
- The environment variable is pre-filled for common providers, but it is
  editable.
- Paste a key and click **Save key**. It is never sent back to the UI.
- Leave the key empty to use OpenCode's own login/auth store.
- The key is applied when you start a game (the Settings dialog's
  **Apply & start new game** action).

## Storage

The key is stored locally at:

```
~/.config/gambiit/credentials.json
```

The file is written with **0600** permissions and the directory with **0700**.
The key is stored as plain text in this local file — it is not encrypted. If you
prefer not to store keys there, use OpenCode's own authentication instead.

The frontend only receives credential status:

```json
{ "provider": "openai", "envVar": "OPENAI_API_KEY", "hasKey": true }
```

It never receives the key itself.

## Environment variables

Common mappings:

| Provider | Environment variable |
|---|---|
| `openai` | `OPENAI_API_KEY` |
| `anthropic` | `ANTHROPIC_API_KEY` |
| `openrouter` | `OPENROUTER_API_KEY` |
| `opencode` | `OPENCODE_API_KEY` |
| `opencode-go` | `OPENCODE_GO_API_KEY` |
| `google` | `GOOGLE_GENERATIVE_AI_API_KEY` |
| `groq` | `GROQ_API_KEY` |
| `mistral` | `MISTRAL_API_KEY` |
| `xai` | `XAI_API_KEY` |
| `deepseek` | `DEEPSEEK_API_KEY` |
| `localai` | `LOCALAI_API_KEY` |
| `ollama` | no key needed (local) |

The mapping is only a convenience. Some providers use different names; edit the
field to match the provider's documentation.

When a key is present and the desktop `opencode-cli` is used, Gambiit invokes it
with `--standalone` so the key is visible to the private child server. Without a
custom key it uses the normal OpenCode Desktop background service and its
existing authentication.

## OpenCode Go and GitHub Copilot

These providers may use the OpenCode Desktop app's own session/login. A custom
API key may not override that login. If the key is not honoured, use OpenCode's
auth flow (`opencode auth login`) and leave the Gambiit key empty.
