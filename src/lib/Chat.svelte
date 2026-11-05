<script lang="ts">
  import { afterUpdate } from "svelte";
  import { i18n } from "./i18n";
  import type { ChatMessage } from "./types";

  export let messages: ChatMessage[] = [];
  export let personaName = "";
  export let avatar = "";
  export let accent = "#c9a227";
  export let busy = false;
  export let disabled = false;
  export let onSend: (message: string) => Promise<string | null> = async () => null;
  export let onClear: () => Promise<string | null> = async () => null;

  let input = "";
  let error = "";
  let sending = false;
  let clearing = false;
  let list: HTMLDivElement | null = null;

  afterUpdate(() => {
    if (list) {
      list.scrollTop = list.scrollHeight;
    }
  });

  function formatTime(seconds: number): string {
    if (!seconds) return "";
    return new Date(seconds * 1000).toLocaleTimeString([], {
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  async function send() {
    const text = input.trim();
    if (!text || busy || sending || disabled) return;
    sending = true;
    error = "";
    const failure = await onSend(text);
    if (failure) {
      error = failure;
    } else {
      input = "";
    }
    sending = false;
  }

  function onKeydown(event: KeyboardEvent) {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      void send();
    }
  }

  function useSuggestion(text: string) {
    input = text;
  }

  async function clear() {
    if (clearing || busy) return;
    if (!window.confirm($i18n("chat.clearConfirm"))) return;
    clearing = true;
    error = "";
    const failure = await onClear();
    if (failure) {
      error = failure;
    }
    clearing = false;
  }

  const canSend = () => input.trim().length > 0 && !busy && !sending && !disabled;
</script>

<div class="chat">
  <header class="chat-head">
    <div class="identity">
      <span class="avatar" style={`--persona-accent:${accent}`}>
        {#if avatar}
          <img src={avatar} alt="" draggable="false" />
        {:else}
          <span>?</span>
        {/if}
      </span>
      <div>
        <p class="eyebrow">{$i18n("chat.eyebrow")}</p>
        <h3>{personaName}</h3>
        <span class="online">
          <i></i>
          {busy ? $i18n("chat.typing") : $i18n("chat.available")}
        </span>
      </div>
    </div>
    {#if messages.length > 0}
      <button
        class="clear"
        type="button"
        disabled={busy || clearing}
        title={$i18n("chat.clear")}
        on:click={clear}
      >
        {$i18n("chat.clear")}
      </button>
    {/if}
  </header>

  <div class="chat-list" bind:this={list}>
    {#if messages.length === 0}
      <div class="empty">
        <div class="empty-mark">“</div>
        <strong>{$i18n("chat.emptyTitle")}</strong>
        <p>{$i18n("chat.emptyBody")}</p>
        <div class="suggestions">
          <button type="button" on:click={() => useSuggestion($i18n("chat.suggestion1"))}>
            {$i18n("chat.suggestion1")}
          </button>
          <button type="button" on:click={() => useSuggestion($i18n("chat.suggestion2"))}>
            {$i18n("chat.suggestion2")}
          </button>
          <button type="button" on:click={() => useSuggestion($i18n("chat.suggestion3"))}>
            {$i18n("chat.suggestion3")}
          </button>
        </div>
      </div>
    {:else}
      {#each messages as message, index (`${message.at}-${index}`)}
        {#if message.role === "system"}
          <p class="system">{message.text}</p>
        {:else}
          <article class="bubble" class:user={message.role === "user"}>
            {#if message.role === "agent"}
              <span class="mini-avatar" style={`--persona-accent:${accent}`}>
                {#if avatar}
                  <img src={avatar} alt="" draggable="false" />
                {:else}
                  <span>{personaName.slice(0, 1) || "?"}</span>
                {/if}
              </span>
            {/if}
            <div class="bubble-body">
              <p>{message.text}</p>
              <span class="stamp">
                {message.role === "user"
                  ? $i18n("chat.you")
                  : personaName}
                · {formatTime(message.at)}
              </span>
            </div>
          </article>
        {/if}
      {/each}
      {#if busy}
        <div class="bubble typing">
          <span class="mini-avatar" style={`--persona-accent:${accent}`}>
            {#if avatar}<img src={avatar} alt="" draggable="false" />{/if}
          </span>
          <div class="dots" aria-label={$i18n("chat.typing")}>
            <i></i><i></i><i></i>
          </div>
        </div>
      {/if}
    {/if}
    {#if error}
      <p class="error">{error}</p>
    {/if}
  </div>

  <footer class="composer">
    <textarea
      bind:value={input}
      rows="2"
      maxlength="2000"
      placeholder={busy ? $i18n("chat.typing") : $i18n("chat.placeholder")}
      disabled={busy || disabled}
      on:keydown={onKeydown}
    ></textarea>
    <div class="composer-foot">
      <span>{$i18n("chat.hint")}</span>
      <button
        class="send"
        type="button"
        disabled={!canSend()}
        on:click={send}
      >
        {busy || sending ? $i18n("chat.sending") : $i18n("chat.send")}
      </button>
    </div>
  </footer>
</div>

<style>
  .chat {
    display: flex;
    height: 100%;
    flex-direction: column;
    overflow: hidden;
  }

  .chat-head {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 10px;
    padding: 14px 14px 11px;
    border-bottom: 1px solid var(--border);
    background: linear-gradient(180deg, var(--accent-soft), transparent);
  }

  .identity {
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
  }

  .avatar,
  .mini-avatar {
    display: grid;
    flex: 0 0 auto;
    place-items: center;
    overflow: hidden;
    border: 1px solid color-mix(in srgb, var(--persona-accent) 55%, var(--border));
    border-radius: 50%;
    background: var(--panel-2);
  }

  .avatar {
    width: 42px;
    height: 42px;
    box-shadow: 0 0 0 3px var(--accent-soft);
  }

  .avatar img,
  .mini-avatar img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .mini-avatar {
    width: 26px;
    height: 26px;
    font-family: var(--font-display);
    font-size: calc(12px * var(--font-scale));
  }

  .eyebrow {
    margin: 0 0 2px;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    font-weight: 700;
    letter-spacing: 0.16em;
    text-transform: uppercase;
  }

  h3 {
    margin: 0;
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(20px * var(--font-scale));
    font-weight: 500;
    line-height: 1.1;
  }

  .online {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    margin-top: 2px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
  }

  .online i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--success);
    box-shadow: 0 0 6px var(--success);
  }

  .clear {
    padding: 4px 7px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--panel);
    color: var(--faint);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .clear:hover:not(:disabled) {
    border-color: var(--accent);
    color: var(--text);
  }

  .chat-list {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 8px;
    overflow-y: auto;
    padding: 13px 12px;
    scrollbar-width: thin;
  }

  .empty {
    display: flex;
    height: 100%;
    min-height: 260px;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 6px;
    padding: 20px 14px;
    text-align: center;
  }

  .empty-mark {
    display: grid;
    width: 44px;
    height: 44px;
    margin-bottom: 3px;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: 50%;
    color: var(--accent);
    font-family: var(--font-display);
    font-size: calc(30px * var(--font-scale));
    line-height: 0;
  }

  .empty strong {
    color: var(--text);
    font-family: var(--font-display);
    font-size: calc(18px * var(--font-scale));
    font-weight: 500;
  }

  .empty p {
    max-width: 260px;
    margin: 0;
    color: var(--muted);
    font-size: calc(10px * var(--font-scale));
    line-height: 1.55;
  }

  .suggestions {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: 5px;
    margin-top: 7px;
  }

  .suggestions button {
    padding: 4px 8px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--panel-2);
    color: var(--muted);
    font-size: calc(9px * var(--font-scale));
    cursor: pointer;
  }

  .suggestions button:hover {
    border-color: var(--accent);
    color: var(--text);
  }

  .bubble {
    display: flex;
    align-items: flex-end;
    gap: 6px;
    max-width: 92%;
  }

  .bubble.user {
    align-self: flex-end;
    flex-direction: row-reverse;
  }

  .bubble-body {
    padding: 7px 9px;
    border: 1px solid var(--border);
    border-radius: 10px 10px 10px 3px;
    background: var(--panel-2);
  }

  .bubble.user .bubble-body {
    border-color: color-mix(in srgb, var(--accent) 48%, var(--border));
    border-radius: 10px 10px 3px 10px;
    background: var(--accent-soft);
  }

  .bubble-body p {
    margin: 0;
    color: var(--text);
    font-size: calc(11px * var(--font-scale));
    line-height: 1.5;
    overflow-wrap: anywhere;
    white-space: pre-wrap;
  }

  .stamp {
    display: block;
    margin-top: 3px;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
  }

  .system {
    align-self: center;
    max-width: 90%;
    margin: 2px 0;
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(8px * var(--font-scale));
    line-height: 1.5;
    text-align: center;
  }

  .typing {
    align-items: center;
  }

  .dots {
    display: inline-flex;
    gap: 3px;
    padding: 9px 11px;
    border: 1px solid var(--border);
    border-radius: 10px 10px 10px 3px;
    background: var(--panel-2);
  }

  .dots i {
    width: 5px;
    height: 5px;
    border-radius: 50%;
    background: var(--muted);
    animation: blink 1.2s ease-in-out infinite;
  }

  .dots i:nth-child(2) {
    animation-delay: 0.18s;
  }

  .dots i:nth-child(3) {
    animation-delay: 0.36s;
  }

  @keyframes blink {
    0%,
    70%,
    100% {
      opacity: 0.25;
      transform: translateY(0);
    }
    35% {
      opacity: 1;
      transform: translateY(-2px);
    }
  }

  .error {
    align-self: stretch;
    padding: 6px 8px;
    margin: 0;
    border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
    border-radius: 6px;
    color: var(--danger);
    font-size: calc(9px * var(--font-scale));
    line-height: 1.45;
  }

  .composer {
    flex: 0 0 auto;
    padding: 9px 12px 12px;
    border-top: 1px solid var(--border);
    background: var(--panel);
  }

  textarea {
    width: 100%;
    min-height: 54px;
    max-height: 120px;
    padding: 7px 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--panel-2);
    color: var(--text);
    font-family: var(--font-ui);
    font-size: calc(11px * var(--font-scale));
    line-height: 1.45;
    resize: vertical;
  }

  textarea:disabled {
    opacity: 0.55;
  }

  .composer-foot {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    margin-top: 5px;
  }

  .composer-foot span {
    color: var(--faint);
    font-family: var(--font-mono);
    font-size: calc(7px * var(--font-scale));
  }

  .send {
    padding: 6px 11px;
    border: 1px solid var(--accent);
    border-radius: 7px;
    background: var(--accent);
    color: #17120b;
    font-size: calc(10px * var(--font-scale));
    font-weight: 700;
    cursor: pointer;
  }

  .send:hover:not(:disabled) {
    background: var(--accent-strong);
  }

  .send:disabled {
    opacity: 0.42;
    cursor: not-allowed;
  }
</style>
