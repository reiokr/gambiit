<script lang="ts">
  import { afterUpdate } from "svelte";
  import type { MoveRecord } from "./types";

  export let history: MoveRecord[] = [];

  let listEl: HTMLDivElement | null = null;
  let lastPly = 0;

  $: pairs = (() => {
    const rows: Array<{
      number: number;
      white: MoveRecord | undefined;
      black: MoveRecord | undefined;
    }> = [];
    for (let index = 0; index < history.length; index += 2) {
      rows.push({
        number: index / 2 + 1,
        white: history[index],
        black: history[index + 1],
      });
    }
    return rows;
  })();

  afterUpdate(() => {
    if (listEl && lastPly !== history.length) {
      lastPly = history.length;
      listEl.scrollTop = listEl.scrollHeight;
    }
  });
</script>

<div class="moves" bind:this={listEl}>
  {#if pairs.length === 0}
    <div class="empty">
      <span class="empty-icon">✦</span>
      <p>No moves yet.</p>
      <small>The first line is yours.</small>
    </div>
  {:else}
    <div class="rows">
      {#each pairs as row}
        <div class="row">
          <span class="number">{row.number}.</span>
          {#if row.white}
            <span
              class="move"
              class:latest={row.white.ply === history.length}
              title={row.white.uci}
            >
              {row.white.san}
            </span>
          {:else}
            <span class="move"></span>
          {/if}
          {#if row.black}
            <span
              class="move"
              class:latest={row.black.ply === history.length}
              title={row.black.uci}
            >
              {row.black.san}
            </span>
          {:else}
            <span class="move"></span>
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .moves {
    height: 100%;
    overflow-y: auto;
    padding: 10px 12px 20px;
  }

  .rows {
    display: flex;
    flex-direction: column;
    gap: 1px;
  }

  .row {
    display: grid;
    grid-template-columns: 34px 1fr 1fr;
    align-items: center;
    min-height: 29px;
    border-radius: 6px;
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .row:nth-child(odd) {
    background: rgba(255, 255, 255, 0.018);
  }

  .number {
    padding-left: 7px;
    color: var(--faint);
    font-size: 10px;
  }

  .move {
    padding: 4px 8px;
    border-radius: 5px;
    color: var(--text);
    cursor: default;
  }

  .move.latest {
    background: var(--accent-soft);
    color: var(--accent-strong);
    font-weight: 700;
  }

  .empty {
    display: flex;
    height: 100%;
    min-height: 240px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 4px;
    color: var(--muted);
    text-align: center;
  }

  .empty-icon {
    margin-bottom: 5px;
    color: var(--accent);
    font-size: 22px;
  }

  .empty p {
    margin: 0;
    font-family: var(--font-display);
    font-size: 18px;
  }

  .empty small {
    color: var(--faint);
    font-size: 11px;
  }
</style>
