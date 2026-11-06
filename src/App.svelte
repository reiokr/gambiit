<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import Board from "./lib/Board.svelte";
  import Analysis from "./lib/Analysis.svelte";
  import BroadcastTicker from "./lib/BroadcastTicker.svelte";
  import Chat from "./lib/Chat.svelte";
  import Coach from "./lib/Coach.svelte";
  import Commentary from "./lib/Commentary.svelte";
  import GameLibrary from "./lib/GameLibrary.svelte";
  import MoveList from "./lib/MoveList.svelte";
  import PlayerBar from "./lib/PlayerBar.svelte";
  import SettingsPanel from "./lib/SettingsPanel.svelte";
  import type {
    ClockUpdate,
    ColorKey,
    DrawOfferOutcome,
    GameSnapshot,
    GameSummary,
    MoveInfo,
    Settings,
  } from "./lib/types";
  import { capturedPieces, materialScore } from "./lib/board";
  import {
    applyFontScale,
    applyMotion,
    loadHumanColor,
    loadSettings,
    saveHumanColor,
    saveSettings,
  } from "./lib/settings";
  import { applyTheme } from "./lib/themes";
  import { applyPieceSet } from "./lib/pieces";
  import { playSound } from "./lib/sound";
  import { i18n, localeMeta, setLocale } from "./lib/i18n";
  import { personaFor } from "./lib/personas";

  type Tab = "moves" | "theater" | "chat" | "coach" | "analysis";

  let snapshot: GameSnapshot | null = null;
  let settings: Settings = loadSettings();
  let humanColor: ColorKey = loadHumanColor();
  let selected: string | null = null;
  let flipped = humanColor === "black";
  let thinking = false;
  let loading = true;
  let error = "";
  let activeTab: Tab = "theater";
  let showSettings = false;
  let showGames = false;
  let broadcast = false;
  let games: GameSummary[] = [];
  let pgnPreview = "";
  let models: string[] = [];
  let modelsLoaded = false;
  let appliedSettings = "";
  let promotion: { from: string; to: string; options: MoveInfo[] } | null =
    null;
  let resultDismissed = false;
  let lastSoundPly = 0;
  let chatBusy = false;
  let coachBusy = false;
  let drawBusy = false;
  let drawNotice: { accepted: boolean; comment: string } | null = null;
  let clockPolling = false;
  let announcement = "";

  $: if (snapshot && !snapshot.settings.chatEnabled && activeTab === "chat") {
    activeTab = "theater";
  }
  $: if (snapshot && !snapshot.settings.coachEnabled && activeTab === "coach") {
    activeTab = "theater";
  }
  $: canAgentMove =
    !!snapshot &&
    !snapshot.gameOver &&
    snapshot.turn === snapshot.agentColor &&
    !thinking &&
    !chatBusy &&
    !coachBusy &&
    !drawBusy;
  $: canTakeback =
    !!snapshot &&
    !snapshot.gameOver &&
    !thinking &&
    !chatBusy &&
    !coachBusy &&
    !drawBusy &&
    snapshot.lastMove?.color === snapshot.humanColor;
  $: hasHumanMove =
    !!snapshot &&
    snapshot.history.some((move) => move.color === snapshot!.humanColor);
  $: material = snapshot
    ? materialScore(snapshot.board)
    : { white: 0, black: 0, diff: 0 };
  $: captured = snapshot
    ? capturedPieces(snapshot.board)
    : { byWhite: [], byBlack: [] };
  $: agentMaterial = snapshot
    ? snapshot.agentColor === "white"
      ? material.diff
      : -material.diff
    : 0;
  $: humanMaterial = -agentMaterial;
  $: agentCaptured = snapshot
    ? snapshot.agentColor === "white"
      ? captured.byWhite
      : captured.byBlack
    : [];
  $: humanCaptured = snapshot
    ? snapshot.humanColor === "white"
      ? captured.byWhite
      : captured.byBlack
    : [];
  $: boardInteractive =
    !!snapshot &&
    !snapshot.gameOver &&
    !thinking &&
    !chatBusy &&
    !coachBusy &&
    !drawBusy &&
    snapshot.turn === snapshot.humanColor &&
    !promotion;
  $: promotionColor = snapshot?.humanColor ?? humanColor;
  $: agentClock = snapshot?.clock
    ? snapshot.agentColor === "white"
      ? snapshot.clock.whiteMs
      : snapshot.clock.blackMs
    : null;
  $: humanClock = snapshot?.clock
    ? snapshot.humanColor === "white"
      ? snapshot.clock.whiteMs
      : snapshot.clock.blackMs
    : null;
  $: agentClockRunning = snapshot?.clock?.running === snapshot?.agentColor;
  $: humanClockRunning = snapshot?.clock?.running === snapshot?.humanColor;
  $: agentName = snapshot?.settings.persona ?? settings.persona;
  $: agentModel = snapshot?.settings.model ?? settings.model;
  $: customPersona = snapshot?.settings.customPersona ?? settings.customPersona;
  $: agentPersona =
    agentName === "Custom"
      ? { ...personaFor("Custom"), accent: customPersona.accent }
      : personaFor(agentName);
  $: agentError = snapshot?.agentError ?? "";
  $: localizedStatus = snapshot
    ? $i18n(
        `status.${snapshot.statusKey}`,
        snapshot.statusSide
          ? { side: $i18n(`side.${snapshot.statusSide}`) }
          : {},
      )
    : "";

  function personaLabel(persona: string): string {
    if (persona === "Custom") {
      return customPersona.name.trim() || $i18n("personas.custom");
    }
    return $i18n(`personas.${personaFor(persona).key}`);
  }

  function errorMessage(value: unknown): string {
    if (typeof value === "string") return value;
    if (value instanceof Error) return value.message;
    try {
      return JSON.stringify(value);
    } catch {
      return "Unexpected error";
    }
  }

  function playerStatus(color: ColorKey): string {
    if (!snapshot) return "waiting";
    if (snapshot.gameOver) {
      if (snapshot.winner === color) return "won";
      if (snapshot.winner) return "lost";
      return "draw";
    }
    if (thinking && color === snapshot.agentColor) return "thinking";
    if (snapshot.turn !== color) return "waiting";
    return "toMove";
  }

  function statusTextFor(game: GameSnapshot): string {
    return $i18n(
      `status.${game.statusKey}`,
      game.statusSide ? { side: $i18n(`side.${game.statusSide}`) } : {},
    );
  }

  function applySnapshot(next: GameSnapshot) {
    const previous = snapshot;
    snapshot = next;
    settings = next.settings;
    const colorChanged = next.humanColor !== humanColor;
    humanColor = next.humanColor;
    if (colorChanged) {
      saveHumanColor(next.humanColor);
    }
    applySettingsSideEffects(next.settings);
    thinking = next.thinking;

    if (next.lastMove && next.lastMove.ply !== lastSoundPly) {
      lastSoundPly = next.lastMove.ply;
      const kind = next.gameOver
        ? "end"
        : next.check
          ? "check"
          : next.lastMove.captured
            ? "capture"
            : "move";
      playSound(kind, next.settings.sound);
    }

    if (next.gameOver && previous && !previous.gameOver) {
      resultDismissed = false;
    }

    if (
      next.lastMove &&
      (!previous || next.lastMove.ply !== previous.lastMove?.ply)
    ) {
      announcement = $i18n("a11y.move", {
        side: $i18n(`side.${next.lastMove.color}`),
        san: next.lastMove.san,
      });
      const status = statusTextFor(next);
      if (status) {
        announcement += ` ${status}`;
      }
    } else if (next.gameOver && previous && !previous.gameOver) {
      announcement = statusTextFor(next);
    }
  }

  function applySettingsSideEffects(next: Settings) {
    const serialized = JSON.stringify(next);
    if (serialized === appliedSettings) return;
    appliedSettings = serialized;
    saveSettings(next);
    applyTheme(next.theme, next.customTheme);
    void applyPieceSet(next.pieceSet);
    applyFontScale(next.fontScale);
    applyMotion(next.animations);
    setLocale(next.uiLanguage);
  }

  async function startGame(color: ColorKey = humanColor): Promise<string | null> {
    loading = true;
    error = "";
    selected = null;
    promotion = null;
    resultDismissed = false;
    lastSoundPly = 0;
    drawNotice = null;
    thinking = false;
    humanColor = color;
    flipped = color === "black";
    saveHumanColor(color);

    const agentLanguageName = localeMeta(settings.agentLanguage).name;
    if (settings.agentLanguageName !== agentLanguageName) {
      settings = { ...settings, agentLanguageName };
      saveSettings(settings);
    }

    try {
      const next = await invoke<GameSnapshot>("new_game", {
        settings,
        humanColor: color,
      });
      applySnapshot(next);
      if (next.turn === next.agentColor && !next.gameOver) {
        await agentMove();
      }
      return null;
    } catch (value) {
      const message = errorMessage(value);
      error = message;
      return message;
    } finally {
      loading = false;
    }
  }

  async function agentMove() {
    if (!snapshot || thinking || snapshot.gameOver) return;
    thinking = true;
    error = "";
    try {
      const next = await invoke<GameSnapshot>("agent_move");
      applySnapshot(next);
    } catch (value) {
      error = errorMessage(value);
      try {
        const state = await invoke<GameSnapshot>("get_state");
        snapshot = state;
        thinking = state.thinking;
      } catch {
        // Keep the previous board if the state cannot be refreshed.
      }
    } finally {
      thinking = false;
    }
  }

  async function humanMove(uci: string) {
    if (!snapshot || thinking || snapshot.gameOver) return;
    selected = null;
    promotion = null;
    error = "";
    drawNotice = null;
    try {
      const next = await invoke<GameSnapshot>("make_move", { mv: uci });
      applySnapshot(next);
      if (
        next.settings.responseMode === "auto" &&
        next.turn === next.agentColor &&
        !next.gameOver
      ) {
        await agentMove();
      }
    } catch (value) {
      error = errorMessage(value);
    }
  }

  function onSquareClick(square: string) {
    if (
      !snapshot ||
      thinking ||
      snapshot.gameOver ||
      snapshot.turn !== snapshot.humanColor ||
      promotion
    ) {
      return;
    }

    const piece =
      snapshot.board.find((entry) => entry.square === square)?.piece ?? null;

    if (selected) {
      if (square === selected) {
        selected = null;
        return;
      }

      const candidates = snapshot.legalMoves.filter(
        (move) => move.from === selected && move.to === square,
      );
      if (candidates.length > 1 && candidates.some((move) => move.promotion)) {
        promotion = { from: selected, to: square, options: candidates };
        return;
      }
      if (candidates.length > 0) {
        void humanMove(candidates[0].uci);
        return;
      }
      if (piece && piece.color === snapshot.humanColor) {
        selected = square;
      } else {
        selected = null;
      }
      return;
    }

    if (piece && piece.color === snapshot.humanColor) {
      selected = square;
    }
  }

  function choosePromotion(uci: string) {
    void humanMove(uci);
  }

  async function takeback() {
    if (!snapshot || !canTakeback) return;
    error = "";
    try {
      const next = await invoke<GameSnapshot>("takeback");
      applySnapshot(next);
    } catch (value) {
      error = errorMessage(value);
    }
  }

  async function resign() {
    if (!snapshot || snapshot.gameOver) return;
    if (!window.confirm($i18n("app.confirmResign"))) return;
    error = "";
    try {
      const next = await invoke<GameSnapshot>("resign");
      applySnapshot(next);
    } catch (value) {
      error = errorMessage(value);
    }
  }

  async function offerDraw() {
    if (!snapshot || snapshot.gameOver || thinking || chatBusy || drawBusy) {
      return;
    }
    drawBusy = true;
    drawNotice = null;
    error = "";
    try {
      const outcome = await invoke<DrawOfferOutcome>("offer_draw");
      applySnapshot(outcome.snapshot);
      drawNotice = {
        accepted: outcome.accepted,
        comment: outcome.comment,
      };
    } catch (value) {
      error = errorMessage(value);
    } finally {
      drawBusy = false;
    }
  }

  async function refreshClock() {
    if (
      document.hidden ||
      clockPolling ||
      !snapshot?.clock ||
      snapshot.gameOver ||
      thinking ||
      chatBusy ||
      drawBusy
    ) {
      return;
    }
    clockPolling = true;
    try {
      const update = await invoke<ClockUpdate>("clock_tick");
      if (!snapshot || update.gameId !== snapshot.gameId) return;
      if (update.gameOver) {
        const state = await invoke<GameSnapshot>("get_state");
        applySnapshot(state);
      } else if (update.clock) {
        snapshot = { ...snapshot, clock: update.clock };
      }
    } catch {
      // Clock polling is best-effort; the next state refresh will catch up.
    } finally {
      clockPolling = false;
    }
  }

  async function sendChat(message: string): Promise<string | null> {
    if (!snapshot || chatBusy || thinking) {
      return "Wait for the opponent to finish before sending a message.";
    }
    chatBusy = true;
    error = "";
    try {
      const next = await invoke<GameSnapshot>("chat_send", { message });
      applySnapshot(next);
      return null;
    } catch (value) {
      const failure = errorMessage(value);
      error = failure;
      return failure;
    } finally {
      chatBusy = false;
    }
  }

  async function clearChat(): Promise<string | null> {
    if (!snapshot || chatBusy) return "Chat is busy.";
    try {
      const next = await invoke<GameSnapshot>("clear_chat");
      applySnapshot(next);
      return null;
    } catch (value) {
      const failure = errorMessage(value);
      error = failure;
      return failure;
    }
  }

  async function coachRequest(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<string | null> {
    if (!snapshot || coachBusy) return "The coach is busy.";
    coachBusy = true;
    error = "";
    try {
      const next = await invoke<GameSnapshot>(command, args);
      applySnapshot(next);
      return null;
    } catch (value) {
      const failure = errorMessage(value);
      error = failure;
      return failure;
    } finally {
      coachBusy = false;
    }
  }

  function coachExplain(): Promise<string | null> {
    return coachRequest("coach_explain");
  }

  function coachHint(level: number): Promise<string | null> {
    return coachRequest("coach_hint", { level });
  }

  function coachReview(): Promise<string | null> {
    return coachRequest("coach_review");
  }

  function coachClear(): Promise<string | null> {
    return coachRequest("coach_clear");
  }

  function handleBroadcastKeydown(event: KeyboardEvent) {
    if (broadcast && !promotion && event.key === "Escape") {
      broadcast = false;
    }
  }

  function newGame() {
    if (
      snapshot &&
      snapshot.history.length > 0 &&
      !snapshot.gameOver &&
      !window.confirm($i18n("app.confirmNewGame"))
    ) {
      return;
    }
    void startGame(humanColor);
  }

  async function refreshModels() {
    try {
      models = await invoke<string[]>("list_models");
    } catch {
      models = [];
    } finally {
      modelsLoaded = true;
    }
  }

  async function refreshGames() {
    try {
      games = await invoke<GameSummary[]>("list_games");
    } catch {
      games = [];
    }
  }

  async function refreshGameLibrary() {
    await refreshGames();
    try {
      pgnPreview = await invoke<string>("get_pgn");
    } catch {
      pgnPreview = "";
    }
  }

  async function openGames() {
    showSettings = false;
    await refreshGameLibrary();
    showGames = true;
  }

  async function loadSavedGame(id: string) {
    if (thinking || chatBusy || drawBusy) {
      error = "Wait for the opponent to finish before opening another game.";
      return;
    }
    try {
      const next = await invoke<GameSnapshot>("load_game", { id });
      applySnapshot(next);
      flipped = next.humanColor === "black";
      drawNotice = null;
      showGames = false;
      error = "";
      if (next.turn === next.agentColor && !next.gameOver) {
        await agentMove();
      }
    } catch (value) {
      error = errorMessage(value);
    }
  }

  async function newGameFromLibrary() {
    showGames = false;
    await startGame(humanColor);
  }

  async function deleteSavedGame(id: string) {
    try {
      await invoke("delete_game", { id });
      if (snapshot?.gameId === id) {
        showGames = false;
        await startGame(humanColor);
      } else {
        await refreshGames();
      }
    } catch (value) {
      error = errorMessage(value);
    }
  }

  async function renameSavedGame(id: string, name: string) {
    try {
      const summary = await invoke<GameSummary>("rename_game", { id, name });
      await refreshGames();
      if (snapshot && snapshot.gameId === id) {
        snapshot = { ...snapshot, gameName: summary.name };
      }
    } catch (value) {
      error = errorMessage(value);
    }
  }

  async function savePgnFile(): Promise<string | void> {
    try {
      return await invoke<string>("export_pgn", {
        name: snapshot?.gameName ?? "gambiit",
      });
    } catch (value) {
      error = errorMessage(value);
      throw value;
    }
  }

  async function importPgn(pgn: string): Promise<boolean> {
    if (thinking || chatBusy || drawBusy) {
      error = "Wait for the opponent to finish before importing a game.";
      return false;
    }
    try {
      const next = await invoke<GameSnapshot>("import_pgn", {
        pgn,
        settings,
      });
      applySnapshot(next);
      flipped = next.humanColor === "black";
      drawNotice = null;
      showGames = false;
      error = "";
      if (next.turn === next.agentColor && !next.gameOver) {
        await agentMove();
      }
      return true;
    } catch (value) {
      error = errorMessage(value);
      return false;
    }
  }

  function openSettings() {
    showGames = false;
    showSettings = true;
    if (!modelsLoaded) {
      void refreshModels();
    }
  }

  function cancelSettings() {
    applyTheme(settings.theme, settings.customTheme);
    void applyPieceSet(settings.pieceSet);
    applyFontScale(settings.fontScale);
    applyMotion(settings.animations);
    setLocale(settings.uiLanguage);
    showSettings = false;
  }

  async function applySettings(
    next: Settings,
    color: ColorKey,
    startNew: boolean,
  ): Promise<string | null> {
    const needsNewGame = startNew || !snapshot || color !== snapshot.humanColor;
    if (needsNewGame) {
      const previousSettings = settings;
      const previousColor = humanColor;
      const previousFlipped = flipped;
      settings = next;
      saveSettings(next);
      applyTheme(next.theme, next.customTheme);
      void applyPieceSet(next.pieceSet);
      applyFontScale(next.fontScale);
      applyMotion(next.animations);
      setLocale(next.uiLanguage);
      const message = await startGame(color);
      if (!message) {
        showSettings = false;
        return null;
      }
      settings = previousSettings;
      humanColor = previousColor;
      flipped = previousFlipped;
      saveSettings(previousSettings);
      saveHumanColor(previousColor);
      applyTheme(previousSettings.theme, previousSettings.customTheme);
      void applyPieceSet(previousSettings.pieceSet);
      applyFontScale(previousSettings.fontScale);
      setLocale(previousSettings.uiLanguage);
      return message;
    }
    try {
      const updated = await invoke<GameSnapshot>("update_settings", {
        settings: next,
      });
      applySnapshot(updated);
      return null;
    } catch (value) {
      const message = errorMessage(value);
      error = message;
      return message;
    }
  }

  onMount(() => {
    applyTheme(settings.theme, settings.customTheme);
    void applyPieceSet(settings.pieceSet);
    applyFontScale(settings.fontScale);
    applyMotion(settings.animations);
    setLocale(settings.uiLanguage);
    appliedSettings = JSON.stringify(settings);
    const timer = window.setInterval(() => void refreshClock(), 250);
    void (async () => {
      await refreshGames();
      if (games.length > 0) {
        await loadSavedGame(games[0].id);
      }
      if (!snapshot) {
        await startGame(humanColor);
      }
    })();
    return () => window.clearInterval(timer);
  });
</script>

<svelte:window on:keydown={handleBroadcastKeydown} />

{#if !snapshot}
  <div class="loading-screen">
    <div class="loading-mark">G</div>
    <div class="spinner large"></div>
    <p>{$i18n("app.settingTheStage")}</p>
  </div>
{:else}
  <div class="app" class:broadcast>
    <div class="sr-only" role="status" aria-live="polite" aria-atomic="true">
      {announcement}
    </div>
    <header class="topbar">
      <div class="brand">
        <div class="brand-mark">G</div>
        <div class="brand-copy">
          <h1>GAMBIIT</h1>
          <p>{$i18n("app.tagline")}</p>
        </div>
      </div>

      <div class="top-actions">
        <span class="status-pill" class:live={!snapshot.gameOver}>
          <i></i>
          {localizedStatus}
        </span>
        {#if broadcast}
          <button
            class="btn ghost broadcast-exit"
            type="button"
            on:click={() => (broadcast = false)}
          >
            {$i18n("app.exitBroadcast")}
          </button>
        {:else}
          <button
            class="icon-btn"
            type="button"
            title={$i18n("app.broadcast")}
            aria-label={$i18n("app.broadcast")}
            on:click={() => (broadcast = true)}
          >
            ◉
          </button>
          <button
            class="icon-btn"
            type="button"
            title={$i18n("games.title")}
            aria-label={$i18n("games.title")}
            on:click={openGames}
          >
            ▤
          </button>
          <button
            class="icon-btn"
            type="button"
            title={$i18n("app.flipBoard")}
            aria-label={$i18n("app.flipBoard")}
            on:click={() => (flipped = !flipped)}
          >
            ⇅
          </button>
          <button
            class="icon-btn"
            type="button"
            title={$i18n("app.settings")}
            aria-label={$i18n("app.settings")}
            on:click={openSettings}
          >
            ⚙
          </button>
        {/if}
      </div>
    </header>

    <main class="main">
      <section class="stage">
        <PlayerBar
          name={personaLabel(agentName)}
          subtitle={`${agentModel} · ${$i18n(`side.${snapshot.agentColor}`)}`}
          color={snapshot.agentColor}
          isAgent
          thinking={thinking}
          active={snapshot.turn === snapshot.agentColor && !snapshot.gameOver}
          statusKey={playerStatus(snapshot.agentColor)}
          captured={agentCaptured}
          material={agentMaterial}
          avatar={agentPersona.avatar}
          accent={agentPersona.accent}
          clockMs={agentClock}
          clockActive={agentClockRunning}
          clockStopped={snapshot.gameOver}
          broadcast={broadcast}
        />

        <div class="board-shell">
          <Board
            board={snapshot.board}
            legalMoves={snapshot.legalMoves}
            lastMove={snapshot.lastMove}
            checkSquare={snapshot.checkSquare}
            {selected}
            {flipped}
            interactive={boardInteractive}
            animations={snapshot.settings.animations}
            {promotion}
            {promotionColor}
            onSquareClick={onSquareClick}
            onPromotion={choosePromotion}
          />

          {#if thinking}
            <div class="thinking-ribbon">
              <span class="spinner"></span>
              {$i18n("app.thinking", { name: personaLabel(agentName) })}
            </div>
          {:else if drawBusy}
            <div class="thinking-ribbon">
              <span class="spinner"></span>
              {$i18n("app.consideringDraw")}
            </div>
          {/if}

          {#if snapshot.gameOver && !resultDismissed}
            <div class="result-overlay">
              <div class="result-card">
                <p class="eyebrow">{$i18n("app.finalCurtain")}</p>
                <h2>{localizedStatus}</h2>
                {#if snapshot.winner}
                  <p class="winner">
                    {$i18n("app.takesTheStage", {
                      side: $i18n(`side.${snapshot.winner}`),
                    })}
                  </p>
                {/if}
                <div class="result-actions">
                  <button
                    class="btn primary"
                    type="button"
                    on:click={() =>
                      snapshot && startGame(snapshot.humanColor)}
                  >
                    {$i18n("app.rematch")}
                  </button>
                  <button
                    class="btn ghost"
                    type="button"
                    on:click={() => (resultDismissed = true)}
                  >
                    {$i18n("app.reviewBoard")}
                  </button>
                </div>
              </div>
            </div>
          {/if}
        </div>

        <PlayerBar
          name={$i18n("app.you")}
          subtitle={`${$i18n(`side.${snapshot.humanColor}`)} · ${$i18n("app.localPlayer")}`}
          color={snapshot.humanColor}
          active={snapshot.turn === snapshot.humanColor && !snapshot.gameOver}
          statusKey={playerStatus(snapshot.humanColor)}
          captured={humanCaptured}
          material={humanMaterial}
          clockMs={humanClock}
          clockActive={humanClockRunning}
          clockStopped={snapshot.gameOver}
          broadcast={broadcast}
        />

        <div class="board-actions">
          <button
            class="btn ghost"
            type="button"
            disabled={chatBusy || coachBusy || drawBusy}
            on:click={newGame}
          >
            {$i18n("app.newGame")}
          </button>
          <button
            class="btn ghost"
            type="button"
            disabled={!canTakeback}
            title={$i18n("app.onlyOwnMove")}
            on:click={takeback}
          >
            {$i18n("app.takeBack")}
          </button>
          {#if snapshot.settings.drawOffersEnabled}
            <button
              class="btn ghost"
              type="button"
              disabled={snapshot.gameOver || thinking || chatBusy || coachBusy || drawBusy}
              on:click={offerDraw}
            >
              {drawBusy ? $i18n("app.offeringDraw") : $i18n("app.offerDraw")}
            </button>
          {/if}
          <button
            class="btn ghost danger"
            type="button"
            disabled={snapshot.gameOver || chatBusy || coachBusy || drawBusy}
            on:click={resign}
          >
            {$i18n("app.resign")}
          </button>
          {#if canAgentMove && snapshot.settings.responseMode === "button"}
            <button class="btn primary agent-move" type="button" on:click={agentMove}>
              {$i18n("app.agentMove")}
            </button>
          {/if}
        </div>

        {#if drawNotice}
          <div class="draw-notice" class:accepted={drawNotice.accepted}>
            <div>
              <strong>
                {drawNotice.accepted
                  ? $i18n("app.drawAccepted")
                  : $i18n("app.drawDeclined")}
              </strong>
              {#if drawNotice.comment}<span>{drawNotice.comment}</span>{/if}
            </div>
            <button
              type="button"
              aria-label={$i18n("app.dismissNotice")}
              on:click={() => (drawNotice = null)}
            >
              ✕
            </button>
          </div>
        {/if}
      </section>

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
              persona={personaLabel(agentName)}
              model={agentModel}
              avatar={agentPersona.avatar}
              accent={agentPersona.accent}
            />
          {/if}
          {#if snapshot.settings.chatEnabled}
            <div class="chat-pane" class:active={activeTab === "chat"}>
              <Chat
                messages={snapshot.chat}
                personaName={personaLabel(agentName)}
                avatar={agentPersona.avatar}
                accent={agentPersona.accent}
                busy={chatBusy}
                disabled={thinking || coachBusy || drawBusy}
                onSend={sendChat}
                onClear={clearChat}
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
                onExplain={coachExplain}
                onHint={coachHint}
                onReview={coachReview}
                onClear={coachClear}
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
                <button class="btn small" type="button" on:click={agentMove}>
                  {$i18n("app.retryAgent")}
                </button>
              {/if}
            </div>
          {:else}
            <div class="footer-line">
              <span class="footer-dot"></span>
              <span class="footer-model">{agentModel}</span>
            </div>
            <div class="footer-persona">{personaLabel(agentName)}</div>
          {/if}
        </footer>
      </aside>
    </main>

    {#if broadcast}
      <div class="broadcast-strip">
        <BroadcastTicker
          cards={snapshot.commentary}
          history={snapshot.history}
          persona={personaLabel(agentName)}
          model={agentModel}
          avatar={agentPersona.avatar}
          accent={agentPersona.accent}
          thinking={thinking}
        />
      </div>
    {/if}

    {#if error}
      <div class="error-banner" role="alert">
        <span>{error}</span>
        <button
          type="button"
          aria-label={$i18n("app.dismissError")}
          on:click={() => (error = "")}
        >
          ✕
        </button>
      </div>
    {/if}

    {#if showSettings}
      <SettingsPanel
        {settings}
        {models}
        initialColor={snapshot.humanColor}
        onApply={applySettings}
        onClose={cancelSettings}
        onRefreshModels={refreshModels}
      />
    {/if}

    {#if showGames}
      <GameLibrary
        {games}
        currentId={snapshot.gameId}
        pgn={pgnPreview}
        onOpen={loadSavedGame}
        onNew={newGameFromLibrary}
        onDelete={deleteSavedGame}
        onRename={renameSavedGame}
        onRefresh={() => void refreshGameLibrary()}
        onSavePgn={savePgnFile}
        onImportPgn={importPgn}
        onClose={() => (showGames = false)}
      />
    {/if}
  </div>
{/if}

<style>
  .app {
    width: min(1480px, 100%);
    min-height: 100vh;
    margin: 0 auto;
    padding: 16px 22px 24px;
  }

  .app.broadcast {
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    padding: 10px 16px 14px;
  }

  .app.broadcast .topbar {
    margin-bottom: 10px;
  }

  .app.broadcast .brand-copy p {
    display: none;
  }

  .app.broadcast .main {
    display: flex;
    flex: 1;
    align-items: flex-start;
    justify-content: center;
  }

  .app.broadcast .sidebar,
  .app.broadcast .board-actions {
    display: none;
  }

  .app.broadcast .stage {
    max-width: max(320px, min(1100px, calc(100vh - 345px)));
    gap: 10px;
  }

  .broadcast-strip {
    flex: 0 0 auto;
    margin-top: 10px;
  }

  .app.broadcast .broadcast-exit {
    padding: 7px 12px;
    border-color: color-mix(in srgb, var(--accent) 40%, var(--border));
    color: var(--text);
  }

  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    margin-bottom: 14px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
  }

  .brand-mark {
    display: grid;
    width: 42px;
    height: 42px;
    place-items: center;
    border: 1px solid var(--accent);
    border-radius: 10px;
    background:
      radial-gradient(circle at 30% 20%, var(--accent-soft), transparent 65%),
      var(--panel);
    color: var(--accent);
    font-family: var(--font-display);
    font-size: calc(25px * var(--font-scale));
    font-weight: 700;
    line-height: 1;
    box-shadow: 0 0 0 1px rgba(0, 0, 0, 0.35);
  }

  .brand-copy h1 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(21px * var(--font-scale));
    font-weight: 600;
    letter-spacing: 0.18em;
    line-height: 1;
  }

  .brand-copy p {
    margin: 5px 0 0;
    color: var(--faint);
    font-family: var(--font-display);
    font-size: calc(11px * var(--font-scale));
    font-style: italic;
    letter-spacing: 0.02em;
  }

  .top-actions {
    display: flex;
    align-items: center;
    gap: 9px;
  }

  .status-pill {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 11px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--panel);
    color: var(--muted);
    font-size: calc(11px * var(--font-scale));
    white-space: nowrap;
  }

  .status-pill i {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--faint);
  }

  .status-pill.live i {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }

  .icon-btn {
    display: grid;
    width: 36px;
    height: 36px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 9px;
    background: var(--panel);
    color: var(--muted);
    font-size: calc(16px * var(--font-scale));
    cursor: pointer;
    transition:
      border-color 120ms ease,
      color 120ms ease,
      background 120ms ease;
  }

  .icon-btn:hover {
    border-color: var(--accent);
    background: var(--panel-2);
    color: var(--text);
  }

  .main {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 400px;
    align-items: start;
    gap: 22px;
  }

  .stage {
    display: flex;
    width: 100%;
    max-width: max(
      320px,
      min(780px, calc(100vh - 300px - (var(--font-scale) - 1) * 160px))
    );
    margin: 0 auto;
    flex-direction: column;
    gap: 8px;
  }

  .board-shell {
    position: relative;
    width: 100%;
  }

  .thinking-ribbon {
    position: absolute;
    top: 14px;
    left: 50%;
    z-index: 7;
    display: inline-flex;
    align-items: center;
    gap: 7px;
    padding: 7px 12px;
    border: 1px solid color-mix(in srgb, var(--accent) 50%, var(--border));
    border-radius: 999px;
    background: color-mix(in srgb, var(--panel) 88%, transparent);
    color: var(--accent-strong);
    font-family: var(--font-mono);
    font-size: calc(10px * var(--font-scale));
    pointer-events: none;
    transform: translateX(-50%);
    backdrop-filter: blur(8px);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.24);
  }

  .board-actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: 2px;
  }

  .draw-notice {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    padding: 8px 10px;
    border: 1px solid color-mix(in srgb, var(--danger) 42%, var(--border));
    border-radius: 9px;
    background: color-mix(in srgb, var(--panel) 92%, var(--danger));
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.5;
  }

  .draw-notice.accepted {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
    background: color-mix(in srgb, var(--panel) 92%, var(--success));
  }

  .draw-notice div {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .draw-notice strong {
    color: var(--text);
    font-size: calc(10px * var(--font-scale));
  }

  .draw-notice button {
    flex: 0 0 auto;
    padding: 0 3px;
    border: 0;
    background: transparent;
    color: var(--faint);
    cursor: pointer;
  }

  .draw-notice button:hover {
    color: var(--text);
  }

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

  .btn.primary {
    border-color: var(--accent);
    background: var(--accent);
    color: #17120b;
  }

  .btn.primary:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .btn.danger:hover:not(:disabled) {
    border-color: var(--danger);
    color: var(--danger);
  }

  .btn.agent-move {
    margin-left: auto;
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

  .result-overlay {
    position: absolute;
    inset: 0;
    z-index: 20;
    display: grid;
    padding: 24px;
    place-items: center;
    border-radius: 14px;
    background: rgba(8, 6, 4, 0.68);
    backdrop-filter: blur(4px);
  }

  .result-card {
    width: min(420px, 100%);
    padding: 26px;
    border: 1px solid var(--border);
    border-radius: 16px;
    background: var(--panel);
    box-shadow: var(--shadow);
    text-align: center;
  }

  .eyebrow {
    margin: 0 0 7px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(9px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.18em;
    text-transform: uppercase;
  }

  .result-card h2 {
    margin: 0;
    font-family: var(--font-display);
    font-size: calc(28px * var(--font-scale));
    font-weight: 500;
    line-height: 1.15;
  }

  .winner {
    margin: 8px 0 0;
    color: var(--muted);
    font-size: calc(12px * var(--font-scale));
  }

  .result-actions {
    display: flex;
    justify-content: center;
    gap: 9px;
    margin-top: 20px;
  }

  .error-banner {
    position: fixed;
    right: 20px;
    bottom: 20px;
    z-index: 50;
    display: flex;
    max-width: min(520px, calc(100vw - 40px));
    align-items: flex-start;
    gap: 12px;
    padding: 11px 12px 11px 14px;
    border: 1px solid color-mix(in srgb, var(--danger) 55%, var(--border));
    border-radius: 9px;
    background: color-mix(in srgb, var(--panel) 92%, var(--danger));
    color: var(--text);
    font-size: calc(11px * var(--font-scale));
    line-height: 1.45;
    box-shadow: 0 14px 40px rgba(0, 0, 0, 0.35);
  }

  .error-banner span {
    flex: 1;
  }

  .error-banner button {
    border: 0;
    background: transparent;
    color: var(--muted);
    cursor: pointer;
  }

  .loading-screen {
    display: flex;
    min-height: 100vh;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 13px;
    color: var(--muted);
    font-size: calc(12px * var(--font-scale));
  }

  .loading-mark {
    display: grid;
    width: 70px;
    height: 70px;
    place-items: center;
    border: 1px solid var(--accent);
    border-radius: 16px;
    background: var(--panel);
    color: var(--accent);
    font-family: var(--font-display);
    font-size: calc(42px * var(--font-scale));
    font-weight: 700;
  }

  .spinner {
    width: 10px;
    height: 10px;
    border: 1.5px solid var(--accent-soft);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  .spinner.large {
    width: 22px;
    height: 22px;
    border-width: 2px;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (max-width: 1120px) {
    .main {
      grid-template-columns: 1fr;
    }
    .sidebar {
      position: static;
      height: 560px;
      min-height: 0;
    }
    .stage {
      max-width: 720px;
    }
  }

  @media (max-width: 700px) {
    .app {
      padding: 12px;
    }
    .topbar {
      align-items: flex-start;
      flex-direction: column;
    }
    .top-actions {
      width: 100%;
      justify-content: space-between;
    }
    .status-pill {
      flex: 1;
      justify-content: center;
    }
    .sidebar {
      height: 520px;
    }
    .board-actions .btn.agent-move {
      margin-left: 0;
    }
  }
</style>
