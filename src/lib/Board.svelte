<script lang="ts">
  import type { ColorKey, MoveInfo, MoveRecord, SquareView } from "./types";
  import { isLightSquare, orderedSquares } from "./board";
  import { pieceAlt, pieceUrl } from "./pieces";

  export let board: SquareView[] = [];
  export let legalMoves: MoveInfo[] = [];
  export let lastMove: MoveRecord | null = null;
  export let checkSquare: string | null = null;
  export let selected: string | null = null;
  export let flipped = false;
  export let interactive = true;
  export let animations = true;
  export let promotion: { from: string; to: string; options: MoveInfo[] } | null =
    null;
  export let promotionColor: ColorKey = "white";
  export let onSquareClick: (square: string) => void = () => {};
  export let onPromotion: (uci: string) => void = () => {};

  $: squares = orderedSquares(flipped);
  $: pieces = new Map(board.map((square) => [square.square, square.piece]));
  $: targets = selected
    ? legalMoves.filter((move) => move.from === selected)
    : [];
  $: promotionOptions = promotion
    ? promotion.options.filter(
        (move, index, all) =>
          !!move.promotion &&
          all.findIndex((other) => other.promotion === move.promotion) === index,
      )
    : [];

  function pieceAt(square: string) {
    return pieces.get(square) ?? null;
  }

  function targetAt(square: string) {
    return targets.find((move) => move.to === square);
  }

  function isCapture(square: string) {
    const target = targetAt(square);
    return !!target && (!!target.captured || target.isEnPassant);
  }

  function isLast(square: string) {
    return lastMove?.from === square || lastMove?.to === square;
  }

  function showRank(square: string) {
    return square[0] === (flipped ? "h" : "a");
  }

  function showFile(square: string) {
    return square[1] === (flipped ? "8" : "1");
  }

  function handleClick(square: string) {
    if (!interactive || promotion) return;
    onSquareClick(square);
  }
</script>

<div
  class="board"
  class:animations
  aria-label="Chess board"
>
  {#each squares as square (square)}
    {@const piece = pieceAt(square)}
    {@const target = targetAt(square)}
    <button
      type="button"
      class="square"
      class:light={isLightSquare(square)}
      class:dark={!isLightSquare(square)}
      class:last={isLast(square)}
      class:selected={selected === square}
      class:check={checkSquare === square}
      class:legal={!!target}
      class:capture={isCapture(square)}
      class:inactive={!interactive}
      aria-label={`${square}${piece ? `, ${pieceAlt(piece.color, piece.role)}` : ", empty"}${target ? ", legal move" : ""}`}
      aria-disabled={!interactive}
      on:click={() => handleClick(square)}
    >
      {#if piece}
        {#key `${square}-${piece.color}-${piece.role}`}
          <img
            class="piece"
            src={pieceUrl(piece.color, piece.role)}
            alt=""
            draggable="false"
          />
        {/key}
      {/if}

      {#if target && !isCapture(square)}
        <span class="legal-dot" aria-hidden="true"></span>
      {/if}
      {#if target && isCapture(square)}
        <span class="legal-ring" aria-hidden="true"></span>
      {/if}

      {#if showRank(square)}
        <span class="coord rank">{square[1]}</span>
      {/if}
      {#if showFile(square)}
        <span class="coord file">{square[0]}</span>
      {/if}
    </button>
  {/each}

  {#if promotion}
    <div
      class="promotion-backdrop"
      role="presentation"
      on:click|stopPropagation
    >
      <div class="promotion-card" role="dialog" aria-label="Choose promotion">
        <p>Promote to</p>
        <div class="promotion-options">
          {#each promotionOptions as option}
            <button
              type="button"
              title={option.promotion ?? ""}
              aria-label={`Promote to ${option.promotion}`}
              on:click={() => onPromotion(option.uci)}
            >
              <img
                src={pieceUrl(promotionColor, option.promotion ?? "q")}
                alt=""
                draggable="false"
              />
            </button>
          {/each}
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .board {
    position: relative;
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    grid-template-rows: repeat(8, 1fr);
    width: 100%;
    aspect-ratio: 1;
    overflow: hidden;
    border: 10px solid var(--board-border);
    border-radius: 14px;
    background: var(--board-border);
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.06),
      0 24px 60px rgba(0, 0, 0, 0.45);
    user-select: none;
    touch-action: manipulation;
  }

  .square {
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    min-width: 0;
    min-height: 0;
    padding: 0;
    border: 0;
    border-radius: 0;
    appearance: none;
    cursor: pointer;
    transition: filter 120ms ease;
  }

  .square.light {
    background: var(--board-light);
  }

  .square.dark {
    background: var(--board-dark);
  }

  .square.inactive {
    cursor: default;
  }

  .square:not(.inactive):hover {
    filter: brightness(1.06);
  }

  .square.last::before {
    content: "";
    position: absolute;
    inset: 0;
    background: var(--last);
    pointer-events: none;
  }

  .square.selected::after {
    content: "";
    position: absolute;
    inset: 0;
    border: 3px solid var(--accent-strong);
    box-shadow: inset 0 0 0 999px var(--selected);
    pointer-events: none;
    z-index: 3;
  }

  .square.check {
    background-image: radial-gradient(
      circle at center,
      rgba(201, 74, 63, 0.8) 0%,
      rgba(201, 74, 63, 0.35) 42%,
      transparent 72%
    );
  }

  .piece {
    position: relative;
    z-index: 1;
    width: 86%;
    height: 86%;
    object-fit: contain;
    pointer-events: none;
    filter: drop-shadow(0 3px 3px rgba(0, 0, 0, 0.28));
    transform-origin: center;
  }

  .animations .piece {
    animation: piece-in 170ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }

  @keyframes piece-in {
    from {
      transform: scale(0.82) translateY(-3px);
      opacity: 0.65;
    }
    to {
      transform: scale(1) translateY(0);
      opacity: 1;
    }
  }

  .legal-dot {
    position: absolute;
    z-index: 2;
    width: 22%;
    height: 22%;
    border-radius: 50%;
    background: var(--legal);
    box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.18);
    pointer-events: none;
  }

  .legal-ring {
    position: absolute;
    z-index: 2;
    inset: 10%;
    border: 5px solid var(--legal);
    border-radius: 50%;
    opacity: 0.85;
    pointer-events: none;
  }

  .coord {
    position: absolute;
    z-index: 4;
    font-family: var(--font-mono);
    font-size: 9px;
    font-weight: 700;
    line-height: 1;
    opacity: 0.62;
    pointer-events: none;
  }

  .light .coord {
    color: var(--board-dark);
  }

  .dark .coord {
    color: var(--board-light);
  }

  .coord.file {
    right: 4px;
    bottom: 3px;
  }

  .coord.rank {
    left: 4px;
    top: 3px;
  }

  .promotion-backdrop {
    position: absolute;
    inset: 0;
    z-index: 12;
    display: grid;
    place-items: center;
    background: rgba(8, 6, 4, 0.62);
    backdrop-filter: blur(3px);
  }

  .promotion-card {
    min-width: 240px;
    padding: 18px 20px 20px;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--panel);
    box-shadow: var(--shadow);
    text-align: center;
  }

  .promotion-card p {
    margin: 0 0 12px;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
  }

  .promotion-options {
    display: flex;
    justify-content: center;
    gap: 8px;
  }

  .promotion-options button {
    display: grid;
    width: 54px;
    height: 54px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
    cursor: pointer;
    transition:
      background 120ms ease,
      border-color 120ms ease,
      transform 120ms ease;
  }

  .promotion-options button:hover {
    border-color: var(--accent);
    background: var(--panel-3);
    transform: translateY(-1px);
  }

  .promotion-options img {
    width: 40px;
    height: 40px;
    object-fit: contain;
  }

  @media (prefers-reduced-motion: reduce) {
    .animations .piece {
      animation: none;
    }
    .square,
    .promotion-options button {
      transition: none;
    }
  }
</style>
