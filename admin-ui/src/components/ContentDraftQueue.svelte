<script lang="ts">
  import type { InactiveContentItem } from "../api";

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
  <div class="section-heading-row compact">
    <div>
      <p class="terminal-kicker">Review queue</p>
      <h3>Inactive drafts</h3>
    </div>
    {#if canClearGenerated}
      <button type="button" class="neo-button secondary small" on:click={clearGenerated} disabled={busy}>
        Clear generated
      </button>
    {/if}
  </div>

  {#if loading}
    <p class="muted">Loading drafts...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    <div class="content-list">
      {#each items as item}
        <article class="content-row draft">
          <div>
            <strong>{item.title}</strong>
            <p>{item.text || item.audio_path}</p>
            <span>{item.source} · {item.language || item.content_type} · inactive</span>
          </div>
          {#if item.preview_url}
            <audio controls src={item.preview_url}></audio>
          {/if}
          <div class="button-row">
            <button type="button" class="neo-button" on:click={() => activate(item.id)} disabled={busy}>
              Activate
            </button>
            <button type="button" class="neo-button danger" on:click={() => trash(item.id)} disabled={busy}>
              Trash
            </button>
          </div>
        </article>
      {/each}
    </div>
  {:else}
    <p class="muted">No inactive drafts waiting for review.</p>
  {/if}
</section>

