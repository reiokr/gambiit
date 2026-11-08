<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { i18n } from "./i18n";
  import type { SetupCheck, SetupDiagnostics } from "./types";

  const LABELS: Record<string, string> = {
    opencode: "settings.checkOpencode",
    stockfish: "settings.checkStockfish",
    model: "settings.checkModel",
    credentials: "settings.checkCredentials",
    storage: "settings.checkStorage",
    mcp: "settings.checkMcp",
  };

  const HINTS: Record<string, string> = {
    opencode: "settings.checkOpencodeHint",
    stockfish: "settings.checkStockfishHint",
    model: "settings.checkModelHint",
    credentials: "settings.checkCredentialsHint",
    storage: "settings.checkStorageHint",
    mcp: "settings.checkMcpHint",
  };

  let report: SetupDiagnostics | null = null;
  let busy = false;
  let error = "";

  onMount(() => {
    void refresh();
  });

  async function refresh() {
    if (busy) return;
    busy = true;
    error = "";
    try {
      report = await invoke<SetupDiagnostics>("setup_diagnostics");
    } catch (value) {
      error = value instanceof Error ? value.message : String(value);
    } finally {
      busy = false;
    }
  }

  function label(check: SetupCheck): string {
    return $i18n(LABELS[check.key] ?? check.key);
  }

  function detail(check: SetupCheck): string {
    if (check.path) {
      return `${check.path}${check.version ? ` · ${check.version}` : ""}`;
    }
    if (!check.ok) {
      return $i18n(HINTS[check.key] ?? "settings.checkMissing");
    }
    return check.detail ?? "";
  }

  function status(check: SetupCheck): string {
    if (check.ok) return $i18n("settings.checkReady");
    if (!check.required) return $i18n("settings.checkOptional");
    return $i18n("settings.checkMissing");
  }
</script>

<section class="setup">
  <div class="setup-head">
    <div>
      <h3>{$i18n("settings.checkTitle")}</h3>
      <span>{$i18n("settings.checkHint")}</span>
    </div>
    <button class="mini-btn" type="button" disabled={busy} on:click={refresh}>
      {busy ? $i18n("settings.checkRunning") : $i18n("settings.checkRefresh")}
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}

  {#if report}
    <p class="summary" class:ready={report.overallOk}>
      {report.overallOk
        ? $i18n("settings.checkSummaryReady")
        : $i18n("settings.checkSummaryAction")}
    </p>
    <ul>
      {#each report.checks as check}
        <li class:ok={check.ok} class:optional={!check.required}>
          <span class="dot" aria-hidden="true"></span>
          <div class="copy">
            <strong>{label(check)}</strong>
            {#if detail(check)}<small>{detail(check)}</small>{/if}
          </div>
          <span class="state">{status(check)}</span>
        </li>
      {/each}
    </ul>
    <small class="version">Gambiit {report.appVersion}</small>
  {/if}
</section>

<style>
  .setup {
    padding: 12px 13px;
    margin-bottom: 18px;
    border: 1px solid var(--border);
    border-radius: 10px;
    background: var(--panel-2);
  }

  .setup-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
    margin-bottom: 9px;
  }

  .setup-head h3 {
    margin: 0;
    font-size: calc(12px * var(--font-scale));
    font-weight: 700;
  }

  .setup-head span {
    display: block;
    margin-top: 2px;
    color: var(--faint);
    font-size: calc(9px * var(--font-scale));
  }

  .mini-btn {
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    white-space: nowrap;
    cursor: pointer;
  }

  .mini-btn:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }

  .mini-btn:disabled {
    opacity: 0.5;
    cursor: wait;
  }

  .summary {
    margin: 0 0 8px;
    color: var(--danger);
    font-size: calc(10px * var(--font-scale));
    font-weight: 600;
  }

  .summary.ready {
    color: var(--success);
  }

  ul {
    display: flex;
    margin: 0;
    padding: 0;
    list-style: none;
    flex-direction: column;
    gap: 6px;
  }

  li {
    display: grid;
    grid-template-columns: 7px minmax(0, 1fr) auto;
    align-items: start;
    gap: 8px;
  }

  .dot {
    width: 7px;
    height: 7px;
    margin-top: 4px;
    border-radius: 50%;
    background: var(--danger);
  }

  li.ok .dot {
    background: var(--success);
  }

  li.optional:not(.ok) .dot {
    background: var(--accent);
  }

  .copy {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 1px;
  }

  .copy strong {
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
  }

  .copy small {
    overflow-wrap: anywhere;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.4;
  }

  .state {
    color: var(--danger);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    white-space: nowrap;
  }

  li.ok .state {
    color: var(--success);
  }

  li.optional:not(.ok) .state {
    color: var(--accent);
  }

  .error {
    margin: 8px 0;
    color: var(--danger);
    font-size: calc(10px * var(--font-scale));
  }

  .version {
    display: block;
    margin-top: 9px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    text-align: right;
  }
</style>
