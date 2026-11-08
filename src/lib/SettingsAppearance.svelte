<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import type { PieceSetSummary, Settings } from "./types";
  import { DEFAULT_CUSTOM_THEME, THEMES, applyTheme } from "./themes";
  import ThemeEditor from "./ThemeEditor.svelte";
  import { piecePreview } from "./pieces";
  import { i18n } from "./i18n";

  export let draft: Settings;
  export let onChange: (patch: Partial<Settings>) => void = () => {};

  let pieceSets: PieceSetSummary[] = [];
  let pieceSetError = "";
  let pieceSetMessage = "";

  $: pieceSetMissing =
    pieceSets.length > 0 &&
    !pieceSets.some((set) => set.id === draft.pieceSet);

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

  function selectTheme(theme: string) {
    onChange({ theme });
    applyTheme(theme, draft.customTheme);
  }

  function updateCustomTheme(theme: Settings["customTheme"]) {
    onChange({ customTheme: theme });
    applyTheme("custom", theme);
  }

  function resetCustomTheme() {
    const theme = { ...DEFAULT_CUSTOM_THEME };
    onChange({ customTheme: theme });
    applyTheme("custom", theme);
  }

  function selectPieceSet(id: string) {
    onChange({ pieceSet: id });
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
</script>

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
                value={draft.fontScale}
                on:input={(event) =>
                  onChange({ fontScale: Number(event.currentTarget.value) })}
              />
            </label>
            <div class="field">
              <span>{$i18n("settings.soundMotion")}</span>
              <div class="checks">
                <label class="check">
                  <input type="checkbox" checked={draft.sound}
                  on:change={(event) =>
                    onChange({ sound: event.currentTarget.checked })} />
                  <span>{$i18n("settings.sound")}</span>
                </label>
                <label class="check">
                  <input type="checkbox" checked={draft.animations}
                  on:change={(event) =>
                    onChange({ animations: event.currentTarget.checked })} />
                  <span>{$i18n("settings.animations")}</span>
                </label>
              </div>
            </div>
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

  input:not([type="checkbox"]):not([type="range"]) {
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

  @media (max-width: 700px) {
    .grid.two {
      grid-template-columns: 1fr;
    }
  }
</style>
