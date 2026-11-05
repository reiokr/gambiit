# Localization

Gambiit discovers locale catalogs automatically from **`src/locales/*.json`**.
The interface language and the agent response language are selected
independently in **Settings → Board & interface**. This lets the UI stay in one
language while the model answers in another (for example, an Estonian UI with an
English agent because the model writes better in English).

## Add a language

1. Copy `src/locales/en.json` to a new file, for example `src/locales/de.json`.
2. Translate the values (not the keys).
3. Set the metadata:

```json
"_meta": {
  "name": "German",
  "nativeName": "Deutsch"
}
```

- `name` is the English language name. It is sent to the agent prompt, e.g.
  “Commentary language: German”.
- `nativeName` is what appears in the language selector, e.g. “Deutsch”.
- The filename is the language code used in settings (`de`).

4. Restart `pnpm tauri dev` or rebuild. The language appears in both language
   selectors in Settings → Board & interface.

No Rust or Svelte changes are needed.

## Missing keys

If a key is missing in the selected language, Gambiit falls back to `en.json`.
This lets translators ship a partial catalog without breaking the UI.

## Placeholders

Some strings contain placeholders:

```json
"thinking": "{name} is thinking…",
"modelsDiscovered": "{count} models discovered"
```

Keep the braces and names unchanged; only translate the surrounding text.

## Language behaviour

- **Interface language** changes the app UI. The Settings panel previews it
  live; “Apply & start new game” applies it together with the rest of the
  settings.
- **Agent response language** only changes the prompt language. It does not
  change the UI and can differ from it.
- Agent conversation language is driven by `_meta.name`, not the UI labels.
- There is no language switcher in the game window; all language settings live
  under Settings.

## Sections to translate

The catalog is nested by feature:

- `app` — shell, buttons, result screen, tabs
- `board` — board accessibility and promotion dialog
- `piece` — piece names for screen readers
- `player` — player-strip status
- `side` / `status` — game result and turn text
- `moves` — move-list empty state
- `commentary` — theater panel
- `settings` — settings dialog
- `personas` — persona names and descriptions
- `difficulty` — Stockfish difficulty presets
- `themes` — theme names and descriptions

## RTL

`_meta.dir` is reserved for future right-to-left layout support. It is not
applied yet.
