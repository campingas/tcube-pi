<script lang="ts">
  import { Bell, ArrowLeft, Settings } from "@lucide/svelte";
  import type { AuthSession } from "../api";

  export let session: AuthSession | null;
  export let roleLabel = "";
  export let roleClass = "";
  export let title = "";
  export let subtitle = "";
  export let showBack = false;
  export let goHome: () => void;
  export let goBack: () => void;
  export let openSettings: () => void;
</script>

<nav class="topbar">
  <div class="topbar-left">
    <button type="button" class="topbar-logo topbar-logo-btn" aria-label="Go to dashboard" on:click={goHome}>
      T<span>·</span>Cube
    </button>
    <div class="topbar-session">
      <span class="topbar-session-prefix">Signed in as</span>
      <span class="topbar-session-user">{session?.account?.display_name ?? session?.account?.username}</span>
      <span class="topbar-session-role role-{roleClass}">{roleLabel}</span>
    </div>
    {#if title}
      <div class="topbar-session">
        <span class="topbar-session-prefix">{title}</span>
        {#if subtitle}<span class="topbar-session-user">{subtitle}</span>{/if}
      </div>
    {/if}
  </div>
  <div class="topbar-right">
    {#if showBack}
      <button type="button" class="icon-btn" aria-label="Back to dashboard" on:click={goBack}>
        <ArrowLeft size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
    {/if}
    <button type="button" class="icon-btn" aria-label="Notifications">
      <Bell size={18} strokeWidth={1.5} aria-hidden="true" />
    </button>
    <button type="button" class="icon-btn" aria-label="Settings" on:click={openSettings}>
      <Settings size={18} strokeWidth={1.5} aria-hidden="true" />
    </button>
  </div>
</nav>
