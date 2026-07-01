<script lang="ts">
  import { Check, Trash2, WandSparkles } from "@lucide/svelte";
  import type { InactiveContentItem } from "../api";
  import { sourceLabel, trimAudioTitle } from "../view-utils";

  export let items: InactiveContentItem[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let busy = false;
  export let activate: (id: string) => void | Promise<void>;
  export let trash: (id: string) => void | Promise<void>;
  export let preview: (item: InactiveContentItem) => void | Promise<void> = () => {};

  function handlePreviewKeydown(event: KeyboardEvent, item: InactiveContentItem) {
    if (!item.preview_url) return;
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      void preview(item);
    }
  }
</script>

<section class="content-surface">
  {#if loading}
    <p class="muted">Loading drafts...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    <div class="content-list">
      {#each items as item}
        {#if item.preview_url}
          <div
            class="ci ci-playable"
            role="button"
            tabindex="0"
            aria-label={`Play draft ${item.title}`}
            on:click={() => preview(item)}
            on:keydown={(event) => handlePreviewKeydown(event, item)}
          >
            <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : item.source === 'recorded' ? 'ci-recorded' : 'ci-default'}">
              {#if item.source === "generated"}
                <WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />
              {:else}
                <Check size={16} strokeWidth={1.5} aria-hidden="true" />
              {/if}
            </div>
            <div class="ci-meta">
              <strong class="ci-name" title={item.title}>{trimAudioTitle(item.title)}</strong>
              <p class="ci-detail">{item.text || sourceLabel(item.source)} · tap to preview</p>
            </div>
            <div class="ci-actions">
              <button type="button" class="cia ok" on:click|stopPropagation={() => activate(item.id)} aria-label="Activate draft" disabled={busy}>
                <Check size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
              <button type="button" class="cia del" on:click|stopPropagation={() => trash(item.id)} aria-label="Move draft to trash" disabled={busy}>
                <Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
            </div>
          </div>
        {:else}
          <div class="ci" role="listitem">
            <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : item.source === 'recorded' ? 'ci-recorded' : 'ci-default'}">
              {#if item.source === "generated"}
                <WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />
              {:else}
                <Check size={16} strokeWidth={1.5} aria-hidden="true" />
              {/if}
            </div>
            <div class="ci-meta">
              <strong class="ci-name" title={item.title}>{trimAudioTitle(item.title)}</strong>
              <p class="ci-detail">{item.text || sourceLabel(item.source)}</p>
            </div>
            <div class="ci-actions">
              <button type="button" class="cia ok" on:click={() => activate(item.id)} aria-label="Activate draft" disabled={busy}>
                <Check size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
              <button type="button" class="cia del" on:click={() => trash(item.id)} aria-label="Move draft to trash" disabled={busy}>
                <Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {:else}
    <p class="muted">No inactive drafts waiting for review.</p>
  {/if}
</section>
