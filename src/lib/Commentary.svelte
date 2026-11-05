<script lang="ts">
  import { afterUpdate } from "svelte";
  import type { CommentaryCard } from "./types";
  import { i18n } from "./i18n";

  export let cards: CommentaryCard[] = [];
  export let persona = "Theater Master";
  export let model = "";

  let scrollEl: HTMLDivElement | null = null;
  let lastCard: HTMLElement | null = null;
  let lastCount = 0;

  afterUpdate(() => {
    if (scrollEl && lastCount !== cards.length) {
      lastCount = cards.length;
      if (!lastCard) {
        scrollEl.scrollTop = 0;
        return;
      }
      const cardTop = lastCard.getBoundingClientRect().top;
      const containerTop = scrollEl.getBoundingClientRect().top;
      scrollEl.scrollTop += cardTop - containerTop;
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
      <h3>{$i18n("commentary.quiet")}</h3>
      <p>
        {$i18n("commentary.emptyPrefix")}
        <strong>{persona}</strong>
        {$i18n("commentary.emptySuffix")}
      </p>
      <small>{model}</small>
    </div>
  {:else}
    <div class="cards">
      {#each cards as card, index (index)}
        <article class="card" bind:this={lastCard}>
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
              <span>
                {card.tokens.toLocaleString()}
                {$i18n("commentary.tokens")}
              </span>
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
    font-size: calc(15px * var(--font-scale));
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
    font-size: calc(13px * var(--font-scale));
    font-weight: 700;
  }

  .byline {
    color: var(--faint);
    font-size: calc(10px * var(--font-scale));
  }

  .eval {
    padding: 2px 7px;
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    font-weight: 700;
  }

  .eval.positive {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
    color: var(--success);
  }

  .comment {
    margin: 10px 0 0;
    color: var(--text);
    font-size: calc(12.5px * var(--font-scale));
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
    font-size: calc(11px * var(--font-scale));
    line-height: 1.45;
  }

  .candidate.rejected {
    opacity: 0.72;
  }

  .candidate-move {
    flex: 0 0 auto;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(10.5px * var(--font-scale));
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
    font-size: calc(12.5px * var(--font-scale));
    font-style: italic;
    line-height: 1.55;
  }

  .meta {
    display: flex;
    gap: 10px;
    margin-top: 9px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
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
    font-size: calc(28px * var(--font-scale));
  }

  .empty h3 {
    margin: 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(22px * var(--font-scale));
    font-weight: 500;
  }

  .empty p {
    max-width: 280px;
    margin: 0;
    font-size: calc(12px * var(--font-scale));
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
    font-size: calc(9px * var(--font-scale));
  }
</style>
