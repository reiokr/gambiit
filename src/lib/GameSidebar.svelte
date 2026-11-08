<script lang="ts">
  import type { GameSnapshot } from "./types";
  import { i18n } from "./i18n";
  import Analysis from "./Analysis.svelte";
  import Chat from "./Chat.svelte";
  import Coach from "./Coach.svelte";
  import Commentary from "./Commentary.svelte";
  import MoveList from "./MoveList.svelte";

  export let snapshot: GameSnapshot;
  export let activeTab: "moves" | "theater" | "chat" | "coach" | "analysis" =
    "theater";
  export let agentLabel = "";
  export let agentModel = "";
  export let agentPersona: { avatar: string; accent: string };
  export let agentError = "";
  export let hasHumanMove = false;
  export let thinking = false;
  export let chatBusy = false;
  export let coachBusy = false;
  export let drawBusy = false;
  export let canAgentMove = false;
  export let onSendChat: (message: string) => Promise<string | null> = async () =>
    null;
  export let onClearChat: () => Promise<string | null> = async () => null;
  export let onCoachExplain: () => Promise<string | null> = async () => null;
  export let onCoachHint: (level: number) => Promise<string | null> = async () =>
    null;
  export let onCoachReview: () => Promise<string | null> = async () => null;
  export let onCoachClear: () => Promise<string | null> = async () => null;
  export let onAgentMove: () => void = () => {};
</script>

<aside class="sidebar">
  <div
    class="tabs"
    role="tablist"
    aria-label={$i18n("app.gamePanels")}
  >
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "moves"}
      class:active={activeTab === "moves"}
      on:click={() => (activeTab = "moves")}
    >
      {$i18n("app.tabMoves")}
      <span>{snapshot.history.length}</span>
    </button>
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "theater"}
      class:active={activeTab === "theater"}
      on:click={() => (activeTab = "theater")}
    >
      {$i18n("app.tabTheater")}
      <span>{snapshot.commentary.length}</span>
    </button>
    {#if snapshot.settings.chatEnabled}
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "chat"}
        class:active={activeTab === "chat"}
        on:click={() => (activeTab = "chat")}
      >
        {$i18n("app.tabChat")}
        <span>{chatBusy ? "…" : snapshot.chat.length}</span>
      </button>
    {/if}
    {#if snapshot.settings.coachEnabled}
      <button
        type="button"
        role="tab"
        aria-selected={activeTab === "coach"}
        class:active={activeTab === "coach"}
        on:click={() => (activeTab = "coach")}
      >
        {$i18n("app.tabCoach")}
        <span>{coachBusy ? "…" : snapshot.coach.length}</span>
      </button>
    {/if}
    <button
      type="button"
      role="tab"
      aria-selected={activeTab === "analysis"}
      class:active={activeTab === "analysis"}
      on:click={() => (activeTab = "analysis")}
    >
      {$i18n("app.tabAnalysis")}
    </button>
  </div>

  <div class="tab-body">
    {#if activeTab === "moves"}
      <MoveList history={snapshot.history} />
    {:else if activeTab === "theater"}
      <Commentary
        cards={snapshot.commentary}
        persona={agentLabel}
        model={agentModel}
        avatar={agentPersona.avatar}
        accent={agentPersona.accent}
      />
    {/if}
    {#if snapshot.settings.chatEnabled}
      <div class="chat-pane" class:active={activeTab === "chat"}>
        <Chat
          messages={snapshot.chat}
          personaName={agentLabel}
          avatar={agentPersona.avatar}
          accent={agentPersona.accent}
          busy={chatBusy}
          disabled={thinking || coachBusy || drawBusy}
          onSend={onSendChat}
          onClear={onClearChat}
        />
      </div>
    {/if}
    {#if snapshot.settings.coachEnabled}
      <div class="coach-pane" class:active={activeTab === "coach"}>
        <Coach
          gameId={snapshot.gameId}
          historyLength={snapshot.history.length}
          humanTurn={snapshot.turn === snapshot.humanColor && !snapshot.gameOver}
          gameOver={snapshot.gameOver}
          hasHumanMove={hasHumanMove}
          messages={snapshot.coach}
          busy={coachBusy || snapshot.coachThinking}
          disabled={thinking || chatBusy || drawBusy}
          onExplain={onCoachExplain}
          onHint={onCoachHint}
          onReview={onCoachReview}
          onClear={onCoachClear}
        />
      </div>
    {/if}
    <div class="analysis-pane" class:active={activeTab === "analysis"}>
      <Analysis
        gameId={snapshot.gameId}
        historyLength={snapshot.history.length}
        humanColor={snapshot.humanColor}
      />
    </div>
  </div>

  <footer class="sidebar-footer">
    {#if agentError}
      <div class="agent-error">
        <strong>{$i18n("app.opponentUnavailable")}</strong>
        <span>{agentError}</span>
        {#if canAgentMove}
          <button class="btn small" type="button" on:click={onAgentMove}>
            {$i18n("app.retryAgent")}
          </button>
        {/if}
      </div>
    {:else}
      <div class="footer-line">
        <span class="footer-dot"></span>
        <span class="footer-model">{agentModel}</span>
      </div>
      <div class="footer-persona">{agentLabel}</div>
    {/if}
  </footer>
</aside>

<style>
.btn {
  padding: 8px 13px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--panel);
  color: var(--text);
  font-size: calc(11px * var(--font-scale));
  font-weight: 600;
  cursor: pointer;
  transition:
    border-color 120ms ease,
    background 120ms ease,
    color 120ms ease,
    opacity 120ms ease;
}

.btn:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--panel-2);
}

.btn:disabled {
  opacity: 0.38;
  cursor: not-allowed;
}

.btn.small {
  padding: 5px 9px;
  font-size: calc(10px * var(--font-scale));
}

.sidebar {
  position: sticky;
  top: 20px;
  display: flex;
  height: calc(100vh - 100px);
  min-height: 540px;
  flex-direction: column;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--panel);
  box-shadow: 0 16px 40px rgba(0, 0, 0, 0.18);
}

.tabs {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  gap: 2px;
  padding: 7px 7px 0;
  border-bottom: 1px solid var(--border);
  background: var(--panel);
}

.tabs button {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 9px 8px 10px;
  border: 0;
  border-bottom: 2px solid transparent;
  background: transparent;
  color: var(--faint);
  font-size: calc(10.5px * var(--font-scale));
  font-weight: 600;
  cursor: pointer;
}

.tabs button:hover {
  color: var(--muted);
}

.tabs button.active {
  border-bottom-color: var(--accent);
  color: var(--text);
}

.tabs button span {
  padding: 1px 4px;
  border-radius: 4px;
  background: var(--panel-2);
  color: var(--faint);
  font-family: var(--font-mono);
  font-size: calc(8px * var(--font-scale));
}

.tab-body {
  min-height: 0;
  flex: 1;
  overflow: hidden;
}

.analysis-pane,
.chat-pane,
.coach-pane {
  display: none;
  height: 100%;
}

.analysis-pane.active,
.chat-pane.active,
.coach-pane.active {
  display: block;
}

.sidebar-footer {
  flex: 0 0 auto;
  padding: 10px 13px;
  border-top: 1px solid var(--border);
  background: var(--panel);
}

.footer-line {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.footer-dot {
  width: 5px;
  height: 5px;
  flex: 0 0 5px;
  border-radius: 50%;
  background: var(--success);
}

.footer-model {
  overflow: hidden;
  color: var(--muted);
  font-family: var(--font-mono);
  font-size: calc(9px * var(--font-scale));
  text-overflow: ellipsis;
  white-space: nowrap;
}

.footer-persona {
  margin-top: 3px;
  color: var(--faint);
  font-family: var(--font-display);
  font-size: calc(11px * var(--font-scale));
  font-style: italic;
}

.agent-error {
  display: flex;
  flex-direction: column;
  gap: 4px;
  color: var(--muted);
  font-size: calc(10px * var(--font-scale));
  line-height: 1.45;
}

.agent-error strong {
  color: var(--danger);
  font-size: calc(10px * var(--font-scale));
}

.agent-error span {
  max-height: 54px;
  overflow: hidden;
}

.agent-error .btn {
  align-self: flex-start;
  margin-top: 3px;
}


  :global(.app.broadcast) .sidebar {
    display: none;
  }

  @media (max-width: 1120px) {
    .sidebar {
      position: static;
      height: 560px;
      min-height: 0;
    }
  }

  @media (max-width: 700px) {
    .sidebar {
      height: 520px;
    }
  }
</style>
