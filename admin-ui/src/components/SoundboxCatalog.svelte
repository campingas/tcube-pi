<script lang="ts">
  import { Gamepad2, Moon } from "@lucide/svelte";
  import type { SoundboxItem } from "../api";

  export let items: SoundboxItem[] = [];
  export let loading = false;
  export let error: string | null = null;
  export let busy = false;
  export let onPreview: (item: { id: string; title: string; preview_url: string | null }) => void | Promise<void>;
  export let onToggle: (slug: string, active: boolean) => void | Promise<void>;

  const groups = [
    { category: "bedtime", label: "Bedtime songs" },
    { category: "retro", label: "Retro gaming themes" }
  ] as const;

  function handleKeydown(event: KeyboardEvent, item: SoundboxItem) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    void onPreview({ id: `soundbox-${item.slug}`, title: item.title, preview_url: item.preview_url });
  }
</script>

<section class="content-surface">
  {#if loading}
    <p class="muted">Loading SoundBox sounds...</p>
  {:else if error}
    <p class="inline-error">{error}</p>
  {:else if items.length}
    {#each groups as group}
      <div class="soundbox-group">
        <p class="soundbox-group-label">{group.label}</p>
        <div class="content-list">
          {#each items.filter((item) => item.category === group.category) as item}
            <div
              class="ci ci-playable"
              role="button"
              tabindex="0"
              aria-label={`Play ${item.title}`}
              data-testid={`soundbox-item-${item.slug}`}
              on:click={() => onPreview({ id: `soundbox-${item.slug}`, title: item.title, preview_url: item.preview_url })}
              on:keydown={(event) => handleKeydown(event, item)}
            >
              <div class="ci-icon fpi-soundbox">
                {#if item.category === "bedtime"}
                  <Moon size={16} strokeWidth={1.5} aria-hidden="true" />
                {:else}
                  <Gamepad2 size={16} strokeWidth={1.5} aria-hidden="true" />
                {/if}
              </div>
              <div class="ci-meta">
                <strong class="ci-name" title={item.title}>{item.title}</strong>
                <p class="ci-detail">Built-in melody · {item.active ? "Plays on button press" : "Turned off"}</p>
              </div>
              <button
                type="button"
                class="soundbox-toggle"
                class:soundbox-toggle-on={item.active}
                role="switch"
                aria-checked={item.active}
                aria-label={`Toggle ${item.title}`}
                disabled={busy}
                on:click|stopPropagation={() => onToggle(item.slug, !item.active)}
              >
                {item.active ? "On" : "Off"}
              </button>
            </div>
          {/each}
        </div>
      </div>
    {/each}
    <p class="muted soundbox-note">Built-in melodies are synthesized on the cube. Recording, uploads, and generation are not available in SoundBox mode.</p>
  {:else}
    <p class="muted">No SoundBox sounds available.</p>
  {/if}
</section>
