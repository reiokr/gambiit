<script lang="ts">
  import type { GameSummary } from "./types";
  import { i18n } from "./i18n";
  import { focusTrap } from "./a11y";

  export let games: GameSummary[] = [];
  export let currentId = "";
  export let pgn = "";
  export let onOpen: (id: string) => void = () => {};
  export let onNew: () => void = () => {};
  export let onDelete: (id: string) => void = () => {};
  export let onRename: (id: string, name: string) => void = () => {};
  export let onRefresh: () => void = () => {};
  export let onSavePgn: () => Promise<string | void> | string | void = () => {};
  export let onImportPgn: (pgn: string) => Promise<boolean> = async () => false;
  export let onClose: () => void = () => {};

  let activeTab: "saved" | "pgn" = "saved";
  let importText = "";
  let message = "";
  let busy = false;

  function formatDate(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  async function copyPgn() {
    try {
      await navigator.clipboard.writeText(pgn);
      message = $i18n("games.copied");
    } catch {
      message = "Clipboard unavailable.";
    }
  }

  async function savePgn() {
    busy = true;
    message = "";
    try {
      const path = await onSavePgn();
      if (path) message = path;
    } catch (value) {
      message = value instanceof Error ? value.message : String(value);
    } finally {
      busy = false;
    }
  }

  async function importPgn() {
    if (!importText.trim()) return;
    busy = true;
    message = "";
    try {
      const ok = await onImportPgn(importText);
      if (ok) {
        importText = "";
        onClose();
      }
    } catch (value) {
      message = value instanceof Error ? value.message : String(value);
    } finally {
      busy = false;
    }
  }

  function rename(game: GameSummary) {
    const name = window.prompt($i18n("games.renamePrompt"), game.name);
    if (name && name.trim()) {
      onRename(game.id, name.trim());
    }
  }

  function remove(game: GameSummary) {
    if (window.confirm($i18n("games.deleteConfirm"))) {
      onDelete(game.id);
    }
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
    aria-label={$i18n("games.title")}
    tabindex="-1"
    use:focusTrap={onClose}
  >
    <header class="head">
      <div>
        <p class="eyebrow">{$i18n("games.title")}</p>
        <h2>{$i18n("games.title")}</h2>
      </div>
      <button class="close" type="button" aria-label={$i18n("games.close")} on:click={onClose}>
        ✕
      </button>
    </header>

    <div class="tabs" role="tablist">
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "saved"}
        class:active={activeTab === "saved"}
        data-autofocus
        on:click={() => (activeTab = "saved")}
      >
        {$i18n("games.saved")}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "pgn"}
        class:active={activeTab === "pgn"}
        on:click={() => (activeTab = "pgn")}
      >
        {$i18n("games.pgn")}
      </button>
    </div>

    <div class="body">
      {#if activeTab === "saved"}
        <div class="actions">
          <button class="btn primary" type="button" on:click={onNew}>
            {$i18n("games.new")}
          </button>
          <button class="btn ghost" type="button" on:click={onRefresh}>
            {$i18n("games.refresh")}
          </button>
        </div>

        {#if games.length === 0}
          <div class="empty">{$i18n("games.empty")}</div>
        {:else}
          <div class="game-list">
            {#each games as game}
              <article class="game-card" class:current={game.id === currentId}>
                <div class="game-main">
                  <div class="game-title">
                    <strong>{game.name}</strong>
                    {#if game.id === currentId}
                      <span class="badge">{$i18n("games.current")}</span>
                    {/if}
                    <span class="result">{game.result}</span>
                  </div>
                  <div class="game-meta">
                    <span>{game.model}</span>
                    <span>{$i18n("games.moves", { count: game.moveCount })}</span>
                    <span>{formatDate(game.updatedAt)}</span>
                  </div>
                </div>
                <div class="game-buttons">
                  <button class="btn small" type="button" on:click={() => onOpen(game.id)}>
                    {$i18n("games.open")}
                  </button>
                  <button class="btn small ghost" type="button" on:click={() => rename(game)}>
                    {$i18n("games.rename")}
                  </button>
                  <button class="btn small danger" type="button" on:click={() => remove(game)}>
                    {$i18n("games.delete")}
                  </button>
                </div>
              </article>
            {/each}
          </div>
        {/if}
      {:else}
        <section class="pgn-section">
          <div class="section-title">
            <h3>{$i18n("games.exportTitle")}</h3>
            <span>{$i18n("games.exportHint")}</span>
          </div>
          {#if pgn.trim()}
            <textarea readonly value={pgn} spellcheck="false"></textarea>
          {:else}
            <div class="empty">{$i18n("games.pgnEmpty")}</div>
          {/if}
          <div class="pgn-actions">
            <button class="btn" type="button" disabled={!pgn.trim()} on:click={copyPgn}>
              {$i18n("games.copy")}
            </button>
            <button
              class="btn"
              type="button"
              disabled={!pgn.trim() || busy}
              on:click={savePgn}
            >
              {$i18n("games.saveFile")}
            </button>
            {#if message}<span class="message">{message}</span>{/if}
          </div>
        </section>

        <section class="pgn-section">
          <div class="section-title">
            <h3>{$i18n("games.importTitle")}</h3>
            <span>{$i18n("games.importHint")}</span>
          </div>
          <textarea
            bind:value={importText}
            placeholder={$i18n("games.importPlaceholder")}
            spellcheck="false"
          ></textarea>
          <div class="pgn-actions">
            <button
              class="btn primary"
              type="button"
              disabled={!importText.trim() || busy}
              on:click={importPgn}
            >
              {$i18n("games.import")}
            </button>
          </div>
        </section>
      {/if}
    </div>
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
    max-height: min(820px, calc(100vh - 64px));
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

  .tabs {
    display: flex;
    gap: 2px;
    padding: 7px 20px 0;
    border-bottom: 1px solid var(--border);
  }

  .tabs button {
    padding: 9px 12px 10px;
    border: 0;
    border-bottom: 2px solid transparent;
    background: transparent;
    color: var(--faint);
    font-size: calc(11px * var(--font-scale));
    font-weight: 600;
    cursor: pointer;
  }

  .tabs button.active {
    border-bottom-color: var(--accent);
    color: var(--text);
  }

  .body {
    overflow-y: auto;
    padding: 18px 26px 24px;
  }

  .actions {
    display: flex;
    gap: 8px;
    margin-bottom: 14px;
  }

  .btn {
    padding: 8px 13px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel);
    color: var(--text);
    font-size: calc(11px * var(--font-scale));
    font-weight: 600;
    cursor: pointer;
  }

  .btn:hover:not(:disabled) {
    border-color: var(--accent);
    background: var(--panel-2);
  }

  .btn:disabled {
    opacity: 0.38;
    cursor: not-allowed;
  }

  .btn.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #17120b;
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .btn.ghost {
    color: var(--muted);
  }

  .btn.small {
    padding: 5px 9px;
    font-size: calc(10px * var(--font-scale));
  }

  .btn.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  .game-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .game-card {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 11px 12px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .game-card.current {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    background: var(--accent-soft);
  }

  .game-main {
    min-width: 0;
    flex: 1;
  }

  .game-title {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
  }

  .game-title strong {
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(16px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .badge {
    padding: 2px 5px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .result {
    margin-left: auto;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
  }

  .game-meta {
    display: flex;
    flex-wrap: wrap;
    gap: 9px;
    margin-top: 4px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .game-meta span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .game-buttons {
    display: flex;
    flex: 0 0 auto;
    gap: 5px;
  }

  .empty {
    padding: 30px 12px;
    color: var(--muted);
    font-family: var(--font-display);
    font-size: calc(18px * var(--font-scale));
    text-align: center;
  }

  .pgn-section {
    padding-bottom: 20px;
    margin-bottom: 20px;
    border-bottom: 1px solid var(--border);
  }

  .pgn-section:last-child {
    padding-bottom: 0;
    margin-bottom: 0;
    border-bottom: 0;
  }

  .section-title {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 10px;
    margin-bottom: 8px;
  }

  .section-title h3 {
    margin: 0;
    font-size: calc(14px * var(--font-scale));
    font-weight: 700;
  }

  .section-title span {
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
  }

  textarea {
    width: 100%;
    min-height: 150px;
    padding: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
    resize: vertical;
  }

  .pgn-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 9px;
    flex-wrap: wrap;
  }

  .message {
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    overflow-wrap: anywhere;
  }

  @media (max-width: 700px) {
    .backdrop {
      padding: 12px;
    }
    .panel {
      max-height: calc(100vh - 24px);
    }
    .head,
    .body,
    .tabs {
      padding-left: 18px;
      padding-right: 18px;
    }
    .game-card {
      align-items: flex-start;
      flex-direction: column;
    }
    .game-buttons {
      width: 100%;
      justify-content: flex-end;
    }
  }
</style>
