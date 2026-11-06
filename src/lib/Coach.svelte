<script lang="ts">
  import { afterUpdate } from "svelte";
  import { i18n } from "./i18n";
  import type { CoachMessage } from "./types";

  export let gameId = "";
  export let historyLength = 0;
  export let humanTurn = false;
  export let gameOver = false;
  export let hasHumanMove = false;
  export let messages: CoachMessage[] = [];
  export let busy = false;
  export let disabled = false;
  export let onExplain: () => Promise<string | null> = async () => null;
  export let onHint: (level: number) => Promise<string | null> = async () => null;
  export let onReview: () => Promise<string | null> = async () => null;
  export let onClear: () => Promise<string | null> = async () => null;

  let hintLevel = 0;
  let lastGameId = "";
  let lastHistoryLength = -1;
  let error = "";
  let working = false;
  let list: HTMLDivElement | null = null;

  $: busyNow = busy || working;
  $: if (gameId !== lastGameId || historyLength !== lastHistoryLength) {
    lastGameId = gameId;
    lastHistoryLength = historyLength;
    hintLevel = 0;
  }

  afterUpdate(() => {
    if (list) {
      list.scrollTop = list.scrollHeight;
    }
  });

  function hintName(level: number): string {
    if (level <= 1) return $i18n("coach.hintIdea");
    if (level === 2) return $i18n("coach.hintCandidate");
    return $i18n("coach.hintMove");
  }

  function messageLabel(message: CoachMessage): string {
    if (message.action === "hint") {
      return $i18n("coach.hintLabel", {
        level: message.level ?? 1,
        name: hintName(message.level ?? 1),
      });
    }
    return message.action === "review"
      ? $i18n("coach.reviewLabel")
      : $i18n("coach.explainLabel");
  }

  async function run(
    action: () => Promise<string | null>,
    after?: () => void,
  ) {
    if (busy || working || disabled) return;
    working = true;
    error = "";
    const failure = await action();
    if (failure) {
      error = failure;
    } else {
      after?.();
    }
    working = false;
  }

  async function explain() {
    await run(onExplain);
  }

  async function hint() {
    if (hintLevel >= 3) return;
    const level = hintLevel + 1;
    await run(
      () => onHint(level),
      () => (hintLevel = level),
    );
  }

  async function review() {
    await run(onReview);
  }

  async function clear() {
    if (busyNow || messages.length === 0) return;
    if (!window.confirm($i18n("coach.clearConfirm"))) return;
    await run(onClear, () => (hintLevel = 0));
  }
</script>

<div class="coach">
  <header class="coach-head">
    <span class="mark" aria-hidden="true">C</span>
    <div class="identity">
      <p class="eyebrow">{$i18n("coach.eyebrow")}</p>
      <h3>{$i18n("coach.title")}</h3>
      <span class="honest"><i></i>{$i18n("coach.honest")}</span>
    </div>
    {#if messages.length > 0}
      <button
        class="clear"
        type="button"
        disabled={busyNow}
        on:click={clear}
      >
        {$i18n("coach.clear")}
      </button>
    {/if}
  </header>

  <div class="actions">
    <button
      type="button"
      disabled={busyNow || gameOver || disabled}
      title={gameOver
        ? $i18n("coach.gameOver")
        : disabled
          ? $i18n("coach.waitOpponent")
          : $i18n("coach.explainHint")}
      on:click={explain}
    >
      {$i18n("coach.explain")}
    </button>
    <button
      type="button"
      disabled={busyNow || gameOver || disabled || !humanTurn || hintLevel >= 3}
      title={gameOver
        ? $i18n("coach.gameOver")
        : disabled
          ? $i18n("coach.waitOpponent")
          : !humanTurn
            ? $i18n("coach.waitTurn")
            : hintLevel >= 3
              ? $i18n("coach.hintComplete")
              : $i18n("coach.hintHint")}
      on:click={hint}
    >
      {hintLevel === 0
        ? $i18n("coach.hint")
        : hintLevel === 1
          ? $i18n("coach.nextHint")
          : hintLevel === 2
            ? $i18n("coach.showMove")
            : $i18n("coach.hintComplete")}
    </button>
    <button
      type="button"
      disabled={busyNow || gameOver || disabled || !hasHumanMove}
      title={gameOver
        ? $i18n("coach.gameOver")
        : disabled
          ? $i18n("coach.waitOpponent")
          : !hasHumanMove
            ? $i18n("coach.noMove")
            : $i18n("coach.reviewHint")}
      on:click={review}
    >
      {$i18n("coach.review")}
    </button>
  </div>

  {#if hintLevel > 0}
    <p class="hint-progress">
      {$i18n("coach.hintProgress", {
        level: hintLevel,
        name: hintName(hintLevel),
      })}
    </p>
  {/if}

  <div
    class="coach-list"
    bind:this={list}
    role="log"
    aria-live="polite"
    aria-relevant="additions"
    aria-label={$i18n("coach.title")}
  >
    {#if messages.length === 0}
      <div class="empty">
        <span class="empty-mark" aria-hidden="true">◎</span>
        <strong>{$i18n("coach.emptyTitle")}</strong>
        <p>{$i18n("coach.emptyBody")}</p>
      </div>
    {:else}
      {#each messages as message, index (`${message.action}-${index}`)}
        <article class="card">
          <span class="card-label">{messageLabel(message)}</span>
          <p>{message.text}</p>
          <small>
            {message.model}
            {#if message.tokens}
              · {$i18n("coach.tokens", { count: message.tokens })}
            {/if}
          </small>
        </article>
      {/each}
    {/if}

    {#if busyNow}
      <div class="thinking">
        <i></i>
        {$i18n("coach.thinking")}
      </div>
    {/if}
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</div>

<style>
  .coach {
    display: flex;
    height: 100%;
    flex-direction: column;
    gap: 10px;
    padding: 14px;
    overflow: hidden;
  }

  .coach-head {
    display: flex;
    align-items: flex-start;
    gap: 10px;
  }

  .mark {
    display: grid;
    width: 42px;
    height: 42px;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    border-radius: 50%;
    background: color-mix(in srgb, var(--accent) 12%, var(--panel-2));
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 21px;
  }

  .identity {
    min-width: 0;
    flex: 1;
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

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(20px * var(--font-scale));
    font-weight: 500;
  }

  .honest {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    margin-top: 3px;
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
  }

  .honest i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--accent);
  }

  .clear {
    flex: 0 0 auto;
    padding: 4px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .clear:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
  }

  .actions button {
    padding: 6px 9px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
    cursor: pointer;
  }

  .actions button:hover:not(:disabled) {
    border-color: var(--accent);
  }

  .actions button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .hint-progress {
    margin: 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .coach-list {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding-right: 2px;
  }

  .empty {
    display: flex;
    height: 100%;
    min-height: 220px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 18px 12px;
    text-align: center;
  }

  .empty-mark {
    color: var(--accent);
    font-size: 30px;
    line-height: 1;
  }

  .empty strong {
    font-family: var(--font-display);
    font-size: calc(17px * var(--font-scale));
    font-weight: 500;
  }

  .empty p {
    max-width: 250px;
    margin: 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .card {
    padding: 9px 10px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .card-label {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .card p {
    margin: 4px 0 0;
    color: var(--text);
    font-size: calc(11px * var(--font-scale));
    line-height: 1.55;
    white-space: pre-wrap;
  }

  .card small {
    display: block;
    margin-top: 5px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .thinking {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 8px 4px;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
  }

  .thinking i {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    animation: pulse 1.1s ease-in-out infinite;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 0.35;
      transform: scale(0.85);
    }
    50% {
      opacity: 1;
      transform: scale(1);
    }
  }

  .error {
    align-self: stretch;
    padding: 6px 8px;
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
    border-radius: 6px;
    color: var(--danger);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.45;
  }

  @media (prefers-reduced-motion: reduce) {
    .thinking i {
      animation: none;
    }
  }
</style>
