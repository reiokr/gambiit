<script lang="ts">
  import type { ColorKey, Settings } from "./types";
  import { i18n } from "./i18n";
  import { CUSTOM_PERSONA, PERSONAS } from "./personas";
  import PersonaMixer from "./PersonaMixer.svelte";
  import ModelBadge from "./ModelBadge.svelte";
  import { TIME_CONTROLS, clockLabel } from "./clock";
  import { DIFFICULTIES, type DifficultyOption } from "./difficulty";

  export let draft: Settings;
  export let humanColor: ColorKey = "white";
  export let onChange: (patch: Partial<Settings>) => void = () => {};
  export let onHumanColor: (color: ColorKey) => void = () => {};
  export let onOpenModelPicker: () => void = () => {};

  function modelName(model: string): string {
    return model.includes("/") ? model.split("/").pop() || model : model || "—";
  }

  function providerName(model: string): string {
    return model.includes("/") ? model.split("/")[0] : "";
  }

  function bluffKey(value: number): string {
    if (value <= 15) return "settings.bluffHonest";
    if (value <= 39) return "settings.bluffMild";
    if (value <= 64) return "settings.bluffTheatrical";
    if (value <= 84) return "settings.bluffBold";
    return "settings.bluffUnreliable";
  }

  function selectDifficulty(level: DifficultyOption) {
    if (level.id === "custom") {
      onChange({ difficulty: "custom" });
      return;
    }
    onChange({
      difficulty: level.id,
      stockfishDepth: level.depth,
      multipv: level.multipv,
    });
  }

  function selectTimeControl(id: string) {
    const control = TIME_CONTROLS.find((option) => option.id === id);
    if (control && id !== "custom") {
      onChange({
        timeControl: id,
        clockInitialMs: control.initialMs,
        clockIncrementMs: control.incrementMs,
      });
    } else {
      onChange({ timeControl: id });
    }
  }

  function setClockMinutes(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const minutes = Math.min(180, Math.max(0.5, Number(input.value) || 10));
    onChange({
      clockInitialMs: Math.round(minutes * 60_000),
      timeControl: "custom",
    });
  }

  function setClockIncrement(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const seconds = Math.min(60, Math.max(0, Number(input.value) || 0));
    onChange({
      clockIncrementMs: Math.round(seconds * 1000),
      timeControl: "custom",
    });
  }
</script>

        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.opponent")}</h3>
            <span>{$i18n("settings.opponentHint")}</span>
          </div>
        <button class="opponent-model" type="button" on:click={onOpenModelPicker}>
          <ModelBadge model={draft.model} size="md" />
          <span class="opponent-model-copy">
            <span class="opponent-model-label">{$i18n("settings.model")}</span>
            <strong>{modelName(draft.model)}</strong>
            <small>{providerName(draft.model)}</small>
          </span>
          <span class="opponent-model-action">{$i18n("settings.changeModel")} →</span>
        </button>
<div class="field persona-field">
          <span>{$i18n("settings.persona")}</span>
          <div class="persona-grid">
            {#each PERSONAS as persona}
              <button
                type="button"
                class="persona-card"
                class:active={draft.persona === persona.value}
                style={`--persona-accent:${persona.accent}`}
                on:click={() => onChange({ persona: persona.value })}
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
            <button
              type="button"
              class="persona-card"
              class:active={draft.persona === "Custom"}
              style={`--persona-accent:${draft.customPersona.accent}`}
              on:click={() => onChange({ persona: "Custom" })}
            >
              <img src={CUSTOM_PERSONA.avatar} alt="" draggable="false" />
              <span class="persona-copy">
                <strong>
                  {draft.customPersona.name.trim() ||
                    $i18n("personas.custom")}
                </strong>
                <small>{$i18n("personas.customDescription")}</small>
              </span>
            </button>
          </div>
          {#if draft.persona === "Custom"}
            <PersonaMixer
              persona={draft.customPersona}
              onChange={(next) => onChange({ customPersona: next })}
            />
          {/if}
          <small class="persona-hint">{$i18n("settings.personaHint")}</small>
        </div>

        <label class="field persona-style-field">
          <span>{$i18n("settings.personaStyle")}</span>
          <select
            value={draft.personaStyle}
            on:change={(event) =>
              onChange({
                personaStyle: event.currentTarget
                  .value as Settings["personaStyle"],
              })}
          >
            <option value="off">{$i18n("settings.personaStyleOff")}</option>
            <option value="subtle">{$i18n("settings.personaStyleSubtle")}</option>
            <option value="theatrical">
              {$i18n("settings.personaStyleTheatrical")}
            </option>
            <option value="strong">{$i18n("settings.personaStyleStrong")}</option>
          </select>
          <small>{$i18n("settings.personaStyleHint")}</small>
        </label>

        <label class="field bluff-field">
          <span>
            {$i18n("settings.bluffIntensity")}
            <b>{draft.bluffIntensity} / 100</b>
          </span>
          <input
            type="range"
            min="0"
            max="100"
            step="5"
            value={draft.bluffIntensity}
            aria-valuetext={`${draft.bluffIntensity} / 100`}
            on:input={(event) =>
              onChange({
                bluffIntensity: Number(event.currentTarget.value),
              })}
          />
          <small>
            {$i18n(bluffKey(draft.bluffIntensity))} ·
            {$i18n("settings.bluffIntensityHint")}
          </small>
        </label>

        <div class="field">
          <span>{$i18n("settings.afterMove")}</span>
          <div class="segmented">
            <button
              type="button"
              class:active={draft.responseMode === "auto"}
              on:click={() => onChange({ responseMode: "auto" })}
            >
              <strong>{$i18n("settings.auto")}</strong>
              <small>{$i18n("settings.autoHint")}</small>
            </button>
            <button
              type="button"
              class:active={draft.responseMode === "button"}
              on:click={() => onChange({ responseMode: "button" })}
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
              on:click={() => onHumanColor("white")}
            >
              {$i18n("settings.white")}
            </button>
            <button
              type="button"
              class:active={humanColor === "black"}
              on:click={() => onHumanColor("black")}
            >
              {$i18n("settings.black")}
            </button>
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
              value={draft.stockfishDepth}
              on:input={(event) =>
                onChange({
                  stockfishDepth: Number(event.currentTarget.value),
                  difficulty: "custom",
                })}
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
              value={draft.multipv}
              on:input={(event) =>
                onChange({
                  multipv: Number(event.currentTarget.value),
                  difficulty: "custom",
                })}
            />
          </label>
        </div>
        <p class="hint">{$i18n("settings.engineNote")}</p>
      </section>

      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.clock")}</h3>
          <span>{$i18n("settings.clockHint")}</span>
        </div>

        <label class="check clock-toggle">
          <input type="checkbox" checked={draft.clockEnabled}
          on:change={(event) =>
            onChange({ clockEnabled: event.currentTarget.checked })} />
          <span>{$i18n("settings.clockEnable")}</span>
        </label>

        {#if draft.clockEnabled}
          <div class="clock-presets">
            {#each TIME_CONTROLS as control}
              <button
                type="button"
                class="clock-card"
                class:active={draft.timeControl === control.id}
                on:click={() => selectTimeControl(control.id)}
              >
                <strong>{$i18n(`clock.${control.key}.name`)}</strong>
                <small>
                  {control.id === "custom"
                    ? clockLabel(
                        draft.clockInitialMs,
                        draft.clockIncrementMs,
                      )
                    : clockLabel(control.initialMs, control.incrementMs)}
                </small>
              </button>
            {/each}
          </div>

          {#if draft.timeControl === "custom"}
            <div class="grid two clock-custom">
              <label class="field">
                <span>{$i18n("settings.clockMinutes")}</span>
                <input
                  type="number"
                  min="0.5"
                  max="180"
                  step="0.5"
                  value={draft.clockInitialMs / 60_000}
                  on:change={setClockMinutes}
                />
              </label>
              <label class="field">
                <span>{$i18n("settings.clockIncrement")}</span>
                <input
                  type="number"
                  min="0"
                  max="60"
                  step="1"
                  value={draft.clockIncrementMs / 1000}
                  on:change={setClockIncrement}
                />
              </label>
            </div>
          {/if}

          <small class="clock-note">{$i18n("settings.clockNote")}</small>
        {/if}
      </section>

      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.features")}</h3>
            <span>{$i18n("settings.featuresHint")}</span>
          </div>
          <div class="checks feature-checks">
            <label class="check">
              <input type="checkbox" checked={draft.chatEnabled}
              on:change={(event) =>
                onChange({ chatEnabled: event.currentTarget.checked })} />
              <span>{$i18n("settings.opponentChat")}</span>
            </label>
            <label class="check" title={$i18n("settings.coachHint")}>
              <input type="checkbox" checked={draft.coachEnabled}
              on:change={(event) =>
                onChange({ coachEnabled: event.currentTarget.checked })} />
              <span>{$i18n("settings.coach")}</span>
            </label>
            <label class="check">
              <input type="checkbox" checked={draft.drawOffersEnabled}
              on:change={(event) =>
                onChange({ drawOffersEnabled: event.currentTarget.checked })} />
              <span>{$i18n("settings.drawOffers")}</span>
            </label>
          </div>
        </section>

<style>
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

  .opponent-model {
    display: flex;
    width: 100%;
    align-items: center;
    gap: 11px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
    color: var(--text);
    text-align: left;
    cursor: pointer;
    transition:
      border-color 140ms ease,
      background 140ms ease;
  }

  .opponent-model:hover {
    border-color: var(--accent);
    background: var(--accent-soft);
  }

  .opponent-model-copy {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 1px;
  }

  .opponent-model-label {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .opponent-model-copy strong {
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(12px * var(--font-scale));
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .opponent-model-copy small {
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .opponent-model-action {
    flex: 0 0 auto;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    white-space: nowrap;
  }

  .feature-checks {
    margin-top: 2px;
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


  .persona-field {
    margin-top: 14px;
  }

  .persona-style-field {
    margin-top: 12px;
  }

  .bluff-field {
    margin-top: 12px;
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

  .clock-toggle {
    margin-bottom: 10px;
  }

  .clock-presets {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(105px, 1fr));
    gap: 8px;
  }

  .clock-card {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 8px 10px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--muted);
    text-align: left;
    cursor: pointer;
  }

  .clock-card:hover {
    border-color: var(--accent);
  }

  .clock-card.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .clock-card strong {
    color: var(--text);
    font-size: calc(11px * var(--font-scale));
  }

  .clock-card small {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .clock-custom {
    margin-top: 9px;
  }

  .clock-note {
    display: block;
    margin-top: 8px;
    line-height: 1.5;
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


  .hint {
    margin: 12px 0 0;
    color: var(--faint);
    font-size: calc(10.5px * var(--font-scale));
    line-height: 1.55;
  }

</style>
