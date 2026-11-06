<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { i18n } from "./i18n";
  import { DIFFICULTIES } from "./difficulty";
  import ModelBadge from "./ModelBadge.svelte";
  import type { Leaderboard } from "./types";

  let data: Leaderboard | null = null;
  let difficulty = "all";
  let loading = true;
  let error = "";

  onMount(() => {
    void load();
  });

  async function load() {
    loading = true;
    error = "";
    try {
      data = await invoke<Leaderboard>("get_leaderboard", {
        difficulty: difficulty === "all" ? null : difficulty,
      });
    } catch (value) {
      error = value instanceof Error ? value.message : String(value);
      data = null;
    } finally {
      loading = false;
    }
  }

  function changeDifficulty(event: Event) {
    difficulty = (event.currentTarget as HTMLSelectElement).value;
    void load();
  }

  function difficultyName(id: string): string {
    const option = DIFFICULTIES.find((item) => item.id === id);
    return option ? $i18n(`difficulty.${option.key}.name`) : id;
  }

  function percent(value: number | null): string {
    return value === null ? "—" : `${value.toFixed(1)}%`;
  }

  function seconds(ms: number | null): string {
    return ms === null ? "—" : `${(ms / 1000).toFixed(1)}s`;
  }

  function tokens(value: number): string {
    return value.toLocaleString();
  }

  function cost(value: number): string {
    return value > 0 ? `$${value.toFixed(4)}` : "—";
  }
</script>

<div class="leaderboard">
  <header class="head">
    <div>
      <p class="eyebrow">{$i18n("leaderboard.eyebrow")}</p>
      <h3>{$i18n("leaderboard.title")}</h3>
      <p>{$i18n("leaderboard.hint")}</p>
    </div>
    <label class="filter">
      <span>{$i18n("leaderboard.difficulty")}</span>
      <select value={difficulty} on:change={changeDifficulty}>
        <option value="all">{$i18n("leaderboard.allDifficulties")}</option>
        {#each data?.difficulties ?? DIFFICULTIES.map((item) => item.id) as id}
          <option value={id}>{difficultyName(id)}</option>
        {/each}
      </select>
    </label>
  </header>

  {#if loading}
    <div class="empty"><span class="spinner"></span>{$i18n("leaderboard.loading")}</div>
  {:else if error}
    <div class="error" role="alert">{error}</div>
  {:else if !data || data.rows.length === 0}
    <div class="empty">
      <strong>{$i18n("leaderboard.empty")}</strong>
      <p>{$i18n("leaderboard.emptyHint")}</p>
    </div>
  {:else}
    <div class="rows">
      {#each data.rows as row}
        <article class="model-card">
          <header class="model-head">
            <ModelBadge model={row.model} size="md" />
            <div class="model-copy">
              <strong>{row.model}</strong>
              <small>
                {$i18n("leaderboard.games", { count: row.games })}
                {#if row.difficulties.length > 0}
                  · {row.difficulties.map(difficultyName).join(", ")}
                {/if}
              </small>
            </div>
            <span class="record" title={$i18n("leaderboard.recordHint")}>
              {row.wins} / {row.draws} / {row.losses}
            </span>
          </header>

          <dl class="stats">
            <div>
              <dt>{$i18n("leaderboard.winRate")}</dt>
              <dd>{row.finished > 0 ? percent(row.winRate) : "—"}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.accuracy")}</dt>
              <dd class:missing={row.accuracy === null}>{percent(row.accuracy)}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.blunders")}</dt>
              <dd>{row.blunders} / {row.mistakes} / {row.inaccuracies}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.bluffRate")}</dt>
              <dd class:missing={row.bluffRate === null}>
                {percent(row.bluffRate)}
                {#if row.truthLensFlags > 0}
                  <small>{$i18n("leaderboard.flags", { count: row.truthLensFlags })}</small>
                {/if}
              </dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.response")}</dt>
              <dd>{seconds(row.avgResponseMs)}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.tokens")}</dt>
              <dd>{tokens(row.tokens)}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.cost")}</dt>
              <dd>{cost(row.cost)}</dd>
            </div>
            <div>
              <dt>{$i18n("leaderboard.analyzed")}</dt>
              <dd>{row.analyzedGames} / {row.games}</dd>
            </div>
          </dl>
        </article>
      {/each}
    </div>
    <p class="footnote">{$i18n("leaderboard.footnote")}</p>
  {/if}
</div>

<style>
  .leaderboard {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
  }

  .eyebrow {
    margin: 0 0 2px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.15em;
    text-transform: uppercase;
  }

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(22px * var(--font-scale));
    font-weight: 500;
  }

  .head p:not(.eyebrow) {
    max-width: 400px;
    margin: 3px 0 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .filter {
    display: flex;
    flex-direction: column;
    gap: 3px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  select {
    min-width: 120px;
    padding: 5px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel-2);
    color: var(--text);
    font-family: var(--font-ui);
    font-size: calc(10px * var(--font-scale));
    text-transform: none;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .model-card {
    padding: 10px 11px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .model-head {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .model-copy {
    display: flex;
    min-width: 0;
    flex: 1;
    flex-direction: column;
    gap: 1px;
  }

  .model-copy strong {
    overflow: hidden;
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-copy small {
    overflow: hidden;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .record {
    padding: 4px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    white-space: nowrap;
  }

  .stats {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 8px 10px;
    padding-top: 9px;
    margin: 9px 0 0;
    border-top: 1px solid var(--border);
  }

  .stats div {
    min-width: 0;
  }

  .stats dt {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.04em;
    line-height: 1.25;
    text-transform: uppercase;
  }

  .stats dd {
    display: flex;
    align-items: baseline;
    gap: 4px;
    margin: 1px 0 0;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    font-weight: 600;
  }

  .stats dd.missing {
    color: var(--faint);
    font-weight: 400;
  }

  .stats dd small {
    color: var(--faint);
    font-size: calc(7px * var(--font-scale));
    font-weight: 400;
  }

  .empty {
    display: flex;
    min-height: 180px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 24px;
    color: var(--muted);
    font-size: calc(11px * var(--font-scale));
    text-align: center;
  }

  .empty strong {
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(18px * var(--font-scale));
    font-weight: 500;
  }

  .empty p {
    max-width: 340px;
    margin: 0;
    line-height: 1.5;
  }

  .spinner {
    width: 18px;
    height: 18px;
    margin-bottom: 4px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.9s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error {
    padding: 10px;
    border: 1px solid color-mix(in srgb, var(--danger) 45%, var(--border));
    border-radius: 8px;
    color: var(--danger);
    font-size: calc(10px * var(--font-scale));
  }

  .footnote {
    margin: 2px 0 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    line-height: 1.5;
  }

  @media (max-width: 560px) {
    .head {
      flex-direction: column;
    }
    .stats {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
