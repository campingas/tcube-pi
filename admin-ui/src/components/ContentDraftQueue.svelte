<script lang="ts">
  import { Check, Trash2 } from "@lucide/svelte";
  import type { InactiveContentItem } from "../api";
  import { sourceLabel, trimAudioTitle } from "../view-utils";

  export let items: InactiveContentItem[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let busy = false;
  export let canClearGenerated = false;
  export let activate: (id: string) => void | Promise<void>;
  export let trash: (id: string) => void | Promise<void>;
  export let clearGenerated: () => void | Promise<void>;
</script>

<section class="content-surface">
  {#if canClearGenerated}
    <div class="draft-actions-row">
      <span class="draft-actions-note">Drafts stay inactive until activated.</span>
      <button type="button" class="btn-secondary draft-clear-btn" on:click={clearGenerated} disabled={busy}>
        Clear generated
      </button>
    </div>
  {/if}

  {#if loading}
    <p class="muted">Loading drafts...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    <div class="content-list">
      {#each items as item}
        <div class="ci" role="listitem">
          <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : item.source === 'recorded' ? 'ci-recorded' : 'ci-default'}">
            <Check size={16} strokeWidth={1.5} aria-hidden="true" />
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
      {/each}
    </div>
  {:else}
    <p class="muted">No inactive drafts waiting for review.</p>
  {/if}
</section>
