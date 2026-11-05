<script lang="ts">
  import type { ColorKey, Settings } from "./types";
  import { THEMES, applyTheme } from "./themes";
  import { applyFontScale } from "./settings";

  export let settings: Settings;
  export let models: string[] = [];
  export let initialColor: ColorKey = "white";
  export let onApply: (settings: Settings, humanColor: ColorKey) => void = () => {};
  export let onClose: () => void = () => {};

  let draft: Settings = { ...settings, fontScale: settings.fontScale ?? 1.2 };
  let humanColor: ColorKey = initialColor;

  $: applyFontScale(draft.fontScale);

  const personas = [
    "Theater Master",
    "Cold Calculator",
    "Trickster",
    "Panicker",
    "Coach",
    "Chaotic",
  ];

  const previews: Record<string, string[]> = {
    salon: ["#e8d3b0", "#8b5a2b", "#c9a227"],
    tournament: ["#eeeed2", "#769656", "#b9d48b"],
    midnight: ["#9fb4c9", "#3b4f63", "#58a6ff"],
  };

  function selectTheme(theme: string) {
    draft.theme = theme;
    applyTheme(theme);
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
    aria-label="Game settings"
    tabindex="-1"
  >
    <header class="head">
      <div>
        <p class="eyebrow">Game setup</p>
        <h2>Set the stage</h2>
      </div>
      <button class="close" type="button" aria-label="Close settings" on:click={onClose}>
        ✕
      </button>
    </header>

    <div class="body">
      <section class="section">
        <div class="section-title">
          <h3>Opponent</h3>
          <span>OpenCode model + persona</span>
        </div>

        <div class="grid two">
          <label class="field wide">
            <span>Model</span>
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
            <small>{models.length} models discovered</small>
          </label>

          <label class="field">
            <span>Persona</span>
            <select bind:value={draft.persona}>
              {#each personas as persona}
                <option value={persona}>{persona}</option>
              {/each}
            </select>
          </label>
        </div>

        <div class="field">
          <span>After your move</span>
          <div class="segmented">
            <button
              type="button"
              class:active={draft.responseMode === "auto"}
              on:click={() => (draft.responseMode = "auto")}
            >
              <strong>Auto</strong>
              <small>Agent answers immediately</small>
            </button>
            <button
              type="button"
              class:active={draft.responseMode === "button"}
              on:click={() => (draft.responseMode = "button")}
            >
              <strong>Button</strong>
              <small>You trigger “Agent move”</small>
            </button>
          </div>
        </div>

        <div class="field">
          <span>Play as</span>
          <div class="segmented compact">
            <button
              type="button"
              class:active={humanColor === "white"}
              on:click={() => (humanColor = "white")}
            >
              White
            </button>
            <button
              type="button"
              class:active={humanColor === "black"}
              on:click={() => (humanColor = "black")}
            >
              Black
            </button>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-title">
          <h3>Board & interface</h3>
          <span>Live preview</span>
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
              <strong>{theme.name}</strong>
              <small>{theme.description}</small>
            </button>
          {/each}
        </div>

        <div class="grid two">
          <label class="field">
            <span>Agent language</span>
            <select bind:value={draft.language}>
              <option value="en">English</option>
              <option value="et">Estonian</option>
            </select>
          </label>
          <label class="field">
            <span>Font size <b>{Math.round(draft.fontScale * 100)}%</b></span>
            <input
              type="range"
              min="0.9"
              max="1.5"
              step="0.05"
              bind:value={draft.fontScale}
            />
          </label>
        </div>

        <div class="field interface-field">
          <span>Interface</span>
          <div class="checks">
            <label class="check">
              <input type="checkbox" bind:checked={draft.sound} />
              <span>Sound</span>
            </label>
            <label class="check">
              <input type="checkbox" bind:checked={draft.animations} />
              <span>Animations</span>
            </label>
          </div>
        </div>
      </section>

      <section class="section">
        <div class="section-title">
          <h3>Engine</h3>
          <span>Stockfish candidates for hybrid play</span>
        </div>

        <div class="grid two">
          <label class="field">
            <span>Depth <b>{draft.stockfishDepth}</b></span>
            <input
              type="range"
              min="6"
              max="18"
              step="1"
              bind:value={draft.stockfishDepth}
            />
          </label>
          <label class="field">
            <span>Candidate moves <b>{draft.multipv}</b></span>
            <input
              type="range"
              min="1"
              max="5"
              step="1"
              bind:value={draft.multipv}
            />
          </label>
        </div>
        <p class="hint">
          Stockfish proposes the candidates; the model chooses one, explains it,
          and may still bluff about the evaluation. The board stays authoritative.
        </p>
      </section>
    </div>

    <footer class="foot">
      <button class="btn ghost" type="button" on:click={onClose}>Cancel</button>
      <button class="btn primary" type="button" on:click={apply}>
        Apply & start new game
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

  .field.wide {
    grid-column: 1 / -1;
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

  .interface-field {
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
    .themes {
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
