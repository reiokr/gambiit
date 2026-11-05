<script lang="ts">
  import type { ColorKey, Settings } from "./types";
  import { THEMES, applyTheme } from "./themes";
  import { applyFontScale } from "./settings";
  import {
    availableLocales,
    i18n,
    localeMeta,
    setLocale,
  } from "./i18n";
  import { PERSONAS } from "./personas";
  import { DIFFICULTIES, type DifficultyOption } from "./difficulty";

  export let settings: Settings;
  export let models: string[] = [];
  export let initialColor: ColorKey = "white";
  export let onApply: (settings: Settings, humanColor: ColorKey) => void = () => {};
  export let onClose: () => void = () => {};

  let draft: Settings = { ...settings, fontScale: settings.fontScale ?? 1.2 };
  let humanColor: ColorKey = initialColor;

  $: applyFontScale(draft.fontScale);

  const previews: Record<string, string[]> = {
    salon: ["#e8d3b0", "#8b5a2b", "#c9a227"],
    tournament: ["#eeeed2", "#769656", "#b9d48b"],
    midnight: ["#9fb4c9", "#3b4f63", "#58a6ff"],
  };

  function selectTheme(theme: string) {
    draft.theme = theme;
    applyTheme(theme);
  }

  function selectUiLanguage(code: string) {
    draft.uiLanguage = code;
    setLocale(code);
  }

  function selectAgentLanguage(code: string) {
    draft.agentLanguage = code;
    draft.agentLanguageName = localeMeta(code).name;
  }

  function selectDifficulty(level: DifficultyOption) {
    draft.difficulty = level.id;
    if (level.id !== "custom") {
      draft.stockfishDepth = level.depth;
      draft.multipv = level.multipv;
    }
  }

  function apply() {
    onApply({ ...draft }, humanColor);
  }
</script>

<div
  class="backdrop"
  role="presentation"
  on:click={(event) => {
    if (event.target === event.currentTarget) onClose();
  }}
>
  <div
    class="panel"
    role="dialog"
    aria-modal="true"
    aria-label={$i18n("settings.title")}
    tabindex="-1"
  >
    <header class="head">
      <div>
        <p class="eyebrow">{$i18n("settings.gameSetup")}</p>
        <h2>{$i18n("settings.title")}</h2>
      </div>
      <button
        class="close"
        type="button"
        aria-label={$i18n("settings.close")}
        on:click={onClose}
      >
        ✕
      </button>
    </header>

    <div class="body">
      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.opponent")}</h3>
          <span>{$i18n("settings.opponentHint")}</span>
        </div>

        <label class="field">
          <span>{$i18n("settings.model")}</span>
          <input
            list="gambiit-models"
            bind:value={draft.model}
            spellcheck="false"
            autocomplete="off"
            placeholder="provider/model"
          />
          <datalist id="gambiit-models">
            {#each models as model}
              <option value={model}></option>
            {/each}
          </datalist>
          <small>
            {$i18n("settings.modelsDiscovered", { count: models.length })}
          </small>
        </label>

        <div class="field persona-field">
          <span>{$i18n("settings.persona")}</span>
          <div class="persona-grid">
            {#each PERSONAS as persona}
              <button
                type="button"
                class="persona-card"
                class:active={draft.persona === persona.value}
                style={`--persona-accent:${persona.accent}`}
                on:click={() => (draft.persona = persona.value)}
              >
                <img src={persona.avatar} alt="" draggable="false" />
                <span class="persona-copy">
                  <strong>{$i18n(`personas.${persona.key}`)}</strong>
                  <small>
                    {$i18n(`personas.${persona.key}Description`)}
                  </small>
                </span>
              </button>
            {/each}
          </div>
          <small class="persona-hint">{$i18n("settings.personaHint")}</small>
        </div>

        <div class="field">
          <span>{$i18n("settings.afterMove")}</span>
          <div class="segmented">
            <button
              type="button"
              class:active={draft.responseMode === "auto"}
              on:click={() => (draft.responseMode = "auto")}
            >
              <strong>{$i18n("settings.auto")}</strong>
              <small>{$i18n("settings.autoHint")}</small>
            </button>
            <button
              type="button"
              class:active={draft.responseMode === "button"}
              on:click={() => (draft.responseMode = "button")}
            >
              <strong>{$i18n("settings.button")}</strong>
              <small>{$i18n("settings.buttonHint")}</small>
            </button>
          </div>
        </div>

        <div class="field">
          <span>{$i18n("settings.playAs")}</span>
          <div class="segmented compact">
            <button
              type="button"
              class:active={humanColor === "white"}
              on:click={() => (humanColor = "white")}
            >
              {$i18n("settings.white")}
            </button>
            <button
              type="button"
              class:active={humanColor === "black"}
              on:click={() => (humanColor = "black")}
            >
              {$i18n("settings.black")}
            </button>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.boardInterface")}</h3>
          <span>{$i18n("settings.livePreview")}</span>
        </div>

        <div class="themes">
          {#each THEMES as theme}
            <button
              type="button"
              class="theme"
              class:active={draft.theme === theme.id}
              on:click={() => selectTheme(theme.id)}
            >
              <span class="swatches">
                {#each previews[theme.id] ?? [] as color}
                  <i style={`background:${color}`}></i>
                {/each}
              </span>
              <strong>{$i18n(`themes.${theme.id}.name`)}</strong>
              <small>{$i18n(`themes.${theme.id}.description`)}</small>
            </button>
          {/each}
        </div>

        <div class="grid two">
          <label class="field">
            <span>{$i18n("settings.uiLanguage")}</span>
            <select
              value={draft.uiLanguage}
              on:change={(event) => selectUiLanguage(event.currentTarget.value)}
            >
              {#each availableLocales as language}
                <option value={language.code}>{language.nativeName}</option>
              {/each}
            </select>
            <small>{$i18n("settings.uiLanguageHint")}</small>
          </label>
          <label class="field">
            <span>{$i18n("settings.agentLanguage")}</span>
            <select
              value={draft.agentLanguage}
              on:change={(event) => selectAgentLanguage(event.currentTarget.value)}
            >
              {#each availableLocales as language}
                <option value={language.code}>{language.nativeName}</option>
              {/each}
            </select>
            <small>{$i18n("settings.agentLanguageHint")}</small>
          </label>
        </div>

        <div class="grid two">
          <label class="field">
            <span>
              {$i18n("settings.fontSize")}
              <b>{Math.round(draft.fontScale * 100)}%</b>
            </span>
            <input
              type="range"
              min="0.9"
              max="1.5"
              step="0.05"
              bind:value={draft.fontScale}
            />
          </label>
          <div class="field">
            <span>{$i18n("settings.interface")}</span>
            <div class="checks">
              <label class="check">
                <input type="checkbox" bind:checked={draft.sound} />
                <span>{$i18n("settings.sound")}</span>
              </label>
              <label class="check">
                <input type="checkbox" bind:checked={draft.animations} />
                <span>{$i18n("settings.animations")}</span>
              </label>
            </div>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.engine")}</h3>
          <span>{$i18n("settings.engineHint")}</span>
        </div>

        <div class="field">
          <span>{$i18n("settings.difficulty")}</span>
          <div class="difficulty-grid">
            {#each DIFFICULTIES as level}
              <button
                type="button"
                class="difficulty-card"
                class:active={draft.difficulty === level.id}
                on:click={() => selectDifficulty(level)}
              >
                <strong>{$i18n(`difficulty.${level.key}.name`)}</strong>
                <small>{$i18n(`difficulty.${level.key}.description`)}</small>
                {#if level.id !== "custom"}
                  <span class="difficulty-meta">
                    {$i18n("settings.engineMeta", {
                      count: level.multipv,
                      depth: level.depth,
                    })}
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>

        <div class="grid two custom-engine">
          <label class="field">
            <span>
              {$i18n("settings.depth")}
              <b>{draft.stockfishDepth}</b>
            </span>
            <input
              type="range"
              min="5"
              max="20"
              step="1"
              bind:value={draft.stockfishDepth}
              on:input={() => (draft.difficulty = "custom")}
            />
          </label>
          <label class="field">
            <span>
              {$i18n("settings.candidates")}
              <b>{draft.multipv}</b>
            </span>
            <input
              type="range"
              min="1"
              max="6"
              step="1"
              bind:value={draft.multipv}
              on:input={() => (draft.difficulty = "custom")}
            />
          </label>
        </div>
        <p class="hint">{$i18n("settings.engineNote")}</p>
      </section>
    </div>

    <footer class="foot">
      <button class="btn ghost" type="button" on:click={onClose}>
        {$i18n("settings.cancel")}
      </button>
      <button class="btn primary" type="button" on:click={apply}>
        {$i18n("settings.apply")}
      </button>
    </footer>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: grid;
    overflow-y: auto;
    padding: 32px 18px;
    place-items: center;
    background: rgba(5, 4, 3, 0.72);
    backdrop-filter: blur(6px);
  }

  .panel {
    width: min(760px, 100%);
    max-height: min(860px, calc(100vh - 64px));
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--panel);
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 22px 26px 16px;
    border-bottom: 1px solid var(--border);
  }

  .eyebrow {
    margin: 0 0 4px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(28px * var(--font-scale));
    font-weight: 500;
  }

  .close {
    display: grid;
    width: 34px;
    height: 34px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--muted);
    cursor: pointer;
  }

  .close:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .body {
    overflow-y: auto;
    padding: 22px 26px;
  }

  .section {
    padding-bottom: 22px;
    margin-bottom: 22px;
    border-bottom: 1px solid var(--border);
  }

  .section:last-child {
    padding-bottom: 0;
    margin-bottom: 0;
    border-bottom: 0;
  }

  .section-title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 14px;
  }

  .section-title h3 {
    margin: 0;
    font-size: calc(14px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.02em;
  }

  .section-title span {
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
  }

  .grid {
    display: grid;
    gap: 12px;
  }

  .grid.two {
    grid-template-columns: 1fr 1fr;
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: 6px;
    color: var(--muted);
    font-size: calc(11px * var(--font-scale));
  }

  .field > span {
    font-weight: 600;
  }

  .field b {
    color: var(--accent);
    font-family: var(--font-mono);
  }

  input:not([type="checkbox"]):not([type="range"]),
  select {
    width: 100%;
    height: 38px;
    padding: 0 11px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
  }

  input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }

  small {
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
  }

  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .segmented.compact {
    max-width: 260px;
  }

  .segmented button {
    display: flex;
    flex-direction: column;
    gap: 3px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--muted);
    text-align: left;
    cursor: pointer;
  }

  .segmented.compact button {
    align-items: center;
    padding: 9px 12px;
    text-align: center;
  }

  .segmented button strong {
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
  }

  .segmented button small {
    line-height: 1.35;
  }

  .segmented button.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    color: var(--text);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .themes {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 10px;
    margin-bottom: 14px;
  }

  .theme {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
    text-align: left;
    cursor: pointer;
  }

  .theme.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .theme strong {
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
  }

  .theme small {
    line-height: 1.35;
  }

  .swatches {
    display: flex;
    height: 26px;
    overflow: hidden;
    border-radius: 5px;
    margin-bottom: 4px;
  }

  .swatches i {
    flex: 1;
  }

  .persona-field {
    margin-top: 14px;
  }

  .persona-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }

  .persona-card {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 140ms ease,
      background 140ms ease,
      transform 140ms ease;
  }

  .persona-card:hover {
    border-color: var(--persona-accent, var(--accent));
    transform: translateY(-1px);
  }

  .persona-card.active {
    border-color: var(--persona-accent, var(--accent));
    background: color-mix(
      in srgb,
      var(--persona-accent, var(--accent)) 10%,
      var(--panel-2)
    );
    box-shadow: 0 0 0 1px
      color-mix(in srgb, var(--persona-accent, var(--accent)) 35%, transparent);
  }

  .persona-card img {
    width: 46px;
    height: 46px;
    flex: 0 0 46px;
    border-radius: 50%;
    background: var(--panel);
  }

  .persona-copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 3px;
  }

  .persona-copy strong {
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
    line-height: 1.25;
  }

  .persona-copy small {
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.4;
  }

  .persona-hint {
    margin-top: 6px;
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.4;
  }

  .difficulty-grid {
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 8px;
  }

  .difficulty-card {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 10px 11px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 140ms ease,
      background 140ms ease;
  }

  .difficulty-card:hover {
    border-color: var(--accent);
  }

  .difficulty-card.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .difficulty-card strong {
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
  }

  .difficulty-card small {
    color: var(--muted);
    line-height: 1.4;
  }

  .difficulty-meta {
    margin-top: 2px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .custom-engine {
    margin-top: 12px;
  }

  .checks {
    display: flex;
    gap: 18px;
  }

  .check {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--muted);
    cursor: pointer;
  }

  .check input {
    accent-color: var(--accent);
  }

  .hint {
    margin: 12px 0 0;
    color: var(--faint);
    font-size: calc(10.5px * var(--font-scale));
    line-height: 1.55;
  }

  .foot {
    display: flex;
    justify-content: flex-end;
    gap: 10px;
    padding: 16px 26px;
    border-top: 1px solid var(--border);
    background: var(--panel-2);
  }

  .btn {
    padding: 9px 16px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
    font-weight: 600;
    cursor: pointer;
  }

  .btn.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #17120b;
  }

  .btn.primary:hover {
    background: var(--accent-strong);
  }

  .btn.ghost:hover {
    border-color: var(--accent);
  }

  @media (max-width: 700px) {
    .grid.two,
    .themes,
    .persona-grid,
    .difficulty-grid {
      grid-template-columns: 1fr;
    }
    .backdrop {
      padding: 12px;
    }
    .panel {
      max-height: calc(100vh - 24px);
    }
    .head,
    .body,
    .foot {
      padding-left: 18px;
      padding-right: 18px;
    }
  }
</style>
