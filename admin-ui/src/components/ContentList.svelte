<script lang="ts">
  import type { ActiveContentItem } from "../api";
  import { contentPlaySummary, trimAudioTitle } from "../view-utils";
  import AudioContentRow from "./AudioContentRow.svelte";

  export let items: ActiveContentItem[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let busy = false;
  export let contentDurations: Record<string, number> = {};
  export let onPreview: (item: ActiveContentItem) => void | Promise<void>;
  export let onTrash: (item: { id: string; title: string }) => void;
</script>

<section class="content-surface">
  {#if loading}
    <p class="muted">Loading active content...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    <div class="content-list">
      {#each items as item}
        <AudioContentRow
          item={item}
          displayTitle={trimAudioTitle(item.title)}
          detail={contentPlaySummary(item, contentDurations)}
          busy={busy}
          playable={true}
          showTrash={true}
          onPreview={() => onPreview(item)}
          onTrash={() => onTrash(item)}
        />
      {/each}
    </div>
  {:else}
    <p class="muted">No active content for this face.</p>
  {/if}
</section>
