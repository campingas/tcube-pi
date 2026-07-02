<script lang="ts">
  import {
    Activity,
    TriangleAlert,
    ArrowRight,
    Bolt,
    Check,
    CircleCheck,
    Cuboid,
    Database,
    FileVolume,
    Folder,
    Hand,
    HardDrive,
    Languages,
    LogIn,
    Mic,
    Minus,
    Music,
    PawPrint,
    Play,
    RefreshCw,
    Settings,
    Trash2,
    Upload,
    UsbIcon,
    UsersRound,
    WandSparkles,
    Wifi,
    Wrench
  } from "@lucide/svelte";
  import type { AuthSession, RecentActivityEvent, ServiceStatus, SetupReview } from "../api";
  import type { ButtonMode, ContentType } from "../api";
  import type { InventoryFilter, MessageType } from "../types";
  import TopBar from "../components/TopBar.svelte";
  import { contentLabel, faceName, modeClass, relativeTime } from "../view-utils";

  type DashboardButton = {
    id: number;
    mode: ButtonMode;
    language: string;
    contentType: ContentType | null;
    activeCount: number;
  };

  type Prerequisite = {
    id: string;
    label: string;
    detail: string;
    complete: boolean;
    action: string;
  };

  export let state: {
    status: ServiceStatus | null;
    setup: SetupReview | null;
    session: AuthSession | null;
    message: string;
    messageType: MessageType;
    buttons: DashboardButton[];
    events: RecentActivityEvent[];
    prerequisites: Prerequisite[];
    setupReady: boolean;
    blockedSetupText: string;
    totalActive: number;
    totalDrafts: number;
    totalUnused: number;
    menuLlmOnline: boolean;
    menuLlmStatusLoading: boolean;
    menuLlmLabel: string;
  };

  export let actions: {
    goHome: () => void;
    openStatDetail: (filter: InventoryFilter) => void;
    openSettings: () => void;
    openButtonConfig: (id: number) => void;
    selectSetupAction: (id: string) => void;
    completeSetup: () => void | Promise<void>;
  };

  function playsToday(events: RecentActivityEvent[]) {
    const today = new Date().toISOString().slice(0, 10);
    return events.filter((event) => event.kind === "button_pressed" && event.occurred_at.startsWith(today)).length;
  }

  function activityButtonText(event: RecentActivityEvent) {
    if (!event.button_id && !event.button_label) return "";
    return `${event.button_label || faceName(event.button_id ?? 0)} button`;
  }

  function activityAudioName(event: RecentActivityEvent) {
    return event.audio_filename || event.content_title || event.response_text || event.response_id || event.content_id || "audio";
  }

  function activityText(event: RecentActivityEvent) {
    const button = activityButtonText(event);
    const audio = activityAudioName(event);
    if (event.kind === "signed_in") return event.text ? `Signed in as ${event.text}` : "Signed in";
    if (event.kind === "button_pressed") return `${button || "Button"} pressed — played ${audio}`;
    if (event.kind === "content_recorded") return `${audio} recorded${button ? ` for ${button}` : ""}`;
    if (event.kind === "content_uploaded") return `${audio} uploaded${button ? ` for ${button}` : ""}`;
    if (event.kind === "content_generated") return `${audio} generated${button ? ` for ${button}` : ""}`;
    if (event.kind === "content_activated") return `${audio} activated${button ? ` on ${button}` : ""}`;
    if (event.kind === "content_deleted") return `${audio} deleted${button ? ` from ${button}` : ""}`;
    return event.text || audio;
  }

  function activityBadge(event: RecentActivityEvent) {
    if (event.kind === "signed_in") return "Auth";
    if (event.kind === "button_pressed") return "Play";
    if (event.kind === "content_recorded") return "Record";
    if (event.kind === "content_uploaded") return "Upload";
    if (event.kind === "content_generated") return "Generate";
    if (event.kind === "content_activated") return "Active";
    if (event.kind === "content_deleted") return "Trash";
    return "Event";
  }

  let wifiVerified = false;
  let wifiSsid = "wi-fi";
  let wifiAddress = "192.168.0.1";
  let usbConnected = false;
  let usbAddress = "Not connected";
  $: wifiVerified = Boolean(state.setup?.wifi_verified);
  $: wifiSsid = wifiVerified ? state.setup?.wifi_ssid?.trim() || "wi-fi" : "wi-fi";
  $: wifiAddress = wifiVerified ? state.setup?.dashboard_ip?.trim() || "192.168.0.1" : "192.168.0.1";
  $: usbConnected = Boolean(state.status?.usb_connected);
  $: usbAddress = state.status?.usb_address?.trim() || "Not connected";
</script>

<TopBar
  session={state.session}
  roleLabel={state.session?.cubes?.[0]?.role === "owner" ? "owner" : state.session?.cubes?.[0]?.role === "manager" ? "manager" : state.session?.cubes?.[0]?.role || "member"}
  roleClass={state.session?.cubes?.[0]?.role === "owner" ? "owner" : state.session?.cubes?.[0]?.role === "manager" ? "admin" : "member"}
  goHome={actions.goHome}
  goBack={actions.goHome}
  openSettings={actions.openSettings}
/>

<div class="status-bar" role="status" aria-label="System health">
  {#if !Boolean(state.status?.database_present)}
    <div class="status-item">
      <span class:sdot-ok={Boolean(state.status?.database_present)} class:sdot-warn={!Boolean(state.status?.database_present)} class="sdot"></span>
      <Database size={14} strokeWidth={1.5} aria-hidden="true" />
      <span class:status-ok={Boolean(state.status?.database_present)} class:status-warn={!Boolean(state.status?.database_present)}>Database</span>
    </div>
  {/if}
  {#if !Boolean(state.status?.media_root)}
    <div class="status-item">
      <span class:sdot-ok={Boolean(state.status?.media_root)} class:sdot-warn={!Boolean(state.status?.media_root)} class="sdot"></span>
      <HardDrive size={14} strokeWidth={1.5} aria-hidden="true" />
      <span class:status-ok={Boolean(state.status?.media_root)} class:status-warn={!Boolean(state.status?.media_root)}>Audio</span>
    </div>
  {/if}
  {#if !Boolean(state.status?.content_root)}
    <div class="status-item">
      <span class:sdot-ok={Boolean(state.status?.content_root)} class:sdot-warn={!Boolean(state.status?.content_root)} class="sdot"></span>
      <Folder size={14} strokeWidth={1.5} aria-hidden="true" />
      <span class:status-ok={Boolean(state.status?.content_root)} class:status-warn={!Boolean(state.status?.content_root)}>Content</span>
    </div>
  {/if}
  {#if !Boolean(state.setup?.wifi_verified)}
    <div class="status-item">
      <span class:sdot-ok={Boolean(state.setup?.wifi_verified)} class:sdot-warn={!Boolean(state.setup?.wifi_verified)} class="sdot"></span>
      <Wifi size={14} strokeWidth={1.5} aria-hidden="true" />
      <span class:status-ok={Boolean(state.setup?.wifi_verified)} class:status-warn={!Boolean(state.setup?.wifi_verified)}>Wi-Fi</span>
    </div>
  {/if}
  <div class="status-item">
    <span class:sdot-ok={state.menuLlmOnline} class:sdot-warn={!state.menuLlmOnline} class="sdot"></span>
    {#if state.menuLlmOnline}
      <CircleCheck class="status-ok" size={14} strokeWidth={1.5} aria-hidden="true" />
    {:else if state.menuLlmStatusLoading}
      <RefreshCw class="status-warn" size={14} strokeWidth={1.5} aria-hidden="true" />
    {:else}
      <TriangleAlert class="status-warn" size={14} strokeWidth={1.5} aria-hidden="true" />
    {/if}
    <span class:status-ok={state.menuLlmOnline} class:status-warn={!state.menuLlmOnline}>{state.menuLlmLabel}</span>
  </div>
</div>

<div class="body">
  <section class:error={state.messageType === "error"} class:success={state.messageType === "success"} class="notice" aria-live="polite">
    {state.message}
  </section>

  {#if !state.setupReady}
    <section class="setup-banner" aria-label="Setup checklist">
      <div class="setup-banner-hdr">
        <TriangleAlert size={18} strokeWidth={1.5} aria-hidden="true" />
        <div class="setup-banner-title">Setup incomplete</div>
        <div class="setup-pct">{state.prerequisites.filter((item) => item.complete).length} of {state.prerequisites.length} done</div>
      </div>
      <div class="prereq-list" role="list">
        {#each state.prerequisites as item}
          <button type="button" class:prereq-done={item.complete} class="prereq-item" on:click={() => actions.selectSetupAction(item.id)}>
            <div class:pc-done={item.complete} class:pc-todo={!item.complete} class="prereq-check">
              {#if item.complete}<Check size={12} strokeWidth={1.5} aria-hidden="true" />{:else}<Minus size={12} strokeWidth={1.5} aria-hidden="true" />{/if}
            </div>
            <div class="prereq-body">
              <div class="prereq-name">{item.label}</div>
              <div class="prereq-detail">{item.detail}</div>
            </div>
            {#if !item.complete}
              <div class="prereq-action">{item.action}<ArrowRight size={13} strokeWidth={1.5} aria-hidden="true" /></div>
            {/if}
          </button>
        {/each}
      </div>
      <button
        type="button"
        class:ready={state.setupReady}
        class="setup-complete-btn"
        title={!state.setupReady ? `Missing: ${state.blockedSetupText}` : "Completing setup switches the cube to child mode."}
        disabled={!state.setupReady}
        on:click={actions.completeSetup}
      >
        <Play size={16} strokeWidth={1.5} aria-hidden="true" />
        Complete setup — {state.prerequisites.filter((item) => !item.complete).length} items remaining
      </button>
    </section>
  {/if}

  <section class="card" data-testid="dashboard-hero-card">
    <div class="cube-hero">
      <div class="cube-avatar" aria-hidden="true">
        <Cuboid size={28} strokeWidth={1.5} />
      </div>
      <div class="cube-info">
        <div class="cube-name">{state.setup?.cube_name || "Not set"}</div>
        <div class="cube-sub" data-testid="hero-wifi-line">
          <span
            class:hero-icon-ok={wifiVerified}
            class:hero-icon-warn={!wifiVerified}
            class="cube-sub-icon"
            data-testid="hero-wifi-icon"
          >
            <Wifi size={13} strokeWidth={1.5} aria-hidden="true" />
          </span>
          <span class="cube-sub-text">
            <span class="cube-sub-label">{wifiSsid}</span>
            <span class="cube-sub-sep">·</span>
            <span class="cube-sub-value">{wifiAddress}</span>
          </span>
        </div>
        <div class="cube-sub" data-testid="hero-usb-line">
          <span
            class:hero-icon-ok={usbConnected}
            class:hero-icon-muted={!usbConnected}
            class="cube-sub-icon"
            data-testid="hero-usb-icon"
          >
            <UsbIcon size={13} strokeWidth={1.5} aria-hidden="true" />
          </span>
          <span class="cube-sub-text">
            <span class="cube-sub-label">USB</span>
            <span class="cube-sub-sep">·</span>
            <span class="cube-sub-value">{usbAddress}</span>
          </span>
        </div>
      </div>
    </div>
    <div class="cube-stats" aria-label="Cube statistics" data-testid="dashboard-stats">
      <button type="button" class="cstat" data-testid="dashboard-stat-presses" on:click={() => actions.openStatDetail("presses_today")}>
        <div class="cstat-num">{playsToday(state.events)}</div>
        <div class="cstat-lbl">Presses today</div>
      </button>
      <button type="button" class="cstat" data-testid="dashboard-stat-active" on:click={() => actions.openStatDetail("active")}>
        <div class="cstat-num stat-active">{state.totalActive}</div>
        <div class="cstat-lbl">Active sounds</div>
      </button>
      <button type="button" class="cstat" data-testid="dashboard-stat-draft" on:click={() => actions.openStatDetail("draft")}>
        <div class="cstat-num stat-draft">{state.totalDrafts}</div>
        <div class="cstat-lbl">Drafts</div>
      </button>
      <button type="button" class="cstat" data-testid="dashboard-stat-unused" on:click={() => actions.openStatDetail("unused")}>
        <div class="cstat-num stat-unused">{state.totalUnused}</div>
        <div class="cstat-lbl">Unused</div>
      </button>
    </div>
  </section>

  <section class="card" data-testid="dashboard-buttons-card">
    <div class="sec-hdr">
      <div class="sec-title"><Settings size={15} strokeWidth={1.5} aria-hidden="true" />Buttons</div>
    </div>
    <div class="btn-strip-outer">
      <div class="btn-strip" aria-label="Cube button faces" data-testid="dashboard-button-strip">
        {#each state.buttons as button}
          <button type="button" class="btn-face-card" data-testid={`dashboard-button-${button.id}`} on:click={() => actions.openButtonConfig(button.id)}>
            <div class="bfc-icon bfc-{modeClass(button.mode)}" data-testid={`dashboard-button-${button.id}-icon`}>
              {#if button.mode === "language"}
                <Languages size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if button.mode === "animals"}
                <PawPrint size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if button.mode === "music"}
                <Music size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if button.mode === "setup_help"}
                <Wrench size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else}
                <Minus size={18} strokeWidth={1.5} aria-hidden="true" />
              {/if}
            </div>
            <div class="bfc-name">{faceName(button.id)}</div>
            <div class="bfc-count">{button.contentType ? `${button.activeCount} sounds` : "—"}</div>
            <div class="bfc-mode bfm-{modeClass(button.mode)}">{contentLabel(button.mode, button.language)}</div>
          </button>
        {/each}
      </div>
    </div>
  </section>

  <section class="card">
    <div class="sec-hdr">
      <div class="sec-title"><Bolt size={15} strokeWidth={1.5} aria-hidden="true" />Quick actions</div>
    </div>
    <div class="actions-grid">
    <!-- Keep this button disabled for now. For later use of that class. -->
      <!-- <button type="button" class="action-card primary-action" disabled>
        <div class="ac-icon ac-icon-white"><RefreshCw size={20} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="ac-body">
          <div class="ac-title">Run curation</div>
          <div class="ac-desc">Update schedule with LLM</div>
        </div>
        <ArrowRight class="ac-arrow" size={18} strokeWidth={1.5} aria-hidden="true" />
      </button> -->
      <button type="button" class="action-card" disabled>
        <div class="ac-icon ac-icon-violet"><RefreshCw size={20} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="ac-body">
          <div class="ac-title">Run curation</div>
          <div class="ac-desc">Update schedule with LLM</div>
        </div>
        <ArrowRight class="ac-arrow" size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
      <button type="button" class="action-card" disabled>
        <div class="ac-icon ac-icon-violet"><UsersRound size={20} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="ac-body">
          <div class="ac-title">Learning stats</div>
          <div class="ac-desc">Words heard, repetitions</div>
        </div>
        <ArrowRight class="ac-arrow" size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
    </div>
  </section>

  <section class="card">
    <div class="sec-hdr">
      <div class="sec-title"><Activity size={15} strokeWidth={1.5} aria-hidden="true" />Recent activity</div>
    </div>
    {#if state.events.length === 0}
      <div class="empty-state">
        <Activity size={24} strokeWidth={1.5} aria-hidden="true" />
        <strong>No button events yet</strong>
        <p>The feed appears after the child presses a button and the runtime logs the event.</p>
      </div>
    {:else}
      <div class="feed" role="list" aria-label="Recent activity feed">
        {#each state.events as event}
          <div class="feed-item" role="listitem">
            <div class="feed-icon fi-{event.kind}">
              {#if event.kind === "signed_in"}
                <LogIn size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "button_pressed"}
                <Hand size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "content_recorded"}
                <Mic size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "content_uploaded"}
                <Upload size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "content_generated"}
                <WandSparkles size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "content_activated"}
                <Play size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else if event.kind === "content_deleted"}
                <Trash2 size={14} strokeWidth={1.5} aria-hidden="true" />
              {:else}
                <FileVolume size={14} strokeWidth={1.5} aria-hidden="true" />
              {/if}
            </div>
            <div class="feed-body">
              <div class="feed-text">{activityText(event)}</div>
              <div class="feed-time">{relativeTime(event.occurred_at)}</div>
            </div>
            <span class="feed-badge fb-{event.kind}">{activityBadge(event)}</span>
          </div>
        {/each}
      </div>
    {/if}
  </section>
</div>
