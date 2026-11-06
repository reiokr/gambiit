<script lang="ts">
  import { i18n } from "./i18n";
  import type { CustomPersona } from "./types";
  import { CUSTOM_PERSONA } from "./personas";
  import { DEFAULT_CUSTOM_PERSONA } from "./settings";

  export let persona: CustomPersona;
  export let onChange: (persona: CustomPersona) => void = () => {};

  type TraitKey = Exclude<keyof CustomPersona, "name" | "accent">;

  interface Trait {
    key: TraitKey;
    label: string;
    hint: string;
  }

  const TRAITS: Trait[] = [
    {
      key: "theatricality",
      label: "personaMixer.theatricality",
      hint: "personaMixer.theatricalityHint",
    },
    {
      key: "confidence",
      label: "personaMixer.confidence",
      hint: "personaMixer.confidenceHint",
    },
    {
      key: "warmth",
      label: "personaMixer.warmth",
      hint: "personaMixer.warmthHint",
    },
    {
      key: "humor",
      label: "personaMixer.humor",
      hint: "personaMixer.humorHint",
    },
    {
      key: "aggression",
      label: "personaMixer.aggression",
      hint: "personaMixer.aggressionHint",
    },
    {
      key: "verbosity",
      label: "personaMixer.verbosity",
      hint: "personaMixer.verbosityHint",
    },
    {
      key: "deception",
      label: "personaMixer.deception",
      hint: "personaMixer.deceptionHint",
    },
  ];

  function update<K extends keyof CustomPersona>(
    key: K,
    value: CustomPersona[K],
  ) {
    onChange({ ...persona, [key]: value });
  }

  function setTrait(key: TraitKey, value: number) {
    update(key, Math.min(10, Math.max(0, value)));
  }

  function reset() {
    onChange({ ...DEFAULT_CUSTOM_PERSONA });
  }
</script>

<div class="mixer">
  <header class="mixer-head">
    <div class="preview">
      <img src={CUSTOM_PERSONA.avatar} alt="" draggable="false" />
      <span style={`--persona-accent:${persona.accent}`}></span>
    </div>
    <div class="mixer-copy">
      <p class="eyebrow">{$i18n("personaMixer.eyebrow")}</p>
      <h4>{$i18n("personaMixer.title")}</h4>
      <p>{$i18n("personaMixer.hint")}</p>
    </div>
    <button class="reset" type="button" on:click={reset}>
      {$i18n("personaMixer.reset")}
    </button>
  </header>

  <div class="identity-fields">
    <label class="field">
      <span>{$i18n("personaMixer.name")}</span>
      <input
        class="persona-text"
        type="text"
        value={persona.name}
        maxlength="40"
        spellcheck="false"
        on:input={(event) => update("name", event.currentTarget.value)}
      />
    </label>
    <label class="field accent-field">
      <span>{$i18n("personaMixer.accent")}</span>
      <input
        class="persona-color"
        type="color"
        value={persona.accent}
        on:input={(event) => update("accent", event.currentTarget.value)}
      />
    </label>
  </div>

  <div class="traits">
    {#each TRAITS as trait}
      <label class="trait">
        <span class="trait-head">
          <span class="trait-label">{$i18n(trait.label)}</span>
          <b>{persona[trait.key]}</b>
        </span>
        <input
          type="range"
          min="0"
          max="10"
          step="1"
          value={persona[trait.key]}
          aria-valuetext={`${persona[trait.key]} / 10`}
          on:input={(event) => setTrait(trait.key, Number(event.currentTarget.value))}
        />
        <small>{$i18n(trait.hint)}</small>
      </label>
    {/each}
  </div>

  <p class="mixer-note">{$i18n("personaMixer.styleNote")}</p>
</div>

<style>
  .mixer {
    padding: 12px;
    margin-top: 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--panel-2);
  }

  .mixer-head {
    display: flex;
    align-items: flex-start;
    flex-wrap: wrap;
    gap: 11px;
    margin-bottom: 10px;
  }

  .preview {
    position: relative;
    flex: 0 0 auto;
  }

  .preview img {
    display: block;
    width: 56px;
    height: 56px;
    border: 1px solid color-mix(in srgb, var(--persona-accent) 55%, var(--border));
    border-radius: 50%;
  }

  .preview span {
    position: absolute;
    right: 0;
    bottom: 0;
    width: 13px;
    height: 13px;
    border: 2px solid var(--panel-2);
    border-radius: 50%;
    background: var(--persona-accent);
  }

  .mixer-copy {
    min-width: 0;
    flex: 1 1 220px;
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
    font-family: var(--font-display);
    font-size: calc(19px * var(--font-scale));
    font-weight: 500;
  }

  .mixer-copy p:not(.eyebrow) {
    max-width: 360px;
    margin: 3px 0 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .reset {
    flex: 0 0 auto;
    margin-left: auto;
    padding: 5px 8px;
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

  .identity-fields {
    display: grid;
    grid-template-columns: 1fr 100px;
    gap: 9px;
    margin-bottom: 8px;
  }

  .identity-fields .field {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 4px;
  }

  .identity-fields .field > span {
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    font-weight: 600;
  }

  .identity-fields .persona-text {
    width: 100%;
    height: 38px;
    padding: 0 11px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font: inherit;
    font-size: calc(12px * var(--font-scale));
  }

  .identity-fields .persona-text:focus {
    outline: none;
    border-color: var(--accent);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--accent) 22%, transparent);
  }

  .identity-fields .persona-color {
    width: 100%;
    height: 38px;
    padding: 3px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    cursor: pointer;
  }

  .identity-fields .persona-color::-webkit-color-swatch-wrapper {
    padding: 0;
  }

  .identity-fields .persona-color::-webkit-color-swatch {
    border: none;
    border-radius: 5px;
  }

  .identity-fields .persona-color:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }

  .traits {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px 14px;
  }

  .trait {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 2px;
  }

  .trait-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    font-weight: 600;
  }

  .trait-label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .trait b {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .trait input {
    width: 100%;
    accent-color: var(--accent);
  }

  .trait small {
    color: var(--faint);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.35;
  }

  .mixer-note {
    margin: 9px 0 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.5;
  }

  @media (max-width: 560px) {
    .traits {
      grid-template-columns: 1fr;
    }
  }
</style>
