<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import Board from "./Board.svelte";
  import { boardFromFen } from "./board";
  import { i18n } from "./i18n";
  import type {
    AnalysisClass,
    AnalysisPly,
    AnalysisReport,
    ColorKey,
    MoveRecord,
  } from "./types";

  export let gameId = "";
  export let historyLength = 0;
  export let humanColor: ColorKey = "white";

  const DEPTHS = [8, 10, 12, 14, 16];
  const SIDES: ColorKey[] = ["white", "black"];
  const WIDTH = 640;
  const HEIGHT = 180;
  const PAD = 10;

  let depth = 12;
  let report: AnalysisReport | null = null;
  let analyzing = false;
  let error = "";
  let selectedIndex = 0;
  let analyzedGameId = "";
  let analyzedHistoryLength = -1;
  let token = 0;

  $: if (gameId !== analyzedGameId) {
    analyzedGameId = gameId;
    analyzedHistoryLength = historyLength;
    report = null;
    error = "";
    selectedIndex = 0;
    analyzing = false;
    token += 1;
  }

  $: if (historyLength !== analyzedHistoryLength) {
    analyzedHistoryLength = historyLength;
    report = null;
    error = "";
    selectedIndex = 0;
    analyzing = false;
    token += 1;
  }

  $: selected = report?.positions[selectedIndex] ?? null;
  $: selectedBoard = selected ? boardFromFen(selected.fenAfter) : [];
  $: selectedMarker = selected ? markerFor(selected) : null;
  $: flagged = report
    ? report.positions.filter((position) => position.claimMismatch)
    : [];
  $: whiteHasMoves = report
    ? report.positions.some((position) => position.color === "white")
    : false;
  $: blackHasMoves = report
    ? report.positions.some((position) => position.color === "black")
    : false;

  $: graphPoints = report
    ? [
        { cp: report.initialEvalCp },
        ...report.positions.map((position) => ({ cp: position.evalAfterCp })),
      ]
    : [];
  $: graphCoords = graphPoints.map((point, index) => ({
    cp: point.cp,
    x:
      PAD +
      (graphPoints.length <= 1
        ? 0
        : (index / (graphPoints.length - 1)) * (WIDTH - PAD * 2)),
    y: evalY(point.cp),
  }));
  $: graphPath = graphCoords
    .map(
      (point, index) =>
        `${index === 0 ? "M" : "L"}${point.x.toFixed(1)} ${point.y.toFixed(1)}`,
    )
    .join(" ");
  $: areaPath = graphCoords.length
    ? `${graphPath} L${graphCoords[graphCoords.length - 1].x.toFixed(1)} ${HEIGHT / 2} L${graphCoords[0].x.toFixed(1)} ${HEIGHT / 2} Z`
    : "";
  $: selectedPoint = graphCoords[selectedIndex + 1] ?? null;

  function evalY(cp: number): number {
    const clamped = Math.max(-900, Math.min(900, cp));
    const normalized = Math.tanh(clamped / 340);
    return HEIGHT / 2 - normalized * (HEIGHT / 2 - PAD);
  }

  function formatEval(cp: number): string {
    if (cp >= 9_900) return "+M";
    if (cp <= -9_900) return "−M";
    const pawns = cp / 100;
    const sign = pawns >= 0 ? "+" : "−";
    return `${sign}${Math.abs(pawns).toFixed(2)}`;
  }

  function formatLoss(cp: number): string {
    if (cp <= 0) return "0.00";
    return `−${(cp / 100).toFixed(2)}`;
  }

  function formatClaim(cp: number | null): string {
    if (cp === null) return "—";
    return `+${(cp / 100).toFixed(2)}`;
  }

  function markerFor(position: AnalysisPly): MoveRecord {
    return {
      ply: position.ply,
      color: position.color,
      san: position.san,
      uci: position.uci,
      from: position.uci.slice(0, 2),
      to: position.uci.slice(2, 4),
      captured: null,
      fenAfter: position.fenAfter,
      comment: null,
      eval: null,
    };
  }

  function moveLabel(position: AnalysisPly): string {
    return `${Math.ceil(position.ply / 2)}${position.color === "white" ? "." : "…"} ${position.san}`;
  }

  function accuracyFor(color: ColorKey): number {
    return color === "white"
      ? (report?.whiteAccuracy ?? 0)
      : (report?.blackAccuracy ?? 0);
  }

  function hasMovesFor(color: ColorKey): boolean {
    return color === "white" ? whiteHasMoves : blackHasMoves;
  }

  function statFor(
    color: ColorKey,
    kind: "blunders" | "mistakes" | "inaccuracies",
  ): number {
    if (!report) return 0;
    const key = `${color}${kind[0].toUpperCase()}${kind.slice(1)}` as
      | "whiteBlunders"
      | "whiteMistakes"
      | "whiteInaccuracies"
      | "blackBlunders"
      | "blackMistakes"
      | "blackInaccuracies";
    return report[key];
  }

  function classificationLabel(value: AnalysisClass): string {
    return $i18n(`analysis.class.${value}`);
  }

  async function analyze() {
    if (!historyLength || analyzing) return;
    const run = ++token;
    analyzing = true;
    error = "";
    report = null;
    try {
      const result = await invoke<AnalysisReport>("analyze_game", {
        depth,
      });
      if (run !== token) return;
      report = result;
      selectedIndex = Math.max(0, result.positions.length - 1);
    } catch (value) {
      if (run === token) {
        error = value instanceof Error ? value.message : String(value);
      }
    } finally {
      if (run === token) analyzing = false;
    }
  }

  function selectFlagged(position: AnalysisPly) {
    const index = report?.positions.findIndex(
      (entry) => entry.ply === position.ply,
    );
    if (index !== undefined && index >= 0) {
      selectedIndex = index;
    }
  }
</script>

<div class="analysis">
  <header class="head">
    <div class="head-copy">
      <p class="eyebrow">{$i18n("analysis.eyebrow")}</p>
      <h3>{$i18n("analysis.title")}</h3>
      <p class="hint">{$i18n("analysis.hint")}</p>
    </div>
    <div class="controls">
      <label class="depth">
        <span>{$i18n("analysis.depth")}</span>
        <select bind:value={depth} disabled={analyzing}>
          {#each DEPTHS as option}
            <option value={option}>{option}</option>
          {/each}
        </select>
      </label>
      <button
        class="run"
        type="button"
        disabled={!historyLength || analyzing}
        on:click={analyze}
      >
        {analyzing
          ? $i18n("analysis.running")
          : report
            ? $i18n("analysis.rerun")
            : $i18n("analysis.run")}
      </button>
    </div>
  </header>

  {#if !historyLength}
    <div class="empty">
      <div class="empty-mark">⌕</div>
      <strong>{$i18n("analysis.emptyTitle")}</strong>
      <p>{$i18n("analysis.emptyBody")}</p>
    </div>
  {:else if analyzing}
    <div class="empty working">
      <div class="spinner-slow"></div>
      <strong>{$i18n("analysis.workingTitle")}</strong>
      <p>{$i18n("analysis.workingBody", { count: historyLength, depth })}</p>
    </div>
  {:else if error}
    <div class="error-box">
      <strong>{$i18n("analysis.failed")}</strong>
      <p>{error}</p>
      <button class="run" type="button" on:click={analyze}>
        {$i18n("analysis.retry")}
      </button>
    </div>
  {:else if report}
    <div class="report">
      <div class="accuracy-grid">
        {#each SIDES as color}
          <article class="accuracy-card" class:human={color === humanColor}>
            <div class="accuracy-top">
              <span class="side">{$i18n(`side.${color}`)}</span>
              <span class="who">
                {color === humanColor
                  ? $i18n("app.you")
                  : $i18n("player.ai")}
              </span>
            </div>
            <strong class="accuracy">
              {hasMovesFor(color)
                ? `${accuracyFor(color).toFixed(1)}%`
                : "—"}
            </strong>
            <span class="accuracy-caption">{$i18n("analysis.accuracy")}</span>
            <div class="mini-stats">
              <span class="mini-stat blunder" title={$i18n("analysis.class.blunder")}>
                <i></i>{statFor(color, "blunders")}
              </span>
              <span class="mini-stat mistake" title={$i18n("analysis.class.mistake")}>
                <i></i>{statFor(color, "mistakes")}
              </span>
              <span
                class="mini-stat inaccuracy"
                title={$i18n("analysis.class.inaccuracy")}
              >
                <i></i>{statFor(color, "inaccuracies")}
              </span>
            </div>
          </article>
        {/each}
      </div>

      <section class="graph-card" aria-label={$i18n("analysis.graph")}>
        <div class="section-head">
          <h4>{$i18n("analysis.graph")}</h4>
          <span>{$i18n("analysis.depthValue", { depth: report.depth })}</span>
        </div>
        <div class="graph-axis">
          <span>{$i18n("analysis.whiteAdvantage")}</span>
          <span>{$i18n("analysis.blackAdvantage")}</span>
        </div>
        <svg
          class="eval-graph"
          viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
          preserveAspectRatio="none"
          role="img"
          aria-label={$i18n("analysis.graph")}
        >
          <line
            class="midline"
            x1={PAD}
            x2={WIDTH - PAD}
            y1={HEIGHT / 2}
            y2={HEIGHT / 2}
          />
          <path class="area" d={areaPath}></path>
          <path class="line" d={graphPath}></path>
          {#each report.positions as position, index}
            {#if position.classification !== "best" && position.classification !== "good"}
              <circle
                class={`node ${position.classification}`}
                cx={graphCoords[index + 1]?.x ?? 0}
                cy={graphCoords[index + 1]?.y ?? HEIGHT / 2}
                r={3.4}
              ></circle>
            {/if}
          {/each}
          {#if selectedPoint}
            <line
              class="cursor"
              x1={selectedPoint.x}
              x2={selectedPoint.x}
              y1={PAD}
              y2={HEIGHT - PAD}
            ></line>
            <circle
              class="cursor-dot"
              cx={selectedPoint.x}
              cy={selectedPoint.y}
              r={5.5}
            ></circle>
          {/if}
        </svg>
        <div class="graph-legend">
          <span><i class="legend-dot inaccuracy"></i>{$i18n("analysis.class.inaccuracy")}</span>
          <span><i class="legend-dot mistake"></i>{$i18n("analysis.class.mistake")}</span>
          <span><i class="legend-dot blunder"></i>{$i18n("analysis.class.blunder")}</span>
        </div>
      </section>

      <section class="truth-card" class:mismatch={flagged.length > 0}>
        <div class="section-head">
          <h4>
            <span class="lens">◉</span>
            {$i18n("analysis.truthLens")}
          </h4>
          <span>
            {flagged.length > 0
              ? flagged.length === 1
                ? $i18n("analysis.claimsFlaggedOne", { count: flagged.length })
                : $i18n("analysis.claimsFlagged", { count: flagged.length })
              : $i18n("analysis.claimsClean")}
          </span>
        </div>
        {#if flagged.length > 0}
          <p class="truth-hint">{$i18n("analysis.truthHint")}</p>
          <div class="flag-list">
            {#each flagged as position}
              <button
                class="flag"
                type="button"
                on:click={() => selectFlagged(position)}
              >
                <span>{moveLabel(position)}</span>
                <span class="flag-eval">
                  {$i18n("analysis.claimed")}
                  {position.claimedEval ?? "—"}
                  <b>{formatClaim(position.claimGapCp)}</b>
                </span>
              </button>
            {/each}
          </div>
        {:else}
          <p class="truth-hint clean">{$i18n("analysis.truthCleanBody")}</p>
        {/if}
      </section>

      <section class="move-card">
        <div class="section-head">
          <h4>{$i18n("analysis.moves")}</h4>
          <span>{$i18n("analysis.selectMove")}</span>
        </div>
        <div class="ply-list">
          {#each report.positions as position, index}
            <button
              class="ply-row"
              class:active={index === selectedIndex}
              type="button"
              on:click={() => (selectedIndex = index)}
            >
              <span class="ply-no">{Math.ceil(position.ply / 2)}</span>
              <span class="ply-color" class:black={position.color === "black"}></span>
              <span class="ply-san">{position.san}</span>
              <span class="ply-eval">{formatEval(position.evalAfterCp)}</span>
              <span class={`chip ${position.classification}`}>
                {classificationLabel(position.classification)}
              </span>
            </button>
          {/each}
        </div>
      </section>

      {#if selected}
        <section class="detail-card">
          <div class="detail-head">
            <div>
              <p class="eyebrow">{$i18n("analysis.selectedMove")}</p>
              <h4>{moveLabel(selected)}</h4>
            </div>
            <span class={`chip large ${selected.classification}`}>
              {classificationLabel(selected.classification)}
            </span>
          </div>

          <div class="detail-body">
            <div class="mini-board">
              <Board
                board={selectedBoard}
                lastMove={selectedMarker}
                interactive={false}
                flipped={false}
                animations={false}
              />
            </div>
            <dl class="facts">
              <div>
                <dt>{$i18n("analysis.evalBefore")}</dt>
                <dd>{formatEval(selected.evalBeforeCp)}</dd>
              </div>
              <div>
                <dt>{$i18n("analysis.evalAfter")}</dt>
                <dd>{formatEval(selected.evalAfterCp)}</dd>
              </div>
              <div>
                <dt>{$i18n("analysis.bestMove")}</dt>
                <dd class="best">{selected.bestSan ?? selected.bestUci ?? "—"}</dd>
              </div>
              <div>
                <dt>{$i18n("analysis.loss")}</dt>
                <dd class:lossy={selected.lossCp >= 100}>
                  {formatLoss(selected.lossCp)}
                </dd>
              </div>
              <div>
                <dt>{$i18n("analysis.claimed")}</dt>
                <dd>{selected.claimedEval ?? "—"}</dd>
              </div>
              <div>
                <dt>{$i18n("analysis.claimGap")}</dt>
                <dd class:mismatch={selected.claimMismatch}>
                  {formatClaim(selected.claimGapCp)}
                </dd>
              </div>
            </dl>
          </div>

          {#if selected.claimMismatch}
            <p class="claim-warning">
              {$i18n("analysis.claimWarning")}
            </p>
          {/if}
        </section>
      {/if}

      <p class="footnote">{$i18n("analysis.footnote")}</p>
    </div>
  {/if}
</div>

<style>
  .analysis {
    height: 100%;
    overflow-y: auto;
    padding: 14px 14px 22px;
    scrollbar-width: thin;
  }

  .head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    padding-bottom: 12px;
    margin-bottom: 12px;
    border-bottom: 1px solid var(--border);
  }

  .head-copy {
    min-width: 0;
  }

  .eyebrow {
    margin: 0 0 3px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  h3 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(21px * var(--font-scale));
    font-weight: 500;
    line-height: 1.15;
  }

  .hint {
    max-width: 230px;
    margin: 4px 0 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .controls {
    display: flex;
    flex: 0 0 auto;
    flex-direction: column;
    align-items: stretch;
    gap: 6px;
  }

  .depth {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  select {
    padding: 3px 4px;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--panel-2);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
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

  .empty {
    display: flex;
    min-height: 320px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 7px;
    padding: 26px 18px;
    color: var(--muted);
    text-align: center;
  }

  .empty-mark {
    display: grid;
    width: 46px;
    height: 46px;
    margin-bottom: 4px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    color: var(--accent);
    font-size: calc(22px * var(--font-scale));
  }

  .empty strong {
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(19px * var(--font-scale));
    font-weight: 500;
  }

  .empty p {
    max-width: 270px;
    margin: 0;
    font-size: calc(11px * var(--font-scale));
    line-height: 1.6;
  }

  .working {
    min-height: 300px;
  }

  .spinner-slow {
    width: 34px;
    height: 34px;
    margin-bottom: 7px;
    border: 2px solid var(--border);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 1.2s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-box {
    padding: 14px;
    border: 1px solid color-mix(in srgb, var(--danger) 50%, var(--border));
    border-radius: 10px;
    background: color-mix(in srgb, var(--panel) 90%, var(--danger));
  }

  .error-box strong {
    font-size: calc(12px * var(--font-scale));
  }

  .error-box p {
    margin: 5px 0 9px;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
    overflow-wrap: anywhere;
  }

  .report {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .accuracy-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }

  .accuracy-card {
    padding: 10px 11px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .accuracy-card.human {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  }

  .accuracy-top {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 6px;
  }

  .side {
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
    font-weight: 700;
  }

  .who {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.08em;
    text-transform: uppercase;
  }

  .accuracy {
    display: block;
    margin-top: 2px;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(25px * var(--font-scale));
    font-weight: 500;
    line-height: 1.1;
  }

  .accuracy-caption {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.1em;
    text-transform: uppercase;
  }

  .mini-stats {
    display: flex;
    gap: 8px;
    margin-top: 7px;
    padding-top: 6px;
    border-top: 1px solid var(--border);
  }

  .mini-stat {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
  }

  .mini-stat i,
  .legend-dot {
    display: inline-block;
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--muted);
  }

  .mini-stat.blunder i,
  .legend-dot.blunder {
    background: var(--danger);
  }

  .mini-stat.mistake i,
  .legend-dot.mistake {
    background: #e08a4a;
  }

  .mini-stat.inaccuracy i,
  .legend-dot.inaccuracy {
    background: #d9b34a;
  }

  .graph-card,
  .truth-card,
  .move-card,
  .detail-card {
    padding: 10px 11px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
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

  .graph-axis {
    display: flex;
    justify-content: space-between;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .eval-graph {
    display: block;
    width: 100%;
    height: 132px;
    margin-top: 2px;
  }

  .midline {
    stroke: var(--border);
    stroke-width: 1;
    stroke-dasharray: 4 5;
  }

  .area {
    fill: var(--accent-soft);
    opacity: 0.55;
  }

  .line {
    fill: none;
    stroke: var(--accent);
    stroke-width: 1.8;
  }

  .node {
    stroke: var(--panel);
    stroke-width: 1;
  }

  .node.inaccuracy {
    fill: #d9b34a;
  }

  .node.mistake {
    fill: #e08a4a;
  }

  .node.blunder {
    fill: var(--danger);
  }

  .cursor {
    stroke: var(--text);
    stroke-width: 0.8;
    stroke-dasharray: 2 4;
    opacity: 0.45;
  }

  .cursor-dot {
    fill: var(--text);
    stroke: var(--panel);
    stroke-width: 1.5;
  }

  .graph-legend {
    display: flex;
    flex-wrap: wrap;
    gap: 9px;
    margin-top: 2px;
    color: var(--faint);
    font-size: calc(8px * var(--font-scale));
  }

  .graph-legend span {
    display: inline-flex;
    align-items: center;
    gap: 3px;
  }

  .truth-card.mismatch {
    border-color: color-mix(in srgb, #e08a4a 45%, var(--border));
  }

  .lens {
    color: var(--accent);
  }

  .truth-hint {
    margin: 0 0 7px;
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.5;
  }

  .truth-hint.clean {
    margin: 0;
  }

  .flag-list {
    display: flex;
    max-height: 150px;
    flex-direction: column;
    gap: 4px;
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .flag {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    padding: 6px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--text);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
    text-align: left;
  }

  .flag:hover {
    border-color: #e08a4a;
  }

  .flag-eval {
    color: var(--muted);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .flag-eval b {
    color: #e08a4a;
    font-weight: 700;
  }

  .ply-list {
    display: flex;
    max-height: 235px;
    flex-direction: column;
    gap: 2px;
    overflow-y: auto;
    scrollbar-width: thin;
  }

  .ply-row {
    display: grid;
    grid-template-columns: 22px 7px 1fr auto auto;
    align-items: center;
    gap: 5px;
    padding: 4px 5px;
    border: 1px solid transparent;
    border-radius: 5px;
    background: transparent;
    color: var(--text);
    cursor: pointer;
    text-align: left;
  }

  .ply-row:hover {
    background: var(--panel);
  }

  .ply-row.active {
    border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
    background: var(--accent-soft);
  }

  .ply-no {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    text-align: right;
  }

  .ply-color {
    width: 6px;
    height: 6px;
    border: 1px solid var(--muted);
    border-radius: 50%;
    background: #f2e8d5;
  }

  .ply-color.black {
    border-color: var(--faint);
    background: #2b2723;
  }

  .ply-san {
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    font-weight: 600;
  }

  .ply-eval {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .chip {
    padding: 1px 5px;
    border: 1px solid var(--border);
    border-radius: 999px;
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.02em;
    white-space: nowrap;
  }

  .chip.large {
    padding: 3px 8px;
    font-size: calc(8px * var(--font-scale));
  }

  .chip.best {
    border-color: color-mix(in srgb, var(--accent) 60%, var(--border));
    color: var(--accent);
    background: var(--accent-soft);
  }

  .chip.good {
    color: var(--muted);
  }

  .chip.inaccuracy {
    border-color: color-mix(in srgb, #d9b34a 55%, var(--border));
    color: #d9b34a;
  }

  .chip.mistake {
    border-color: color-mix(in srgb, #e08a4a 55%, var(--border));
    color: #e08a4a;
  }

  .chip.blunder {
    border-color: color-mix(in srgb, var(--danger) 55%, var(--border));
    color: var(--danger);
  }

  .detail-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 8px;
    margin-bottom: 8px;
  }

  .detail-head h4 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(18px * var(--font-scale));
    font-weight: 500;
  }

  .detail-body {
    display: grid;
    grid-template-columns: 132px 1fr;
    gap: 10px;
    align-items: start;
  }

  .mini-board {
    width: 132px;
  }

  .mini-board :global(.board) {
    border-width: 4px;
    border-radius: 7px;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.3);
  }

  .mini-board :global(.piece) {
    filter: drop-shadow(0 1px 1px rgba(0, 0, 0, 0.28));
  }

  .mini-board :global(.coord) {
    font-size: 6px;
  }

  .facts {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 5px 9px;
    margin: 0;
  }

  .facts div {
    min-width: 0;
  }

  .facts dt {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    letter-spacing: 0.06em;
    text-transform: uppercase;
  }

  .facts dd {
    margin: 1px 0 0;
    overflow: hidden;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    font-weight: 600;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .facts dd.best {
    color: var(--accent);
  }

  .facts dd.lossy {
    color: #e08a4a;
  }

  .facts dd.mismatch {
    color: var(--danger);
  }

  .claim-warning {
    padding: 7px 8px;
    margin: 9px 0 0;
    border: 1px solid color-mix(in srgb, var(--danger) 35%, var(--border));
    border-radius: 6px;
    background: color-mix(in srgb, var(--panel) 92%, var(--danger));
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.5;
  }

  .footnote {
    margin: 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
    line-height: 1.5;
    text-align: center;
  }

  @media (max-width: 420px) {
    .head {
      flex-direction: column;
    }
    .controls {
      width: 100%;
      flex-direction: row;
      align-items: center;
      justify-content: space-between;
    }
    .detail-body {
      grid-template-columns: 112px 1fr;
    }
    .mini-board {
      width: 112px;
    }
  }
</style>
