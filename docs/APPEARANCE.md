# Appearance: themes and piece sets

Gambiit keeps appearance in `Settings → Appearance`. Everything here is
local to the profile and is stored with saved games.

## Theme editor

Choose the **Custom** theme card to open the theme studio. The editor exposes
nine colours:

| Group | Colours |
| --- | --- |
| Stage | background, panel, border |
| Ink | text, muted text, accent |
| Board | light square, dark square, board border |

Changes preview immediately across the app. **Save changes** saves the palette in
the current profile and in the game settings without restarting the game.
**Reset palette** restores the Salon palette. **Cancel** restores the palette that
was active before the settings panel opened.

Appearance and interface settings (theme, colours, piece set, UI language, sound,
animations, font size) are always saved without a restart. Model, persona,
difficulty and agent-language changes show a warning because they affect the
current opponent from the next move; **Save & start new game** is offered next to
**Save changes**. Changing colour requires a new game.

The custom palette lives in `Settings.customTheme` (camelCase in JSON) and is
used only when `Settings.theme` is `"custom"`.

## Built-in piece sets

Gambiit ships five vector sets:

| Set | Style | License |
| --- | --- | --- |
| Cburnett | detailed classic, the default | CC BY-SA 3.0 |
| Livius | bold flat design | CC0 1.0 |
| Meridian | clean 1952 newspaper-inspired design | CC0 1.0 |
| Meridian Shaded | Meridian with soft gradients | CC0 1.0 |
| Letter | typographic tokens with piece letters | CC0 1.0 |

Livius, Meridian and Meridian Shaded are by Martin Sedlák
(<https://github.com/kmar/chess_svg_piece_sets>). Letter was created for
Gambiit. Attribution files sit next to the assets under `src/assets/pieces/`.

## Custom piece sets

A piece set is a folder under:

```
~/.local/share/gambiit/pieces/
```

For example:

```
~/.local/share/gambiit/pieces/
└── classic/
    ├── wK.svg
    ├── wQ.svg
    ├── wR.svg
    ├── wB.svg
    ├── wN.svg
    ├── wP.svg
    ├── bK.svg
    ├── bQ.svg
    ├── bR.svg
    ├── bB.svg
    ├── bN.svg
    └── bP.svg
```

File names are case-sensitive. The last two characters are the piece code:

| Code | Piece |
| --- | --- |
| `K` | king |
| `Q` | queen |
| `R` | rook |
| `B` | bishop |
| `N` | knight |
| `P` | pawn |

The first character is `w` for White and `b` for Black. Supported extensions,
tried in this order: `.svg`, `.png`, `.webp`, `.jpg`, `.jpeg`.

A partial set is allowed. Any missing piece falls back to the built-in Cburnett
art, so you can start with a single piece file.

Limits:

- each piece file may be at most **512 KB**;
- a preview file may be at most **256 KB**;
- the folder name is used as the display name (`my-set` becomes “My Set”);
- folder names may contain ASCII letters, digits, `-` and `_` only.

After adding or changing files, press **Refresh sets** in Settings. Use **Open
pieces folder** to create and open `~/.local/share/gambiit/pieces/`.

## How pieces are loaded

The Rust backend reads the selected set and returns data URLs for the available
pieces. The frontend keeps the built-in URLs as a fallback per piece. Piece
images are rendered inside `<img>` elements; scripts inside an SVG are not
executed. Selecting a set updates the board, player bars and promotion picker
immediately.

The selected folder name is stored as `Settings.pieceSet`; `"cburnett"` means the
built-in set.

## Adding a new built-in theme

Built-in themes are CSS variable blocks in `src/app.css`:

```css
[data-theme="salon"] {
  --bg: #14110e;
  --panel: #201a15;
  --accent: #c9a227;
  /* … */
}
```

Add the id to `THEMES` in `src/lib/themes.ts`, add `themes.<id>.name` and
`themes.<id>.description` to every locale catalog, and add a swatch preview in
`src/lib/SettingsPanel.svelte` if you want one. Custom themes use inline CSS
variables and do not need a `data-theme` block.
