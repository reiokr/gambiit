<script lang="ts">
  import type { ColorKey, GameSnapshot, MoveInfo } from "./types";
  import { i18n } from "./i18n";
  import { capturedPieces, materialScore } from "./board";
  import Board from "./Board.svelte";
  import PlayerBar from "./PlayerBar.svelte";

  export let snapshot: GameSnapshot;
  export let agentLabel = "";
  export let agentModel = "";
  export let agentPersona: { avatar: string; accent: string } = {
    avatar: "",
    accent: "#c9a227",
  };
  export let localizedStatus = "";
  export let thinking = false;
  export let chatBusy = false;
  export let coachBusy = false;
  export let drawBusy = false;
  export let selected: string | null = null;
  export let flipped = false;
  export let promotion: {
    from: string;
    to: string;
    options: MoveInfo[];
  } | null = null;
  export let drawNotice: { accepted: boolean; comment: string } | null = null;
  export let showResult = false;
  export let broadcast = false;
  export let canAgentMove = false;
  export let onSquareClick: (square: string) => void = () => {};
  export let onPromotion: (uci: string) => void = () => {};
  export let onRematch: (color: ColorKey) => void = () => {};
  export let onReviewBoard: () => void = () => {};
  export let onNewGame: () => void = () => {};
  export let onTakeback: () => void = () => {};
  export let onOfferDraw: () => void = () => {};
  export let onResign: () => void = () => {};
  export let onAgentMove: () => void = () => {};
  export let onDismissDraw: () => void = () => {};

  $: agentClock = snapshot.clock
    ? snapshot.agentColor === "white"
      ? snapshot.clock.whiteMs
      : snapshot.clock.blackMs
    : null;
  $: humanClock = snapshot.clock
    ? snapshot.humanColor === "white"
      ? snapshot.clock.whiteMs
      : snapshot.clock.blackMs
    : null;
  $: agentClockRunning = snapshot.clock?.running === snapshot.agentColor;
  $: humanClockRunning = snapshot.clock?.running === snapshot.humanColor;
  $: material = materialScore(snapshot.board);
  $: captured = capturedPieces(snapshot.board);
  $: agentMaterial = snapshot.agentColor === "white" ? material.diff : -material.diff;
  $: humanMaterial = -agentMaterial;
  $: agentCaptured =
    snapshot.agentColor === "white" ? captured.byWhite : captured.byBlack;
  $: humanCaptured =
    snapshot.humanColor === "white" ? captured.byWhite : captured.byBlack;
  $: boardInteractive =
    !snapshot.gameOver &&
    !thinking &&
    !chatBusy &&
    !coachBusy &&
    !drawBusy &&
    snapshot.turn === snapshot.humanColor &&
    !promotion;
  $: promotionColor = snapshot.humanColor;
  $: canTakeback =
    !snapshot.gameOver &&
    !thinking &&
    !chatBusy &&
    !coachBusy &&
    !drawBusy &&
    snapshot.lastMove?.color === snapshot.humanColor;

  function playerStatus(color: ColorKey): string {
    if (snapshot.gameOver) {
      if (snapshot.winner === color) return "won";
      if (snapshot.winner) return "lost";
      return "draw";
    }
    if (thinking && color === snapshot.agentColor) return "thinking";
    if (snapshot.turn !== color) return "waiting";
    return "toMove";
  }
</script>

<section class="stage">
  <PlayerBar
    name={agentLabel}
    subtitle={`${agentModel} · ${$i18n(`side.${snapshot.agentColor}`)}`}
    color={snapshot.agentColor}
    isAgent
    thinking={thinking}
    active={snapshot.turn === snapshot.agentColor && !snapshot.gameOver}
    statusKey={playerStatus(snapshot.agentColor)}
    captured={agentCaptured}
    material={agentMaterial}
    avatar={agentPersona.avatar}
    accent={agentPersona.accent}
    clockMs={agentClock}
    clockActive={agentClockRunning}
    clockStopped={snapshot.gameOver}
    broadcast={broadcast}
  />

  <div class="board-shell">
    <Board
      board={snapshot.board}
      legalMoves={snapshot.legalMoves}
      lastMove={snapshot.lastMove}
      checkSquare={snapshot.checkSquare}
      {selected}
      {flipped}
      interactive={boardInteractive}
      animations={snapshot.settings.animations}
      {promotion}
      {promotionColor}
      onSquareClick={onSquareClick}
      onPromotion={onPromotion}
    />

    {#if thinking}
      <div class="thinking-ribbon">
        <span class="spinner"></span>
        {$i18n("app.thinking", { name: agentLabel })}
      </div>
    {:else if drawBusy}
      <div class="thinking-ribbon">
        <span class="spinner"></span>
        {$i18n("app.consideringDraw")}
      </div>
    {/if}

    {#if showResult}
      <div class="result-overlay">
        <div class="result-card">
          <p class="eyebrow">{$i18n("app.finalCurtain")}</p>
          <h2>{localizedStatus}</h2>
          {#if snapshot.winner}
            <p class="winner">
              {$i18n("app.takesTheStage", {
                side: $i18n(`side.${snapshot.winner}`),
              })}
            </p>
          {/if}
          <div class="result-actions">
            <button
              class="btn primary"
              type="button"
              on:click={() => onRematch(snapshot.humanColor)}
            >
              {$i18n("app.rematch")}
            </button>
            <button
              class="btn ghost"
              type="button"
              on:click={onReviewBoard}
            >
              {$i18n("app.reviewBoard")}
            </button>
          </div>
        </div>
      </div>
    {/if}
  </div>

  <PlayerBar
    name={$i18n("app.you")}
    subtitle={`${$i18n(`side.${snapshot.humanColor}`)} · ${$i18n("app.localPlayer")}`}
    color={snapshot.humanColor}
    active={snapshot.turn === snapshot.humanColor && !snapshot.gameOver}
    statusKey={playerStatus(snapshot.humanColor)}
    captured={humanCaptured}
    material={humanMaterial}
    clockMs={humanClock}
    clockActive={humanClockRunning}
    clockStopped={snapshot.gameOver}
    broadcast={broadcast}
  />

  <div class="board-actions">
    <button
      class="btn ghost"
      type="button"
      disabled={chatBusy || coachBusy || drawBusy}
      on:click={onNewGame}
    >
      {$i18n("app.newGame")}
    </button>
    <button
      class="btn ghost"
      type="button"
      disabled={!canTakeback}
      title={$i18n("app.onlyOwnMove")}
      on:click={onTakeback}
    >
      {$i18n("app.takeBack")}
    </button>
    {#if snapshot.settings.drawOffersEnabled}
      <button
        class="btn ghost"
        type="button"
        disabled={snapshot.gameOver || thinking || chatBusy || coachBusy || drawBusy}
        on:click={onOfferDraw}
      >
        {drawBusy ? $i18n("app.offeringDraw") : $i18n("app.offerDraw")}
      </button>
    {/if}
    <button
      class="btn ghost danger"
      type="button"
      disabled={snapshot.gameOver || chatBusy || coachBusy || drawBusy}
      on:click={onResign}
    >
      {$i18n("app.resign")}
    </button>
    {#if canAgentMove && snapshot.settings.responseMode === "button"}
      <button class="btn primary agent-move" type="button" on:click={onAgentMove}>
        {$i18n("app.agentMove")}
      </button>
    {/if}
  </div>

  {#if drawNotice}
    <div class="draw-notice" class:accepted={drawNotice.accepted}>
      <div>
        <strong>
          {drawNotice.accepted
            ? $i18n("app.drawAccepted")
            : $i18n("app.drawDeclined")}
        </strong>
        {#if drawNotice.comment}<span>{drawNotice.comment}</span>{/if}
      </div>
      <button
        type="button"
        aria-label={$i18n("app.dismissNotice")}
        on:click={onDismissDraw}
      >
        ✕
      </button>
    </div>
  {/if}
</section>

<style>
  .stage {
    display: flex;
    width: 100%;
    max-width: max(
      320px,
      min(780px, calc(100vh - 300px - (var(--font-scale) - 1) * 160px))
    );
    margin: 0 auto;
    flex-direction: column;
    gap: 8px;
  }

  .board-shell {
    position: relative;
    width: 100%;
  }

  .thinking-ribbon {
    position: absolute;
    top: 14px;
    left: 50%;
    z-index: 7;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border));
    border-radius: 999px;
    background: color-mix(in srgb, var(--panel) 88%, transparent);
    color: var(--accent-strong);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    pointer-events: none;
    transform: translateX(-50%);
    backdrop-filter: blur(8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.24);
  }

  .board-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 2px;
  }

  .draw-notice {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, var(--danger) 42%, var(--border));
    border-radius: 9px;
    background: color-mix(in srgb, var(--panel) 92%, var(--danger));
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .draw-notice.accepted {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
    background: color-mix(in srgb, var(--panel) 92%, var(--success));
  }

  .draw-notice div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .draw-notice strong {
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
  }

  .draw-notice button {
    flex: 0 0 auto;
    padding: 0 3px;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .draw-notice button:hover {
    color: var(--text);
  }

  .result-overlay {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: grid;
    padding: 24px;
    place-items: center;
    border-radius: 14px;
    background: rgba(8, 6, 4, 0.68);
    backdrop-filter: blur(4px);
  }

  .result-card {
    width: min(420px, 100%);
    padding: 26px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--panel);
    box-shadow: var(--shadow);
    text-align: center;
  }

  .eyebrow {
    margin: 0 0 7px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  .result-card h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(28px * var(--font-scale));
    font-weight: 500;
    line-height: 1.15;
  }

  .winner {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: calc(12px * var(--font-scale));
  }

  .result-actions {
    display: flex;
    justify-content: center;
    gap: 9px;
    margin-top: 20px;
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
    transition:
      border-color 120ms ease,
      background 120ms ease,
      color 120ms ease,
      opacity 120ms ease;
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

  .btn.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  .btn.agent-move {
    margin-left: auto;
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--accent-soft);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  :global(.app.broadcast) .board-actions {
    display: none;
  }

  :global(.app.broadcast) .stage {
    max-width: max(320px, min(1100px, calc(100vh - 345px)));
    gap: 10px;
  }

  @media (max-width: 700px) {
    .board-actions .btn.agent-move {
      margin-left: 0;
    }
  }
</style>
