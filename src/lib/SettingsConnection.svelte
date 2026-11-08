<script lang="ts">
  import { tick } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { Settings } from "./types";
  import ModelBadge from "./ModelBadge.svelte";
  import SetupDiagnostics from "./SetupDiagnostics.svelte";
  import { i18n } from "./i18n";

  export let draft: Settings;
  export let models: string[] = [];
  export let onRefreshModels: () => Promise<void> | void = () => {};
  export let onChange: (patch: Partial<Settings>) => void = () => {};

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

  let modelOpen = false;
  let modelFilter = "";
  let modelInput: HTMLInputElement | null = null;
  let provider = "";
  let syncedProvider = "";
  let envVar = "";
  let credential: CredentialStatus | null = null;
  let keyInput = "";
  let keyMessage = "";
  let keyBusy = false;
  let providerSync = 0;

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
  $: if (provider !== syncedProvider) {
    syncedProvider = provider;
    void syncProvider(provider);
  }

  export async function focusModel() {
    modelFilter = "";
    await tick();
    modelOpen = true;
    modelInput?.focus();
    modelInput?.select();
  }

  async function syncProvider(current: string) {
    const token = ++providerSync;
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
    onChange({ model });
    modelFilter = "";
    modelOpen = false;
  }

  function handleModelBlur() {
    window.setTimeout(() => {
      modelOpen = false;
    }, 120);
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
</script>

<section class="section">
  <SetupDiagnostics />
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
        on:click={() => onChange({ agentMode: "prompt" })}
      >
        <strong>{$i18n("settings.agentModePrompt")}</strong>
        <small>{$i18n("settings.agentModePromptHint")}</small>
      </button>
      <button
        type="button"
        class:active={draft.agentMode === "mcp"}
        on:click={() => onChange({ agentMode: "mcp" })}
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
          value={draft.model}
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
            onChange({ model: event.currentTarget.value });
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
              <button type="button" on:click={() => void onRefreshModels()}>
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
            <input bind:value={envVar} spellcheck="false" autocomplete="off" />
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

  .model-field {
    position: relative;
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
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 9px;
  }

  .credential-actions .btn {
    padding: 6px 10px;
    font-size: calc(10px * var(--font-scale));
  }

  .key-message {
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
  }

  .segmented {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
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

  input {
    width: 100%;
    height: 38px;
    padding: 0 11px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font-size: calc(12px * var(--font-scale));
  }

  small {
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
  }

  .hint {
    margin: 12px 0 0;
    color: var(--faint);
    font-size: calc(10.5px * var(--font-scale));
    line-height: 1.55;
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

  .btn.ghost:hover {
    border-color: var(--accent);
  }

  @media (max-width: 700px) {
    .grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>
