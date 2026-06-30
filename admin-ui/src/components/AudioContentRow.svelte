<script lang="ts">
  import { ArrowRight, FileAudio, Mic, Trash2, Upload, WandSparkles } from "@lucide/svelte";

  export let item: AudioRowSource;
  export let displayTitle: string | null = null;
  export let detail: string;
  export let reason: string | null = null;
  export let busy = false;
  export let playable = false;
  export let showTrash = false;
  export let showOpen = false;
  export let onPreview: (item: AudioRowSource) => void | Promise<void> = () => {};
  export let onTrash: (item: AudioRowSource) => void = () => {};
  export let onOpen: (item: AudioRowSource) => void = () => {};

  function handleKeydown(event: KeyboardEvent) {
    if (!playable) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      void onPreview(item);
    }
  }
</script>

<script lang="ts" context="module">
  export type AudioRowSource = {
    id: string;
    title: string;
    source: string;
    preview_url?: string | null;
  };
</script>

{#if playable}
  <div
    class="ci ci-playable"
    role="button"
    tabindex="0"
    aria-label={`Play ${item.title}`}
    on:click={() => onPreview(item)}
    on:keydown={handleKeydown}
  >
    <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : item.source === 'recorded' ? 'ci-recorded' : 'ci-default'}">
      {#if item.source === "generated"}
        <WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else if item.source === "uploaded"}
        <Upload size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else if item.source === "recorded"}
        <Mic size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else}
        <FileAudio size={16} strokeWidth={1.5} aria-hidden="true" />
      {/if}
    </div>
    <div class="ci-meta">
      <strong class="ci-name" title={item.title}>{displayTitle ?? item.title}</strong>
      <p class="ci-detail">{detail}</p>
      {#if reason}
        <p class="inventory-reason">{reason}</p>
      {/if}
    </div>
    {#if showTrash || showOpen}
      <div class="ci-actions">
        {#if showTrash}
          <button type="button" class="cia del" on:click|stopPropagation={() => onTrash(item)} aria-label="Move to trash" disabled={busy}>
            <Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />
          </button>
        {/if}
        {#if showOpen}
          <button type="button" class="cia" on:click|stopPropagation={() => onOpen(item)} aria-label="Open button">
            <ArrowRight size={16} strokeWidth={1.5} aria-hidden="true" />
          </button>
        {/if}
      </div>
    {/if}
  </div>
{:else}
  <div class="ci" role="listitem">
    <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : item.source === 'recorded' ? 'ci-recorded' : 'ci-default'}">
      {#if item.source === "generated"}
        <WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else if item.source === "uploaded"}
        <Upload size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else if item.source === "recorded"}
        <Mic size={16} strokeWidth={1.5} aria-hidden="true" />
      {:else}
        <FileAudio size={16} strokeWidth={1.5} aria-hidden="true" />
      {/if}
    </div>
    <div class="ci-meta">
      <strong class="ci-name" title={item.title}>{displayTitle ?? item.title}</strong>
      <p class="ci-detail">{detail}</p>
      {#if reason}
        <p class="inventory-reason">{reason}</p>
      {/if}
    </div>
    {#if showOpen}
      <div class="ci-actions">
        <button type="button" class="cia" on:click={() => onOpen(item)} aria-label="Open button">
          <ArrowRight size={16} strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
    {/if}
  </div>
{/if}
