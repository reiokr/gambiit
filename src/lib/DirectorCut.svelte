<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Board from "./Board.svelte";
  import { boardFromFen } from "./board";
  import { i18n } from "./i18n";
  import { AGENT_CANCELLED_MESSAGE } from "./agent";
  import type {
    AnalysisClass,
    AnalysisReport,
    DirectorAct,
    DirectorCut,
    MoveRecord,
  } from "./types";

  export let gameId = "";
  export let historyLength = 0;
  export let report: AnalysisReport | null = null;

  let director: DirectorCut | null = null;
  let busy = false;
  let cancelling = false;
  let error = "";
  let token = 0;
  let lastGameId = "";
  let lastHistoryLength = -1;
  let lastReport: AnalysisReport | null = null;

  $: if (
    gameId !== lastGameId ||
    historyLength !== lastHistoryLength ||
    report !== lastReport
  ) {
    lastGameId = gameId;
    lastHistoryLength = historyLength;
    lastReport = report;
    director = null;
    busy = false;
    error = "";
    token += 1;
  }

  $: canCreate = !!report && report.positions.length >= 8 && !busy;

  async function create() {
    if (!report || busy) return;
    const run = ++token;
    busy = true;
    error = "";
    try {
      const result = await invoke<DirectorCut>("director_cut", { report });
      if (run === token) {
        director = result;
      }
    } catch (value) {
      if (run === token) {
        const message = value instanceof Error ? value.message : String(value);
        error = message === AGENT_CANCELLED_MESSAGE ? "" : message;
      }
    } finally {
      if (run === token) {
        busy = false;
      }
    }
  }

  async function cancel() {
    if (cancelling) return;
    cancelling = true;
    try {
      await invoke("cancel_agent");
    } catch {
      // The running command reports the cancellation through its own error.
    } finally {
      cancelling = false;
    }
  }

  function kind(value: DirectorAct["kind"]): string {
    return $i18n(`analysis.directorKind.${value}`);
  }

  function marker(act: DirectorAct): MoveRecord | null {
    if (!act.lastMove) return null;
    return {
      ply: act.lastMove.ply,
      color: act.lastMove.color,
      san: act.lastMove.san,
      uci: act.lastMove.uci,
      from: act.lastMove.from,
      to: act.lastMove.to,
      captured: null,
      fenAfter: act.fen,
      comment: null,
      eval: null,
    };
  }

  function formatEval(cp: number): string {
    if (cp >= 9_900) return "+M";
    if (cp <= -9_900) return "−M";
    const pawns = cp / 100;
    const sign = pawns >= 0 ? "+" : "−";
    return `${sign}${Math.abs(pawns).toFixed(2)}`;
  }

  function formatClaim(cp: number | null): string {
    if (cp === null) return "—";
    return `+${(cp / 100).toFixed(2)}`;
  }

  function classificationLabel(value: AnalysisClass): string {
    return $i18n(`analysis.class.${value}`);
  }
</script>

{#if report}
  <section class="director-card" class:ready={!!director}>
    <div class="section-head">
      <h4>
        <span class="lens">✦</span>
        {$i18n("analysis.directorTitle")}
      </h4>
      <span class="theater-badge">{$i18n("analysis.directorTheater")}</span>
    </div>
    <p class="director-disclaimer">{$i18n("analysis.directorDisclaimer")}</p>

    {#if !director}
      <p class="director-hint">{$i18n("analysis.directorHint")}</p>
      <div class="director-actions">
        <button class="run" type="button" disabled={!canCreate} on:click={create}>
          {busy
            ? $i18n("analysis.directorWriting")
            : $i18n("analysis.directorCreate")}
        </button>
        {#if busy}
          <button
            class="cancel-run"
            type="button"
            disabled={cancelling}
            on:click={cancel}
          >
            {cancelling
              ? $i18n("analysis.directorStopping")
              : $i18n("analysis.directorCancel")}
          </button>
        {/if}
        {#if report.positions.length < 8}
          <small>{$i18n("analysis.directorTooShort")}</small>
        {/if}
      </div>
      {#if error}
        <p class="director-error">{error}</p>
      {/if}
    {:else}
      <article class="director-cut">
        <header class="director-title">
          <h3>{director.title}</h3>
          <p>{director.tagline}</p>
        </header>

        {#each director.acts as act, index}
          <section class="director-act">
            <span class="act-kicker">
              {$i18n("analysis.directorAct", { number: index + 1 })} ·
              {kind(act.kind)}
            </span>
            <h4>{act.title}</h4>
            <div class="act-body">
              <div class="director-board">
                <Board
                  board={boardFromFen(act.fen)}
                  lastMove={marker(act)}
                  interactive={false}
                  flipped={false}
                  animations={false}
                />
              </div>
              <div class="act-copy">
                <p>{act.text}</p>
                {#if act.quote}
                  <blockquote>
                    “{act.quote}”
                    <cite>— {act.quotePersona ?? "?"}</cite>
                  </blockquote>
                {/if}
                {#if act.truthLens.length > 0}
                  <div class="act-flags">
                    <strong>{$i18n("analysis.directorTruthLens")}</strong>
                    <ul>
                      {#each act.truthLens as flag}
                        <li>
                          {flag.san} · “{flag.claimedEval}” ·
                          {$i18n("analysis.directorGap", {
                            value: formatClaim(flag.gapCp),
                          })}
                        </li>
                      {/each}
                    </ul>
                  </div>
                {/if}
                <small class="act-meta">
                  {act.san} · {$i18n("analysis.directorEval")}
                  {formatEval(act.evalCp)}
                  {#if act.classification}
                    · {classificationLabel(act.classification)}
                  {/if}
                </small>
              </div>
            </div>
          </section>
        {/each}

        <p class="director-closing">{director.closing}</p>
        <footer class="director-footer">
          <span>
            {director.model}
            {#if director.tokens}
              · {$i18n("analysis.directorTokens", {
                count: director.tokens,
              })}
            {/if}
          </span>
          <button class="run" type="button" disabled={busy} on:click={create}>
            {busy
              ? $i18n("analysis.directorWriting")
              : $i18n("analysis.directorRegenerate")}
          </button>
          {#if busy}
            <button
              class="cancel-run"
              type="button"
              disabled={cancelling}
              on:click={cancel}
            >
              {cancelling
                ? $i18n("analysis.directorStopping")
                : $i18n("analysis.directorCancel")}
            </button>
          {/if}
        </footer>
        {#if error}
          <p class="director-error">{error}</p>
        {/if}
      </article>
    {/if}
  </section>
{/if}

<style>
  .director-card {
    padding: 10px 11px;
    border: 1px solid color-mix(in srgb, var(--accent) 38%, var(--border));
    border-radius: 10px;
    background:
      radial-gradient(
        circle at 100% 0%,
        color-mix(in srgb, var(--accent) 10%, transparent),
        transparent 45%
      ),
      var(--panel-2);
  }

  .director-card.ready {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  }

  .section-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 7px;
  }

  .section-head h4 {
    display: flex;
    align-items: center;
    gap: 5px;
    margin: 0;
    font-size: calc(11px * var(--font-scale));
    font-weight: 700;
  }

  .section-head > span {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .lens {
    color: var(--accent);
  }

  .section-head .theater-badge {
    padding: 2px 6px;
    border: 1px solid color-mix(in srgb, var(--accent) 40%, var(--border));
    border-radius: 999px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .director-disclaimer {
    margin: 0 0 7px;
    color: var(--faint);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.45;
  }

  .director-hint {
    margin: 0 0 9px;
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.5;
  }

  .director-actions {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }

  .director-actions small {
    color: var(--faint);
    font-size: calc(8px * var(--font-scale));
  }

  .director-error {
    margin: 8px 0 0;
    color: var(--danger);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.45;
  }

  .run {
    padding: 7px 11px;
    border: 1px solid var(--accent);
    border-radius: 7px;
    background: var(--accent);
    color: #17120b;
    font-size: calc(10px * var(--font-scale));
    font-weight: 700;
    white-space: nowrap;
    cursor: pointer;
  }

  .run:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .run:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .cancel-run {
    padding: 7px 10px;
    border: 1px solid var(--border);
    border-radius: 7px;
    background: var(--panel);
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    cursor: pointer;
  }

  .cancel-run:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }

  .cancel-run:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  .director-cut {
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .director-title h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(22px * var(--font-scale));
    font-weight: 500;
    line-height: 1.15;
  }

  .director-title p {
    margin: 3px 0 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    font-style: italic;
    line-height: 1.5;
  }

  .director-act {
    padding-top: 11px;
    border-top: 1px solid var(--border);
  }

  .act-kicker {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .director-act h4 {
    margin: 2px 0 7px;
    font-family: var(--font-display);
    font-size: calc(16px * var(--font-scale));
    font-weight: 500;
  }

  .act-body {
    display: grid;
    grid-template-columns: 132px minmax(0, 1fr);
    gap: 11px;
    align-items: start;
  }

  .director-board {
    width: 132px;
  }

  .director-board :global(.board) {
    border-width: 4px;
    border-radius: 7px;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.3);
  }

  .director-board :global(.piece) {
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.28));
  }

  .director-board :global(.coord) {
    font-size: 6px;
  }

  .act-copy p {
    margin: 0;
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.55;
  }

  .act-copy blockquote {
    margin: 7px 0 0;
    padding: 5px 0 5px 8px;
    border-left: 2px solid color-mix(in srgb, var(--accent) 55%, var(--border));
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    font-style: italic;
    line-height: 1.5;
  }

  .act-copy cite {
    display: block;
    margin-top: 2px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    font-style: normal;
  }

  .act-flags {
    margin-top: 7px;
    padding: 6px 7px;
    border: 1px solid color-mix(in srgb, #e08a4a 35%, var(--border));
    border-radius: 6px;
    background: color-mix(in srgb, #e08a4a 7%, transparent);
  }

  .act-flags strong {
    color: #e08a4a;
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .act-flags ul {
    margin: 3px 0 0;
    padding: 0;
    list-style: none;
  }

  .act-flags li {
    color: var(--muted);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.45;
  }

  .act-meta {
    display: block;
    margin-top: 6px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.04em;
  }

  .director-closing {
    padding-top: 10px;
    margin: 0;
    border-top: 1px solid var(--border);
    color: var(--muted);
    font-family: var(--font-display);
    font-size: calc(13px * var(--font-scale));
    font-style: italic;
    line-height: 1.5;
  }

  .director-footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    flex-wrap: wrap;
  }

  .director-footer > span {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
  }

  @media (max-width: 560px) {
    .act-body {
      grid-template-columns: 1fr;
    }
    .director-board {
      width: 120px;
    }
  }
</style>
