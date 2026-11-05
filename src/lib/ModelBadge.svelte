<script lang="ts">
  export let model = "";
  export let size: "sm" | "md" | "lg" = "md";

  const LABELS: Record<string, string> = {
    "opencode-go": "OC",
    opencode: "OC",
    openai: "OA",
    anthropic: "AN",
    google: "G",
    gemini: "G",
    "github-copilot": "GH",
    xai: "X",
    deepseek: "DS",
    mistral: "M",
    groq: "GQ",
    openrouter: "OR",
    ollama: "OL",
    localai: "LA",
  };

  const COLORS: Record<string, string> = {
    "opencode-go": "#5468ff",
    opencode: "#5468ff",
    openai: "#0f9f7f",
    anthropic: "#c96442",
    google: "#3b74d9",
    gemini: "#3b74d9",
    "github-copilot": "#7c5cff",
    xai: "#3d4b5f",
    deepseek: "#4164e8",
    mistral: "#e56a2b",
    groq: "#d94a34",
    openrouter: "#5b56d6",
    ollama: "#5f7085",
    localai: "#148f87",
  };

  $: provider = model.includes("/")
    ? model.split("/")[0].trim().toLowerCase()
    : "custom";
  $: label = LABELS[provider] ?? initials(provider);
  $: color = COLORS[provider] ?? "#c9a227";

  function initials(value: string): string {
    if (!value || value === "custom") return "AI";
    const parts = value.split(/[-_]/).filter(Boolean);
    if (parts.length > 1) {
      return (parts[0][0] + parts[1][0]).toUpperCase();
    }
    return value.slice(0, 2).toUpperCase();
  }
</script>

<span
  class="badge {size}"
  style={`--model-color:${color}`}
  aria-hidden="true"
>
  {label}
</span>

<style>
  .badge {
    display: inline-grid;
    flex: 0 0 auto;
    place-items: center;
    border: 1px solid color-mix(in srgb, var(--model-color) 70%, white);
    border-radius: 12px;
    background:
      radial-gradient(
        circle at 30% 20%,
        rgba(255, 255, 255, 0.32),
        transparent 60%
      ),
      var(--model-color);
    box-shadow:
      inset 0 0 0 1px rgba(255, 255, 255, 0.16),
      0 5px 14px color-mix(in srgb, var(--model-color) 25%, transparent);
    color: #fff;
    font-family: var(--font-mono);
    font-size: calc(11px * var(--font-scale));
    font-weight: 800;
    letter-spacing: -0.02em;
    line-height: 1;
    text-shadow: 0 1px 1px rgba(0, 0, 0, 0.28);
  }

  .badge.sm {
    width: 26px;
    height: 26px;
    border-radius: 8px;
    font-size: calc(8px * var(--font-scale));
  }

  .badge.md {
    width: 42px;
    height: 42px;
  }

  .badge.lg {
    width: 52px;
    height: 52px;
    border-radius: 14px;
    font-size: calc(13px * var(--font-scale));
  }
</style>
