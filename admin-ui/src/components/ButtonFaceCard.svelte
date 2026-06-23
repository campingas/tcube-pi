<script lang="ts">
  import { contentTypeLabel, modeLabel } from "../button-mode";
  import type { ButtonConfig } from "../types";

  export let button: ButtonConfig;
  export let selected = false;
  export let activeCount = 0;
  export let disabled = false;
  export let choose: (id: number) => void;

  $: incomplete = Boolean(button.contentType) && activeCount === 0;
</script>

<article class:selected class:incomplete class="button-face-card">
  <button type="button" class="face-selector" on:click={() => choose(button.id)}>
    <span class="face-number">0{button.id}</span>
    <span class="face-title">Face {button.id}</span>
  </button>
  <div class="face-meta">
    <span>{modeLabel(button.mode)}</span>
    <strong>{button.mode === "language" ? button.language : contentTypeLabel(button.contentType)}</strong>
  </div>
  <div class="face-footer">
    <span class:warning={incomplete}>{button.contentType ? `${activeCount} active` : "No playlist"}</span>
    <span>{incomplete ? "Needs content" : selected ? "Open" : "Ready"}</span>
  </div>
  <button type="button" class="neo-button secondary full" on:click={() => choose(button.id)} disabled={disabled}>
    Configure
  </button>
</article>

