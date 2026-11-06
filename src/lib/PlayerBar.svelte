<script lang="ts">
  import type { ColorKey, PieceView } from "./types";
  import { pieceUrl, pieceUrls } from "./pieces";
  import { i18n } from "./i18n";
  import { formatClock } from "./clock";

  export let name: string;
  export let subtitle: string;
  export let color: ColorKey;
  export let isAgent = false;
  export let thinking = false;
  export let active = false;
  export let statusKey = "waiting";
  export let captured: PieceView[] = [];
  export let material = 0;
  export let avatar = "";
  export let accent = "";
  export let clockMs: number | null = null;
  export let clockActive = false;
  export let clockStopped = false;
  export let broadcast = false;

  $: advantage = material > 0 ? `+${material}` : `${material}`;
  $: clockLow =
    clockMs !== null &&
    clockMs <= 20_000 &&
    (!clockStopped || clockMs === 0);
</script>

<div
  class="player"
  class:active
  class:agent={isAgent}
  class:broadcast
  data-side={color}
  style={accent ? `--player-accent:${accent}` : undefined}
>
  <div class="avatar" class:agent-avatar={isAgent}>
    {#if isAgent && avatar}
      <img src={avatar} alt="" draggable="false" />
    {:else}
      <span>{isAgent ? "♞" : "♙"}</span>
    {/if}
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
        src={pieceUrl(piece.color, piece.role, $pieceUrls)}
        alt=""
        draggable="false"
      />
    {/each}
  </div>

  {#if clockMs !== null}
    <span
      class="clock"
      class:running={clockActive}
      class:low={clockLow}
      title={$i18n("clock.label")}
      aria-label={`${$i18n("clock.label")}: ${formatClock(clockMs)}`}
    >
      {formatClock(clockMs)}
    </span>
  {/if}

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

  .player.broadcast {
    min-height: 68px;
    padding: 10px 16px;
    gap: 12px;
  }

  .player.broadcast .avatar {
    width: 50px;
    height: 50px;
    flex: 0 0 50px;
    font-size: calc(26px * var(--font-scale));
  }

  .player.broadcast .name {
    font-size: calc(19px * var(--font-scale));
  }

  .player.broadcast .subtitle {
    font-size: calc(11px * var(--font-scale));
  }

  .player.broadcast .clock {
    min-width: 78px;
    padding: 7px 10px;
    font-size: calc(18px * var(--font-scale));
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
    overflow: hidden;
    border-color: color-mix(
      in srgb,
      var(--player-accent, var(--accent)) 55%,
      var(--border)
    );
    background:
      radial-gradient(
        circle at 30% 20%,
        color-mix(
          in srgb,
          var(--player-accent, var(--accent)) 18%,
          transparent
        ),
        transparent 65%
      ),
      var(--panel-2);
    color: var(--accent-strong);
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
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

  .clock {
    min-width: 62px;
    padding: 5px 8px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel-2);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(15px * var(--font-scale));
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    text-align: center;
    letter-spacing: 0.02em;
    transition:
      border-color 150ms ease,
      background 150ms ease,
      color 150ms ease;
  }

  .clock.running {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    background: var(--accent-soft);
    color: var(--accent-strong);
  }

  .clock.low {
    border-color: color-mix(in srgb, var(--danger) 55%, var(--border));
    background: color-mix(in srgb, var(--panel-2) 82%, var(--danger));
    color: var(--danger);
  }

  .clock.low.running {
    animation: clock-pulse 1s ease-in-out infinite;
  }

  @keyframes clock-pulse {
    0%,
    100% {
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 0%, transparent);
    }
    50% {
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 20%, transparent);
    }
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
