<script lang="ts">
  import { afterUpdate } from "svelte";
  import type { CommentaryCard } from "./types";

  export let cards: CommentaryCard[] = [];
  export let persona = "Theater Master";
  export let model = "";

  let scrollEl: HTMLDivElement | null = null;
  let lastCount = 0;

  afterUpdate(() => {
    if (scrollEl && lastCount !== cards.length) {
      lastCount = cards.length;
      scrollEl.scrollTop = scrollEl.scrollHeight;
    }
  });

  function seconds(ms: number) {
    return `${(ms / 1000).toFixed(1)}s`;
  }
</script>

<div class="theater" bind:this={scrollEl}>
  {#if cards.length === 0}
    <div class="empty">
      <div class="mask">♞</div>
      <h3>The theater is quiet.</h3>
      <p>
        Make a move. <strong>{persona}</strong> will answer with a plan, a
        reason, and possibly a lie.
      </p>
      <small>{model}</small>
    </div>
  {:else}
    <div class="cards">
      {#each cards as card, index (index)}
        <article class="card">
          <header class="card-head">
            <span class="avatar">♞</span>
            <div class="title">
              <span class="move">{card.san}</span>
              <span class="byline">{persona}</span>
            </div>
            {#if card.eval}
              <span class="eval" class:positive={card.eval.startsWith("+")}>
                {card.eval}
              </span>
            {/if}
          </header>

          {#if card.comment}
            <p class="comment">{card.comment}</p>
          {/if}

          {#if card.candidates.length > 0}
            <div class="candidates">
              {#each card.candidates as candidate}
                <div class="candidate" class:rejected={!!candidate.whyRejected}>
                  <span class="candidate-move">{candidate.move}</span>
                  <span class="candidate-why">
                    {candidate.whyRejected ?? candidate.why ?? ""}
                  </span>
                </div>
              {/each}
            </div>
          {/if}

          {#if card.plan}
            <blockquote class="plan">{card.plan}</blockquote>
          {/if}

          <footer class="meta">
            <span>{seconds(card.elapsedMs)}</span>
            {#if card.tokens !== null}
              <span>{card.tokens.toLocaleString()} tokens</span>
            {/if}
            {#if card.cost !== null && card.cost > 0}
              <span>${card.cost.toFixed(4)}</span>
            {/if}
          </footer>
        </article>
      {/each}
    </div>
  {/if}
</div>

<style>
  .theater {
    height: 100%;
    overflow-y: auto;
    padding: 12px 12px 24px;
  }

  .cards {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .card {
    padding: 13px 14px;
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 0 10px 10px 0;
    background: linear-gradient(
      90deg,
      var(--accent-soft),
      transparent 42%
    ),
    var(--panel-2);
  }

  .card-head {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .avatar {
    display: grid;
    width: 27px;
    height: 27px;
    flex: 0 0 27px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--panel);
    color: var(--accent);
    font-size: 15px;
  }

  .title {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 1px;
  }

  .move {
    color: var(--accent-strong);
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 700;
  }

  .byline {
    color: var(--faint);
    font-size: 10px;
  }

  .eval {
    padding: 2px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
  }

  .eval.positive {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
    color: var(--success);
  }

  .comment {
    margin: 10px 0 0;
    color: var(--text);
    font-size: 12.5px;
    line-height: 1.62;
    white-space: pre-line;
  }

  .candidates {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-top: 10px;
  }

  .candidate {
    display: flex;
    gap: 7px;
    align-items: baseline;
    padding: 5px 7px;
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.025);
    font-size: 11px;
    line-height: 1.45;
  }

  .candidate.rejected {
    opacity: 0.72;
  }

  .candidate-move {
    flex: 0 0 auto;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 10.5px;
    font-weight: 700;
  }

  .candidate-why {
    color: var(--muted);
  }

  .plan {
    margin: 10px 0 0;
    padding: 7px 10px;
    border-left: 2px solid var(--border);
    color: var(--muted);
    font-family: var(--font-display);
    font-size: 12.5px;
    font-style: italic;
    line-height: 1.55;
  }

  .meta {
    display: flex;
    gap: 10px;
    margin-top: 9px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: 9px;
  }

  .empty {
    display: flex;
    min-height: 340px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 24px;
    color: var(--muted);
    text-align: center;
  }

  .mask {
    display: grid;
    width: 54px;
    height: 54px;
    margin-bottom: 4px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    background: var(--panel-2);
    color: var(--accent);
    font-size: 28px;
  }

  .empty h3 {
    margin: 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 500;
  }

  .empty p {
    max-width: 280px;
    margin: 0;
    font-size: 12px;
    line-height: 1.6;
  }

  .empty strong {
    color: var(--accent);
    font-weight: 600;
  }

  .empty small {
    margin-top: 4px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: 9px;
  }
</style>
