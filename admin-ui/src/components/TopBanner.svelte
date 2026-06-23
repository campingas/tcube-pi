<script lang="ts">
  import type { ServiceStatus, SetupReview } from "../api";
  import CubeLogo from "./CubeLogo.svelte";
  import StatusChip from "./StatusChip.svelte";

  export let status: ServiceStatus | null;
  export let setup: SetupReview | null;
  export let loading = false;
  export let busy = false;
  export let refresh: () => void | Promise<void>;
</script>

<header class="neo-surface top-banner">
  <div class="top-banner-content">
    <div class="brand-lockup">
      <CubeLogo />
      <div>
        <p class="terminal-kicker">Local Pi control plane</p>
        <h1 class="app-title"><span>T-CUBE</span> ADMIN</h1>
        <p class="muted compact-line">
          {setup?.cube_name || "Unprovisioned cube"} · {status?.hostname || "host pending"} · static UI
        </p>
      </div>
    </div>

    <div class="banner-actions">
      <div class="connection-row" aria-label="Runtime status">
        <StatusChip label="Pi" value={status?.status ?? "loading"} active={status?.status === "ok"} />
        <StatusChip label="Database" value={status?.database_present} active={Boolean(status?.database_present)} />
        <StatusChip label="UI assets" value={status?.ui_dist_present} active={Boolean(status?.ui_dist_present)} />
        <StatusChip label="Content" value={status?.content_root} active={Boolean(status?.content_root)} />
        <StatusChip label="Media" value={status?.media_root} active={Boolean(status?.media_root)} />
      </div>
      <button type="button" class="neo-button secondary" on:click={refresh} disabled={busy || loading}>
        Refresh
      </button>
    </div>
  </div>
</header>

