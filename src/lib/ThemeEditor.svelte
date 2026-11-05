<script lang="ts">
  import { i18n } from "./i18n";
  import type { CustomTheme } from "./types";

  export let theme: CustomTheme;
  export let onChange: (theme: CustomTheme) => void = () => {};
  export let onReset: () => void = () => {};

  type ThemeKey = keyof CustomTheme;

  interface ColorField {
    key: ThemeKey;
    label: string;
  }

  interface ColorGroup {
    title: string;
    fields: ColorField[];
  }

  const GROUPS: ColorGroup[] = [
    {
      title: "themeEditor.stage",
      fields: [
        { key: "bg", label: "themeEditor.background" },
        { key: "panel", label: "themeEditor.panel" },
        { key: "border", label: "themeEditor.border" },
      ],
    },
    {
      title: "themeEditor.ink",
      fields: [
        { key: "text", label: "themeEditor.text" },
        { key: "muted", label: "themeEditor.muted" },
        { key: "accent", label: "themeEditor.accent" },
      ],
    },
    {
      title: "themeEditor.board",
      fields: [
        { key: "boardLight", label: "themeEditor.boardLight" },
        { key: "boardDark", label: "themeEditor.boardDark" },
        { key: "boardBorder", label: "themeEditor.boardBorder" },
      ],
    },
  ];

  function set(key: ThemeKey, value: string) {
    if (!/^#[0-9a-f]{6}$/i.test(value)) return;
    const next: CustomTheme = { ...theme };
    next[key] = value.toLowerCase();
    onChange(next);
  }

  function setHex(key: ThemeKey, input: HTMLInputElement) {
    const value = input.value.trim();
    if (/^#[0-9a-f]{6}$/i.test(value)) {
      set(key, value);
    } else {
      input.value = theme[key];
    }
  }
</script>

<div class="editor">
  <div class="editor-head">
    <div>
      <p class="eyebrow">{$i18n("themeEditor.eyebrow")}</p>
      <h4>{$i18n("themeEditor.title")}</h4>
      <p>{$i18n("themeEditor.hint")}</p>
    </div>
    <button class="reset" type="button" on:click={onReset}>
      {$i18n("themeEditor.reset")}
    </button>
  </div>

  <div class="preview" style={`background:${theme.bg};border-color:${theme.border}`}>
    <span class="preview-panel" style={`background:${theme.panel}`}></span>
    <span class="preview-text" style={`background:${theme.text}`}></span>
    <span class="preview-muted" style={`background:${theme.muted}`}></span>
    <span class="preview-accent" style={`background:${theme.accent}`}></span>
    <span class="preview-board">
      <i style={`background:${theme.boardLight}`}></i>
      <i style={`background:${theme.boardDark}`}></i>
      <i style={`background:${theme.boardLight}`}></i>
      <i style={`background:${theme.boardDark}`}></i>
    </span>
  </div>

  <div class="groups">
    {#each GROUPS as group}
      <section class="group">
        <h5>{$i18n(group.title)}</h5>
        {#each group.fields as field}
          <label class="color-row">
            <span class="label">{$i18n(field.label)}</span>
            <input
              class="swatch"
              type="color"
              value={theme[field.key]}
              aria-label={$i18n(field.label)}
              on:input={(event) => set(field.key, event.currentTarget.value)}
            />
            <input
              class="hex"
              type="text"
              value={theme[field.key]}
              spellcheck="false"
              maxlength="7"
              aria-label={$i18n(field.label)}
              on:change={(event) => setHex(field.key, event.currentTarget)}
            />
          </label>
        {/each}
      </section>
    {/each}
  </div>
</div>

<style>
  .editor {
    padding: 12px;
    margin-top: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--panel-2);
  }

  .editor-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 9px;
  }

  .eyebrow {
    margin: 0 0 2px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.14em;
    text-transform: uppercase;
  }

  h4 {
    margin: 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(18px * var(--font-scale));
    font-weight: 500;
  }

  .editor-head p:not(.eyebrow) {
    max-width: 390px;
    margin: 3px 0 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .reset {
    flex: 0 0 auto;
    padding: 5px 9px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .reset:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .preview {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 7px 8px;
    margin-bottom: 9px;
    border: 1px solid;
    border-radius: 8px;
  }

  .preview-panel,
  .preview-text,
  .preview-muted,
  .preview-accent {
    display: block;
    height: 13px;
    border-radius: 4px;
  }

  .preview-panel {
    width: 38px;
  }

  .preview-text {
    width: 24px;
  }

  .preview-muted {
    width: 24px;
  }

  .preview-accent {
    width: 20px;
  }

  .preview-board {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    width: 54px;
    height: 16px;
    margin-left: auto;
    overflow: hidden;
    border-radius: 3px;
  }

  .preview-board i {
    display: block;
  }

  .groups {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
  }

  .group h5 {
    margin: 0 0 5px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .color-row {
    display: grid;
    grid-template-columns: 1fr 24px 64px;
    align-items: center;
    gap: 5px;
    padding: 2px 0;
  }

  .label {
    overflow: hidden;
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .swatch {
    width: 24px;
    height: 21px;
    padding: 1px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--panel);
    cursor: pointer;
  }

  .hex {
    width: 100%;
    min-width: 0;
    padding: 3px 4px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--panel);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    text-transform: lowercase;
  }

  @media (max-width: 640px) {
    .groups {
      grid-template-columns: 1fr;
    }
  }
</style>
