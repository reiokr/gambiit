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

  interface CredentialStatus {
    provider: string;
    envVar: string | null;
    hasKey: boolean;
  }

  const PROVIDER_ENV: Record<string, string> = {
    openai: "OPENAI_API_KEY",
    anthropic: "ANTHROPIC_API_KEY",
    openrouter: "OPENROUTER_API_KEY",
    opencode: "OPENCODE_API_KEY",
    "opencode-go": "OPENCODE_GO_API_KEY",
    google: "GOOGLE_GENERATIVE_AI_API_KEY",
    gemini: "GEMINI_API_KEY",
    groq: "GROQ_API_KEY",
    mistral: "MISTRAL_API_KEY",
    xai: "XAI_API_KEY",
    deepseek: "DEEPSEEK_API_KEY",
    localai: "LOCALAI_API_KEY",
  };

  let draft: Settings = { ...settings, fontScale: settings.fontScale ?? 1.2 };
  let humanColor: ColorKey = initialColor;
  let modelOpen = false;
  let modelFilter = "";
  let modelInput: HTMLInputElement | null = null;
  let provider = "";
  let envVar = "";
  let credential: CredentialStatus | null = null;
  let keyInput = "";
  let keyMessage = "";
  let keyBusy = false;
  let providerSync = 0;
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
    draft.drawOffersEnabled !== settings.drawOffersEnabled ||
    Math.abs(draft.fontScale - settings.fontScale) > 0.001;
  $: clockChanged =
    draft.clockEnabled !== settings.clockEnabled ||
    (draft.clockEnabled &&
      (draft.timeControl !== settings.timeControl ||
        draft.clockInitialMs !== settings.clockInitialMs ||
        draft.clockIncrementMs !== settings.clockIncrementMs));
  $: hasChanges = opponentChanged || colorChanged || appearanceChanged || clockChanged;
  $: if (hasChanges) savedMessage = "";
  $: provider = draft.model.includes("/")
    ? draft.model.split("/")[0].trim()
    : "";
  $: filteredModels = models
    .filter((model) =>
      model.toLowerCase().includes(modelFilter.toLowerCase()),
    )
    .slice(0, 200);
  $: groupedModels = (() => {
    const groups = new Map<string, string[]>();
    for (const model of filteredModels) {
      const group = model.includes("/") ? model.split("/")[0] : "custom";
      const list = groups.get(group) ?? [];
      list.push(model);
      groups.set(group, list);
    }
    return [...groups.entries()].map(([name, entries]) => ({
      provider: name,
      models: entries,
    }));
  })();

  onMount(() => {
    void syncProvider();
    void refreshPieceSets();
  });

  async function syncProvider() {
    const token = ++providerSync;
    const current = provider;
    envVar = PROVIDER_ENV[current] ?? "";
    credential = null;
    keyMessage = "";
    if (!current) return;
    try {
      const status = await invoke<CredentialStatus>("credential_status", {
        provider: current,
      });
      if (token !== providerSync) return;
      credential = status;
      if (status.envVar) {
        envVar = status.envVar;
      }
    } catch {
      if (token === providerSync) {
        credential = null;
      }
    }
  }

  function chooseModel(model: string) {
    draft.model = model;
    modelFilter = "";
    modelOpen = false;
    void syncProvider();
  }

  function closeModelMenuSoon() {
    window.setTimeout(() => {
      modelOpen = false;
    }, 120);
  }

  function handleModelBlur() {
    closeModelMenuSoon();
    void syncProvider();
  }

  async function saveKey() {
    if (!provider || !envVar.trim() || !keyInput.trim()) return;
    keyBusy = true;
    keyMessage = "";
    try {
      credential = await invoke<CredentialStatus>("save_credential", {
        provider,
        envVar,
        key: keyInput,
      });
      keyInput = "";
      keyMessage = $i18n("settings.keySavedMessage");
    } catch (value) {
      keyMessage = value instanceof Error ? value.message : String(value);
    } finally {
      keyBusy = false;
    }
  }

  async function removeKey() {
    if (!provider) return;
    keyBusy = true;
    keyMessage = "";
    try {
      credential = await invoke<CredentialStatus>("delete_credential", {
        provider,
      });
      keyMessage = $i18n("settings.keyRemovedMessage");
    } catch (value) {
      keyMessage = value instanceof Error ? value.message : String(value);
    } finally {
      keyBusy = false;
    }
  }

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
    if (tab === "connection") {
      void syncProvider();
    }
  }

  function modelName(model: string): string {
    return model.includes("/") ? model.split("/").pop() || model : model || "—";
  }

  function providerName(model: string): string {
    return model.includes("/") ? model.split("/")[0] : "";
  }

  async function openModelPicker() {
    selectTab("connection");
    modelFilter = "";
    await tick();
    modelOpen = true;
    modelInput?.focus();
    modelInput?.select();
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
        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.opponent")}</h3>
            <span>{$i18n("settings.opponentHint")}</span>
          </div>
        <button class="opponent-model" type="button" on:click={openModelPicker}>
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
            <button
              type="button"
              class="persona-card"
              class:active={draft.persona === "Custom"}
              style={`--persona-accent:${draft.customPersona.accent}`}
              on:click={() => (draft.persona = "Custom")}
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
              onChange={(next) => (draft.customPersona = next)}
            />
          {/if}
          <small class="persona-hint">{$i18n("settings.personaHint")}</small>
        </div>

        <label class="field persona-style-field">
          <span>{$i18n("settings.personaStyle")}</span>
          <select
            value={draft.personaStyle}
            on:change={(event) =>
              (draft.personaStyle = event.currentTarget
                .value as typeof draft.personaStyle)}
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

      <section class="section">
        <div class="section-title">
          <h3>{$i18n("settings.clock")}</h3>
          <span>{$i18n("settings.clockHint")}</span>
        </div>

        <label class="check clock-toggle">
          <input type="checkbox" bind:checked={draft.clockEnabled} />
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
              <input type="checkbox" bind:checked={draft.chatEnabled} />
              <span>{$i18n("settings.opponentChat")}</span>
            </label>
            <label class="check">
              <input type="checkbox" bind:checked={draft.drawOffersEnabled} />
              <span>{$i18n("settings.drawOffers")}</span>
            </label>
          </div>
        </section>
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
        <section class="section">
          <div class="section-title">
            <h3>{$i18n("settings.connections")}</h3>
            <span>{$i18n("settings.connectionsHint")}</span>
          </div>
          <div class="field agent-mode-field">
            <span>{$i18n("settings.agentMode")}</span>
            <div class="segmented">
              <button
                type="button"
                class:active={draft.agentMode === "prompt"}
                on:click={() => (draft.agentMode = "prompt")}
              >
                <strong>{$i18n("settings.agentModePrompt")}</strong>
                <small>{$i18n("settings.agentModePromptHint")}</small>
              </button>
              <button
                type="button"
                class:active={draft.agentMode === "mcp"}
                on:click={() => (draft.agentMode = "mcp")}
              >
                <strong>{$i18n("settings.agentModeMcp")}</strong>
                <small>{$i18n("settings.agentModeMcpHint")}</small>
              </button>
            </div>
            <small>{$i18n("settings.agentModeHint")}</small>
          </div>
<div class="field model-field">
          <span>{$i18n("settings.model")}</span>
          <div class="model-row">
            <ModelBadge model={draft.model} size="md" />
            <div class="model-picker">
            <input
              bind:this={modelInput}
              bind:value={draft.model}
              spellcheck="false"
              autocomplete="off"
              placeholder="provider/model"
              on:focus={(event) => {
                modelOpen = true;
                modelFilter = "";
                event.currentTarget.select();
              }}
              on:input={(event) => {
                modelOpen = true;
                modelFilter = event.currentTarget.value;
              }}
              on:keydown={(event) => {
                if (event.key === "Escape") modelOpen = false;
              }}
              on:blur={handleModelBlur}
            />
            <span class="model-caret" aria-hidden="true">▾</span>
            {#if modelOpen}
              <div class="model-menu">
                <div class="model-menu-head">
                  <span>
                    {$i18n("settings.modelsShown", {
                      shown: filteredModels.length,
                      total: models.length,
                    })}
                  </span>
                  <button
                    type="button"
                    on:click={() => void onRefreshModels()}
                  >
                    {$i18n("settings.refreshModels")}
                  </button>
                </div>
                <div class="model-options">
                  {#each groupedModels as group}
                    <div class="model-provider">{group.provider}</div>
                    {#each group.models as model}
                      <button
                        type="button"
                        class="model-option"
                        class:active={draft.model === model}
                        on:mousedown|preventDefault
                        on:click={() => chooseModel(model)}
                      >
                        {model}
                      </button>
                    {/each}
                  {/each}
                </div>
              </div>
            {/if}
            </div>
          </div>
          <small>
            {$i18n("settings.modelsDiscovered", { count: models.length })}
          </small>
        </div>

        <div class="credentials">
          {#if provider}
            <div class="credentials-head">
              <span>
                {$i18n("settings.credentials")}
                <b>{provider}</b>
              </span>
              {#if credential?.hasKey}
                <span class="key-saved">{$i18n("settings.keySaved")}</span>
              {/if}
            </div>

            {#if provider === "ollama"}
              <p class="hint">{$i18n("settings.localProvider")}</p>
            {:else}
              <div class="grid two">
                <label class="field">
                  <span>{$i18n("settings.envVar")}</span>
                  <input
                    bind:value={envVar}
                    spellcheck="false"
                    autocomplete="off"
                  />
                </label>
                <label class="field">
                  <span>{$i18n("settings.apiKey")}</span>
                  <input
                    type="password"
                    bind:value={keyInput}
                    autocomplete="off"
                    spellcheck="false"
                    placeholder={credential?.hasKey
                      ? $i18n("settings.keySavedPlaceholder")
                      : $i18n("settings.keyPlaceholder")}
                  />
                </label>
              </div>
              <div class="credential-actions">
                <button
                  class="btn"
                  type="button"
                  disabled={keyBusy || !keyInput.trim() || !envVar.trim()}
                  on:click={saveKey}
                >
                  {$i18n("settings.saveKey")}
                </button>
                {#if credential?.hasKey}
                  <button
                    class="btn ghost"
                    type="button"
                    disabled={keyBusy}
                    on:click={removeKey}
                  >
                    {$i18n("settings.removeKey")}
                  </button>
                {/if}
                {#if keyMessage}
                  <span class="key-message">{keyMessage}</span>
                {/if}
              </div>
              <p class="hint">{$i18n("settings.credentialsHint")}</p>
              {#if provider === "opencode-go" || provider === "github-copilot"}
                <p class="hint">{$i18n("settings.opencodeAuthHint")}</p>
              {/if}
            {/if}
          {/if}
        </div>

                  {#if !provider}
            <p class="hint">{$i18n("settings.chooseModelFirst")}</p>
          {/if}
        </section>
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

  .agent-mode-field {
    margin-bottom: 16px;
  }

  .model-row {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .model-row .model-picker {
    min-width: 0;
    flex: 1;
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

  .model-field {
    position: relative;
  }

  .model-picker {
    position: relative;
  }

  .model-picker input {
    padding-right: 30px;
  }

  .model-caret {
    position: absolute;
    top: 50%;
    right: 11px;
    color: var(--faint);
    font-size: calc(11px * var(--font-scale));
    pointer-events: none;
    transform: translateY(-50%);
  }

  .model-menu {
    position: absolute;
    z-index: 40;
    top: calc(100% + 6px);
    right: 0;
    left: 0;
    display: flex;
    height: min(340px, 46vh);
    max-height: 340px;
    flex-direction: column;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel);
    box-shadow: 0 18px 42px rgba(0, 0, 0, 0.38);
  }

  .model-menu-head {
    position: sticky;
    top: 0;
    z-index: 1;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
    color: var(--faint);
    font-size: calc(9px * var(--font-scale));
  }

  .model-menu-head button {
    padding: 2px 6px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .model-options {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    overflow-y: auto;
    padding: 4px;
  }

  .model-provider {
    padding: 7px 8px 3px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.04em;
  }

  .model-option {
    padding: 5px 8px;
    border: 0;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    text-align: left;
    cursor: pointer;
  }

  .model-option:hover,
  .model-option.active {
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .credentials {
    margin-top: 14px;
    padding: 11px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .credentials-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    color: var(--muted);
    font-size: calc(11px * var(--font-scale));
  }

  .credentials-head b {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
  }

  .key-saved {
    padding: 2px 6px;
    border: 1px solid color-mix(in srgb, var(--success) 45%, var(--border));
    border-radius: 999px;
    color: var(--success);
    font-size: calc(9px * var(--font-scale));
    white-space: nowrap;
  }

  .credential-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 9px;
    flex-wrap: wrap;
  }

  .credential-actions .btn {
    padding: 6px 10px;
    font-size: calc(10px * var(--font-scale));
  }

  .key-message {
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
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

  .persona-field {
    margin-top: 14px;
  }

  .persona-style-field {
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
    .themes,
    .persona-grid,
    .difficulty-grid {
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
