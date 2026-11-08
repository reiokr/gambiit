<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { ColorKey, PieceSetSummary, Settings } from "./types";
  import { DEFAULT_CUSTOM_THEME, THEMES, applyTheme } from "./themes";
  import ThemeEditor from "./ThemeEditor.svelte";
  import { applyPieceSet, piecePreview } from "./pieces";
  import { applyFontScale, applyMotion } from "./settings";
  import {
    availableLocales,
    i18n,
    localeMeta,
    setLocale,
  } from "./i18n";
  import { CUSTOM_PERSONA, PERSONAS } from "./personas";
  import PersonaMixer from "./PersonaMixer.svelte";
  import SettingsConnection from "./SettingsConnection.svelte";
  import SettingsGame from "./SettingsGame.svelte";
  import { TIME_CONTROLS, clockLabel } from "./clock";
  import ModelBadge from "./ModelBadge.svelte";
  import { focusTrap } from "./a11y";
  import { DIFFICULTIES, type DifficultyOption } from "./difficulty";

  export let settings: Settings;
  export let models: string[] = [];
  export let initialColor: ColorKey = "white";
  export let onApply: (
    settings: Settings,
    humanColor: ColorKey,
    startNew: boolean,
  ) => Promise<string | null> = async () => null;
  export let onClose: () => void = () => {};
  export let onRefreshModels: () => Promise<void> | void = () => {};

  let draft: Settings = { ...settings, fontScale: settings.fontScale ?? 1.2 };
  let humanColor: ColorKey = initialColor;
  let connectionTab: SettingsConnection | null = null;
  let pieceSets: PieceSetSummary[] = [];
  let pieceSetError = "";
  let pieceSetMessage = "";
  let applyError = "";
  let applying = false;
  let savedMessage = "";
  type SettingsTab = "game" | "appearance" | "language" | "connection";
  let activeTab: SettingsTab = "game";
  let bodyEl: HTMLDivElement | null = null;

  $: applyFontScale(draft.fontScale);
  $: applyMotion(draft.animations);
  $: if (draft.theme === "custom") {
    applyTheme("custom", draft.customTheme);
  }
  $: if (draft.pieceSet) {
    void applyPieceSet(draft.pieceSet);
  }
  $: pieceSetMissing =
    pieceSets.length > 0 &&
    !pieceSets.some((set) => set.id === draft.pieceSet);
  $: opponentChanged =
    draft.model !== settings.model ||
    draft.persona !== settings.persona ||
    draft.agentMode !== settings.agentMode ||
    draft.personaStyle !== settings.personaStyle ||
    (draft.persona === "Custom" &&
      JSON.stringify(draft.customPersona) !== JSON.stringify(settings.customPersona)) ||
    draft.difficulty !== settings.difficulty ||
    draft.stockfishDepth !== settings.stockfishDepth ||
    draft.multipv !== settings.multipv ||
    draft.agentLanguage !== settings.agentLanguage;
  $: colorChanged = humanColor !== initialColor;
  $: appearanceChanged =
    draft.theme !== settings.theme ||
    !sameTheme(draft.customTheme, settings.customTheme) ||
    draft.pieceSet !== settings.pieceSet ||
    draft.uiLanguage !== settings.uiLanguage ||
    draft.sound !== settings.sound ||
    draft.animations !== settings.animations ||
    draft.responseMode !== settings.responseMode ||
    draft.chatEnabled !== settings.chatEnabled ||
    draft.coachEnabled !== settings.coachEnabled ||
    draft.drawOffersEnabled !== settings.drawOffersEnabled ||
    draft.bluffIntensity !== settings.bluffIntensity ||
    Math.abs(draft.fontScale - settings.fontScale) > 0.001;
  $: clockChanged =
    draft.clockEnabled !== settings.clockEnabled ||
    (draft.clockEnabled &&
      (draft.timeControl !== settings.timeControl ||
        draft.clockInitialMs !== settings.clockInitialMs ||
        draft.clockIncrementMs !== settings.clockIncrementMs));
  $: hasChanges = opponentChanged || colorChanged || appearanceChanged || clockChanged;
  $: if (hasChanges) savedMessage = "";
  onMount(() => {
    void refreshPieceSets();
  });

  const previews: Record<string, string[]> = {
    salon: ["#e8d3b0", "#8b5a2b", "#c9a227"],
    tournament: ["#eeeed2", "#769656", "#b9d48b"],
    midnight: ["#9fb4c9", "#3b4f63", "#58a6ff"],
  };

  function themePreview(id: string): string[] {
    if (id === "custom") {
      const theme = draft.customTheme;
      return [theme.boardLight, theme.boardDark, theme.accent];
    }
    return previews[id] ?? [];
  }

  function sameTheme(left: typeof draft.customTheme, right: typeof draft.customTheme): boolean {
    return (
      left.bg === right.bg &&
      left.panel === right.panel &&
      left.border === right.border &&
      left.text === right.text &&
      left.muted === right.muted &&
      left.accent === right.accent &&
      left.boardLight === right.boardLight &&
      left.boardDark === right.boardDark &&
      left.boardBorder === right.boardBorder
    );
  }

  function selectTheme(theme: string) {
    draft.theme = theme;
    applyTheme(theme, draft.customTheme);
  }

  function updateCustomTheme(theme: typeof draft.customTheme) {
    draft.customTheme = theme;
    applyTheme("custom", theme);
  }

  function resetCustomTheme() {
    const theme = { ...DEFAULT_CUSTOM_THEME };
    draft.customTheme = theme;
    applyTheme("custom", theme);
  }

  function bluffKey(value: number): string {
    if (value <= 15) return "settings.bluffHonest";
    if (value <= 39) return "settings.bluffMild";
    if (value <= 64) return "settings.bluffTheatrical";
    if (value <= 84) return "settings.bluffBold";
    return "settings.bluffUnreliable";
  }

  function selectPieceSet(id: string) {
    draft.pieceSet = id;
    pieceSetMessage = "";
  }

  async function refreshPieceSets() {
    try {
      pieceSets = await invoke<PieceSetSummary[]>("list_piece_sets");
      pieceSetError = "";
    } catch (value) {
      pieceSetError = value instanceof Error ? value.message : String(value);
    }
  }

  async function openPiecesFolder() {
    pieceSetMessage = "";
    try {
      await invoke("open_pieces_folder");
      pieceSetMessage = $i18n("settings.piecesFolderOpened");
    } catch (value) {
      pieceSetError = value instanceof Error ? value.message : String(value);
    }
  }

  async function reloadPieceSets() {
    await refreshPieceSets();
    pieceSetMessage = $i18n("settings.piecesRefreshed");
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

  function selectTimeControl(id: string) {
    draft.timeControl = id;
    const control = TIME_CONTROLS.find((option) => option.id === id);
    if (control && id !== "custom") {
      draft.clockInitialMs = control.initialMs;
      draft.clockIncrementMs = control.incrementMs;
    }
  }

  function setClockMinutes(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const minutes = Math.min(180, Math.max(0.5, Number(input.value) || 10));
    draft.clockInitialMs = Math.round(minutes * 60_000);
    draft.timeControl = "custom";
  }

  function setClockIncrement(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const seconds = Math.min(60, Math.max(0, Number(input.value) || 0));
    draft.clockIncrementMs = Math.round(seconds * 1000);
    draft.timeControl = "custom";
  }

  function selectTab(tab: SettingsTab) {
    activeTab = tab;
    bodyEl?.scrollTo({ top: 0 });
  }

  function modelName(model: string): string {
    return model.includes("/") ? model.split("/").pop() || model : model || "—";
  }

  function providerName(model: string): string {
    return model.includes("/") ? model.split("/")[0] : "";
  }

  async function openModelPicker() {
    selectTab("connection");
    await tick();
    await connectionTab?.focusModel();
  }

  async function submit(startNew: boolean) {
    if (!hasChanges || applying) return;
    applying = true;
    applyError = "";
    savedMessage = "";
    const error = await onApply(
      { ...draft, customTheme: { ...draft.customTheme } },
      humanColor,
      startNew,
    );
    if (error) {
      applyError = error;
    } else {
      savedMessage = $i18n("settings.saved");
    }
    applying = false;
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
    use:focusTrap={onClose}
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

    <div class="settings-tabs" role="tablist" aria-label={$i18n("settings.tabs")}>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "game"}
        class:active={activeTab === "game"}
        data-autofocus
        on:click={() => selectTab("game")}
      >
        {$i18n("settings.tabGame")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "appearance"}
        class:active={activeTab === "appearance"}
        on:click={() => selectTab("appearance")}
      >
        {$i18n("settings.tabAppearance")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "language"}
        class:active={activeTab === "language"}
        on:click={() => selectTab("language")}
      >
        {$i18n("settings.tabLanguage")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "connection"}
        class:active={activeTab === "connection"}
        on:click={() => selectTab("connection")}
      >
        {$i18n("settings.tabConnection")}
      </button>
    </div>

    <div class="body" bind:this={bodyEl}>
      {#if activeTab === "game"}
        <SettingsGame
          {draft}
          {humanColor}
          onChange={(patch) => (draft = { ...draft, ...patch })}
          onHumanColor={(color) => (humanColor = color)}
          onOpenModelPicker={openModelPicker}
        />
      {:else if activeTab === "appearance"}
        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.boardPieces")}</h3>
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
                {#each themePreview(theme.id) as color}
                  <i style={`background:${color}`}></i>
                {/each}
              </span>
              <strong>{$i18n(`themes.${theme.id}.name`)}</strong>
              <small>{$i18n(`themes.${theme.id}.description`)}</small>
            </button>
          {/each}
        </div>

        {#if draft.theme === "custom"}
          <ThemeEditor
            theme={draft.customTheme}
            onChange={updateCustomTheme}
            onReset={resetCustomTheme}
          />
        {/if}

        <div class="field piece-field">
          <span>{$i18n("settings.pieceSet")}</span>
          <div class="piece-sets">
            {#each pieceSets as set}
              <button
                type="button"
                class="piece-card"
                class:active={draft.pieceSet === set.id}
                aria-pressed={draft.pieceSet === set.id}
                on:click={() => selectPieceSet(set.id)}
              >
                <span class="piece-image">
                  <img
                    src={set.preview ?? piecePreview(set.id)}
                    alt=""
                    draggable="false"
                  />
                </span>
                <span class="piece-copy">
                  <strong>{set.name}</strong>
                  <small>
                    {set.builtIn
                      ? $i18n("settings.piecesBuiltIn")
                      : $i18n("settings.piecesCount", { count: set.pieces })}
                  </small>
                </span>
                {#if set.builtIn}
                  <span class="piece-badge">{$i18n("settings.piecesDefault")}</span>
                {/if}
              </button>
            {/each}
          </div>
          <div class="piece-actions">
            <button class="mini-btn" type="button" on:click={reloadPieceSets}>
              {$i18n("settings.piecesRefresh")}
            </button>
            <button class="mini-btn" type="button" on:click={openPiecesFolder}>
              {$i18n("settings.piecesFolder")}
            </button>
            {#if pieceSetMessage}
              <span class="piece-note">{pieceSetMessage}</span>
            {/if}
          </div>
          {#if pieceSetError}
            <small class="piece-error">{pieceSetError}</small>
          {:else if pieceSetMissing}
            <small class="piece-error">{$i18n("settings.piecesMissing")}</small>
          {:else}
            <small>{$i18n("settings.piecesHint")}</small>
          {/if}
        </div>

                </section>

        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.interface")}</h3>
            <span>{$i18n("settings.interfaceHint")}</span>
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
              <span>{$i18n("settings.soundMotion")}</span>
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
      {:else if activeTab === "language"}
        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.language")}</h3>
            <span>{$i18n("settings.languageHint")}</span>
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

                </section>
      {:else}
        <SettingsConnection
          bind:this={connectionTab}
          {draft}
          {models}
          {onRefreshModels}
          onChange={(patch) => (draft = { ...draft, ...patch })}
        />
      {/if}
    </div>

    <footer class="foot">
      {#if colorChanged}
        <p class="foot-warning">{$i18n("settings.colorWarning")}</p>
      {:else if clockChanged}
        <p class="foot-warning">{$i18n("settings.clockWarning")}</p>
      {:else if opponentChanged}
        <p class="foot-warning">{$i18n("settings.gameWarning")}</p>
      {/if}
      {#if applyError}
        <p class="foot-error">{applyError}</p>
      {/if}
      <div class="foot-actions">
        {#if savedMessage}
          <span class="foot-saved">{$i18n("settings.saved")}</span>
        {/if}
        <button class="btn ghost" type="button" on:click={onClose}>
          {$i18n("settings.cancel")}
        </button>
        {#if opponentChanged && !colorChanged && !clockChanged}
          <button
            class="btn"
            type="button"
            disabled={applying}
            on:click={() => submit(true)}
          >
            {$i18n("settings.saveAndNew")}
          </button>
        {/if}
        {#if colorChanged || clockChanged}
          <button
            class="btn primary"
            type="button"
            disabled={applying}
            on:click={() => submit(true)}
          >
            {applying ? $i18n("settings.saving") : $i18n("settings.saveAndNew")}
          </button>
        {:else}
          <button
            class="btn primary"
            type="button"
            disabled={!hasChanges || applying}
            on:click={() => submit(false)}
          >
            {applying ? $i18n("settings.saving") : $i18n("settings.saveChanges")}
          </button>
        {/if}
      </div>
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
    display: flex;
    width: min(760px, 100%);
    height: min(860px, calc(100vh - 64px));
    max-height: min(860px, calc(100vh - 64px));
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 18px;
    background: var(--panel);
    box-shadow: var(--shadow);
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

  .settings-tabs {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 10px 20px;
    border-bottom: 1px solid var(--border);
    background: var(--panel-2);
  }

  .settings-tabs button {
    display: inline-flex;
    min-height: 32px;
    flex: 0 0 auto;
    align-items: center;
    justify-content: center;
    padding: 0 13px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    color: var(--muted);
    font-family: var(--font-ui);
    font-size: calc(11px * var(--font-scale));
    font-weight: 600;
    line-height: 1;
    white-space: nowrap;
    cursor: pointer;
    transition:
      border-color 140ms ease,
      background 140ms ease,
      color 140ms ease;
  }

  .settings-tabs button:hover {
    border-color: var(--border);
    color: var(--text);
  }

  .settings-tabs button.active {
    border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
    background: var(--accent-soft);
    box-shadow: inset 0 0 0 1px var(--accent-soft);
    color: var(--text);
  }

  .body {
    flex: 1;
    min-height: 0;
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

  .themes {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
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

  .piece-field {
    margin: 4px 0 14px;
  }

  .piece-sets {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 7px;
  }

  .piece-card {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel-2);
    color: var(--text);
    text-align: left;
    cursor: pointer;
  }

  .piece-card:hover {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .piece-card.active {
    border-color: var(--accent);
    background: var(--accent-soft);
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .piece-image {
    display: grid;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    place-items: center;
    border-radius: 7px;
    background:
      linear-gradient(135deg, var(--accent-soft), transparent),
      var(--panel);
  }

  .piece-image img {
    width: 28px;
    height: 28px;
    object-fit: contain;
    filter: drop-shadow(0 2px 2px rgba(0, 0, 0, 0.3));
  }

  .piece-copy {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 1px;
  }

  .piece-copy strong {
    overflow: hidden;
    font-size: calc(11px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .piece-copy small {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .piece-badge {
    flex: 0 0 auto;
    padding: 1px 4px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .piece-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 7px;
  }

  .mini-btn {
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .mini-btn:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .piece-note {
    color: var(--success);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .piece-error {
    display: block;
    margin-top: 6px;
    color: var(--danger);
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

  .foot {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 14px 26px 16px;
    border-top: 1px solid var(--border);
    background: var(--panel-2);
  }

  .foot-actions {
    display: flex;
    justify-content: flex-end;
    flex-wrap: wrap;
    gap: 10px;
  }

  .foot-warning {
    padding: 7px 9px;
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--accent) 38%, var(--border));
    border-radius: 7px;
    background: var(--accent-soft);
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .foot-error {
    padding: 7px 9px;
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--border));
    border-radius: 7px;
    background: color-mix(in srgb, var(--panel) 92%, var(--danger));
    color: var(--danger);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .foot-saved {
    margin-right: auto;
    color: var(--success);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    letter-spacing: 0.04em;
  }

  .btn:disabled {
    opacity: 0.42;
    cursor: not-allowed;
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
      height: calc(100vh - 24px);
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
