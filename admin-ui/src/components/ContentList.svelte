<script lang="ts">
  import type { ActiveContentItem } from "../api";

  export let items: ActiveContentItem[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let busy = false;
  export let trash: (id: string) => void | Promise<void>;
</script>

<section class="content-surface">
  <div class="section-heading-row compact">
    <div>
      <p class="terminal-kicker">Published</p>
      <h3>Active content</h3>
    </div>
    <span class="badge ready">{items.length} active</span>
  </div>

  {#if loading}
    <p class="muted">Loading active content...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    <div class="content-list">
      {#each items as item}
        <article class="content-row">
          <div>
            <strong>{item.title}</strong>
            <p>{item.text || item.audio_path || "Audio only"}</p>
            <span>{item.source} · {item.content_type}</span>
          </div>
          {#if item.preview_url}
            <audio controls src={item.preview_url}></audio>
          {/if}
          <button type="button" class="neo-button danger" on:click={() => trash(item.id)} disabled={busy}>
            Trash
          </button>
        </article>
      {/each}
    </div>
  {:else}
    <p class="muted">No active content for this face.</p>
  {/if}
</section>

