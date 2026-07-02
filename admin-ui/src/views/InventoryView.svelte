<script lang="ts">
  import { TriangleAlert, FileVolume, Hand } from "@lucide/svelte";
  import type { AuthSession, ContentInventory, ContentInventoryItem, RecentActivityEvent } from "../api";
  import type { InventoryFilter, MessageType } from "../types";
  import TopBar from "../components/TopBar.svelte";
  import AudioContentRow from "../components/AudioContentRow.svelte";
  import { contentTypeLabel, faceName, relativeTime, sourceLabel } from "../view-utils";

  export let state: {
    session: AuthSession | null;
    message: string;
    messageType: MessageType;
    inventory: ContentInventory | null;
    inventoryError: string | null;
    events: RecentActivityEvent[];
    filter: InventoryFilter;
  };

  export let actions: {
    goHome: () => void;
    openSettings: () => void;
    openInventoryButton: (item: ContentInventoryItem) => void;
  };

  function todayEvents(events: RecentActivityEvent[]) {
    const today = new Date().toISOString().slice(0, 10);
    return events.filter((event) => event.kind === "button_pressed" && event.occurred_at.startsWith(today));
  }

  function inventoryItems(status: InventoryFilter) {
    if (status === "presses_today") return [];
    return state.inventory?.items.filter((item) => item.status === status) ?? [];
  }

  function title(filter: InventoryFilter) {
    if (filter === "presses_today") return "Presses today";
    if (filter === "active") return "Active sounds";
    if (filter === "draft") return "Drafts";
    return "Unused audio";
  }

  function detail(filter: InventoryFilter) {
    if (filter === "presses_today") return "Button presses recorded since local midnight.";
    if (filter === "active") return "Files playable in the current button setup.";
    if (filter === "draft") return "Inactive files waiting for review.";
    return "Active files hidden by the current button mode or language.";
  }

  function emptyTitle(filter: InventoryFilter) {
    if (filter === "presses_today") return "No plays today";
    if (filter === "active") return "No active sounds";
    if (filter === "draft") return "No drafts";
    return "No unused audio";
  }

  function audioDetail(item: ContentInventoryItem) {
    const language = item.language ? ` · ${item.language}` : "";
    return `${faceName(item.button_id)} · ${contentTypeLabel(item.content_type)}${language} · ${sourceLabel(item.source)}`;
  }

  $: filteredEvents = todayEvents(state.events);
  $: filteredItems = inventoryItems(state.filter);
  $: count = state.filter === "presses_today" ? filteredEvents.length : filteredItems.length;
</script>

<TopBar
  session={state.session}
  roleLabel={state.session?.cubes?.[0]?.role || "member"}
  roleClass={state.session?.cubes?.[0]?.role === "owner" ? "owner" : state.session?.cubes?.[0]?.role === "manager" ? "admin" : "member"}
  showBack={true}
  goHome={actions.goHome}
  goBack={actions.goHome}
  openSettings={actions.openSettings}
/>

<div class="body">
  <section class:error={state.messageType === "error"} class:success={state.messageType === "success"} class="notice" aria-live="polite">
    {state.message}
  </section>
  <section class="card inventory-card" data-testid="stat-detail-view">
    <div class="sec-hdr">
      <div>
        <div class="sec-title">
          {#if state.filter === "presses_today"}
            <Hand size={16} strokeWidth={1.5} aria-hidden="true" />
          {:else}
            <FileVolume size={16} strokeWidth={1.5} aria-hidden="true" />
          {/if}
          {title(state.filter)}
        </div>
        <div class="inventory-detail">{detail(state.filter)}</div>
      </div>
      <div class="inventory-actions">
        <span class="inventory-count">{count}</span>
      </div>
    </div>
    {#if state.filter !== "presses_today" && state.inventoryError}
      <div class="content-api-error" role="alert">
        <TriangleAlert size={15} strokeWidth={1.5} aria-hidden="true" />
        <span>{state.inventoryError}</span>
      </div>
    {:else if count === 0}
      <div class="empty-state">
        {#if state.filter === "presses_today"}
          <Hand size={24} strokeWidth={1.5} aria-hidden="true" />
        {:else}
          <FileVolume size={24} strokeWidth={1.5} aria-hidden="true" />
        {/if}
        <strong>{emptyTitle(state.filter)}</strong>
        <p>{detail(state.filter)}</p>
      </div>
    {:else if state.filter === "presses_today"}
      <div class="content-list" role="list">
        {#each filteredEvents as event}
          <div class="ci" role="listitem">
            <div class="ci-icon ci-recorded">
              <Hand size={16} strokeWidth={1.5} aria-hidden="true" />
            </div>
            <div class="ci-meta">
              <strong class="ci-name">{event.response_text || event.response_id || "Played audio"}</strong>
              <p class="ci-detail">{event.button_label || faceName(event.button_id ?? 0)} · {event.mode || "button"} · {relativeTime(event.occurred_at)}</p>
            </div>
          </div>
        {/each}
      </div>
    {:else}
      <div class="content-list" role="list">
        {#each filteredItems as item}
          <AudioContentRow
            item={item}
            detail={audioDetail(item)}
            reason={item.reason}
            showOpen={true}
            onOpen={() => actions.openInventoryButton(item)}
          />
          {/each}
      </div>
    {/if}
  </section>
</div>
