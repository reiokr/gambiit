<script lang="ts">
  import type { ColorKey, PieceView } from "./types";
  import { pieceUrl } from "./pieces";
  import { i18n } from "./i18n";

  export let name: string;
  export let subtitle: string;
  export let color: ColorKey;
  export let isAgent = false;
  export let thinking = false;
  export let active = false;
  export let statusKey = "waiting";
  export let captured: PieceView[] = [];
  export let material = 0;

  $: advantage = material > 0 ? `+${material}` : `${material}`;
</script>

<div class="player" class:active class:agent={isAgent} data-side={color}>
  <div class="avatar" class:agent-avatar={isAgent}>
    <span>{isAgent ? "♞" : "♙"}</span>
  </div>

  <div class="identity">
    <div class="name-row">
      <span class="name">{name}</span>
      {#if isAgent}<span class="tag">{$i18n("player.ai")}</span>{/if}
    </div>
    <div class="subtitle">{subtitle}</div>
  </div>

  <div class="captured" aria-label={$i18n("player.captured")}>
    {#each captured as piece}
      <img
        src={pieceUrl(piece.color, piece.role)}
        alt=""
        draggable="false"
      />
    {/each}
  </div>

  <div class="meta">
    {#if material !== 0}
      <span class="material">{advantage}</span>
    {/if}
    <span class="status" class:thinking>
      {#if thinking}
        <span class="spinner" aria-hidden="true"></span>
        {$i18n("player.thinking")}
      {:else}
        {$i18n(`player.${statusKey}`)}
      {/if}
    </span>
  </div>
</div>

<style>
  .player {
    display: flex;
    align-items: center;
    gap: 10px;
    min-height: 58px;
    padding: 8px 12px;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: linear-gradient(
      180deg,
      rgba(255, 255, 255, 0.025),
      transparent
    ),
    var(--panel);
    transition:
      border-color 160ms ease,
      box-shadow 160ms ease;
  }

  .player.active {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    box-shadow: 0 0 0 1px var(--accent-soft);
  }

  .avatar {
    display: grid;
    width: 40px;
    height: 40px;
    flex: 0 0 40px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--panel-2);
    color: var(--muted);
    font-size: calc(21px * var(--font-scale));
  }

  .avatar.agent-avatar {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background:
      radial-gradient(circle at 30% 20%, var(--accent-soft), transparent 65%),
      var(--panel-2);
    color: var(--accent-strong);
  }

  .identity {
    min-width: 0;
    flex: 1;
  }

  .name-row {
    display: flex;
    align-items: center;
    gap: 7px;
  }

  .name {
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(17px * var(--font-scale));
    font-weight: 600;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .tag {
    padding: 2px 5px;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.1em;
  }

  .subtitle {
    overflow: hidden;
    margin-top: 2px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .captured {
    display: flex;
    max-width: 140px;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 0;
  }

  .captured img {
    width: 17px;
    height: 17px;
    margin-left: -3px;
    object-fit: contain;
    opacity: 0.82;
  }

  .meta {
    display: flex;
    align-items: center;
    gap: 7px;
    flex: 0 0 auto;
  }

  .material {
    padding: 2px 6px;
    border-radius: 5px;
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale));
    font-weight: 700;
  }

  .status {
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .status.thinking {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--accent);
  }

  .spinner {
    width: 9px;
    height: 9px;
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
</style>
