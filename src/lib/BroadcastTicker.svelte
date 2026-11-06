<script lang="ts">
  import { i18n } from "./i18n";
  import type { CommentaryCard, MoveRecord } from "./types";

  export let cards: CommentaryCard[] = [];
  export let history: MoveRecord[] = [];
  export let persona = "";
  export let model = "";
  export let avatar = "";
  export let accent = "#c9a227";
  export let thinking = false;

  $: latest = cards.length > 0 ? cards[cards.length - 1] : null;
  $: recent = history.slice(-4);

  function moveLabel(move: MoveRecord): string {
    return `${Math.ceil(move.ply / 2)}${move.color === "white" ? "." : "…"} ${move.san}`;
  }
</script>

<aside class="ticker" style={`--ticker-accent:${accent}`}>
  <div class="live">
    <i></i>
    {$i18n("broadcast.live")}
  </div>

  <div class="identity">
    <span class="avatar">
      {#if avatar}
        <img src={avatar} alt="" draggable="false" />
      {:else}
        <span>♞</span>
      {/if}
    </span>
    <div class="who">
      <strong>{persona}</strong>
      <small>{model}</small>
    </div>
  </div>

  <div class="copy" aria-live="polite">
    {#if thinking}
      <p class="thinking">
        <span class="spinner" aria-hidden="true"></span>
        {$i18n("broadcast.thinking", { name: persona })}
      </p>
    {:else if latest}
      <p class="quote">{latest.comment}</p>
    {:else}
      <p class="idle">{$i18n("broadcast.waiting")}</p>
    {/if}
  </div>

  <div class="meta">
    {#if latest?.eval}
      <span class="eval">{$i18n("broadcast.eval")} {latest.eval}</span>
    {/if}
    {#if recent.length > 0}
      <span class="recent">
        {#each recent as move}
          <b>{moveLabel(move)}</b>
        {/each}
      </span>
    {/if}
  </div>
</aside>

<style>
  .ticker {
    display: grid;
    grid-template-columns: auto minmax(170px, 230px) minmax(0, 1fr) auto;
    align-items: center;
    gap: 18px;
    min-height: 104px;
    padding: 12px 18px;
    border: 1px solid color-mix(in srgb, var(--ticker-accent) 34%, var(--border));
    border-radius: 14px;
    background:
      radial-gradient(
        circle at 0% 50%,
        color-mix(in srgb, var(--ticker-accent) 9%, transparent),
        transparent 42%
      ),
      linear-gradient(180deg, rgba(255, 255, 255, 0.025), transparent),
      var(--panel);
    box-shadow: 0 18px 44px rgba(0, 0, 0, 0.3);
  }

  .live {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 8px;
    border: 1px solid color-mix(in srgb, var(--danger) 42%, var(--border));
    border-radius: 999px;
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    white-space: nowrap;
  }

  .live i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--danger);
    animation: live-pulse 1.8s ease-in-out infinite;
  }

  .identity {
    display: flex;
    min-width: 0;
    align-items: center;
    gap: 9px;
  }

  .avatar {
    display: grid;
    width: 44px;
    height: 44px;
    flex: 0 0 44px;
    place-items: center;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--ticker-accent) 55%, var(--border));
    border-radius: 50%;
    background:
      radial-gradient(
        circle at 30% 20%,
        color-mix(in srgb, var(--ticker-accent) 18%, transparent),
        transparent 65%
      ),
      var(--panel-2);
    color: var(--ticker-accent);
    font-size: calc(22px * var(--font-scale));
  }

  .avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    border-radius: 50%;
  }

  .who {
    min-width: 0;
  }

  .who strong {
    display: block;
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(17px * var(--font-scale));
    font-weight: 600;
    line-height: 1.2;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .who small {
    display: block;
    overflow: hidden;
    margin-top: 2px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy {
    min-width: 0;
  }

  .quote {
    display: -webkit-box;
    overflow: hidden;
    margin: 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(15px * var(--font-scale));
    line-height: 1.5;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 3;
    line-clamp: 3;
  }

  .idle,
  .thinking {
    margin: 0;
    color: var(--muted);
    font-size: calc(12px * var(--font-scale));
    line-height: 1.5;
  }

  .thinking {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .spinner {
    width: 13px;
    height: 13px;
    flex: 0 0 auto;
    border: 2px solid color-mix(in srgb, var(--ticker-accent) 25%, transparent);
    border-top-color: var(--ticker-accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .meta {
    display: flex;
    max-width: 220px;
    flex-direction: column;
    align-items: flex-end;
    gap: 5px;
  }

  .eval {
    padding: 3px 6px;
    border: 1px solid color-mix(in srgb, var(--ticker-accent) 35%, var(--border));
    border-radius: 5px;
    color: var(--ticker-accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    white-space: nowrap;
  }

  .recent {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 3px 7px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.4;
  }

  .recent b {
    font-weight: 500;
    white-space: nowrap;
  }

  @keyframes live-pulse {
    0%,
    100% {
      opacity: 0.45;
      box-shadow: 0 0 0 0 color-mix(in srgb, var(--danger) 0%, transparent);
    }
    50% {
      opacity: 1;
      box-shadow: 0 0 0 3px color-mix(in srgb, var(--danger) 18%, transparent);
    }
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 820px) {
    .ticker {
      grid-template-columns: auto minmax(0, 1fr) auto;
    }

    .copy {
      grid-column: 1 / -1;
    }

    .meta {
      max-width: 160px;
    }
  }

  @media (max-width: 560px) {
    .ticker {
      grid-template-columns: auto minmax(0, 1fr);
    }

    .meta {
      display: none;
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .live i,
    .spinner {
      animation: none;
    }
  }
</style>
