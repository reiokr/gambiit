# Localization

Gambiit discovers locale catalogs automatically from **`src/locales/*.json`**.
The selected language controls both the interface and the language the agent is
asked to use for its commentary.

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

4. Restart `pnpm tauri dev` or rebuild. The language appears in the top-bar
   selector and in Settings → Board & interface → Language.

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

- The top-bar selector changes the interface immediately.
- The same selector updates the backend settings without resetting the game, so
  the next agent move uses the new language.
- The Settings panel language selector previews the language live. Its
  “Apply & start new game” action starts a fresh game with all new settings.
- Agent conversation language is driven by `_meta.name`, not the UI labels.

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
- `personas` — persona labels
- `themes` — theme names and descriptions

## RTL

`_meta.dir` is reserved for future right-to-left layout support. It is not
applied yet.
