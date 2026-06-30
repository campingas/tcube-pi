<script lang="ts">
  import { KeyRound, LogIn, ShieldCheck, User } from "@lucide/svelte";
  import type { AuthSession } from "../api";
  import type { MessageType } from "../types";

  export let state: {
    session: AuthSession | null;
    invitationCodeFromUrl: string;
    message: string;
    messageType: MessageType;
    bootstrapForm: { username: string; display_name: string; password: string };
    loginForm: { username: string; password: string };
    recoveryForm: { code: string; password: string };
    inviteForm: { code: string; username: string; display_name: string; password: string };
    busy: boolean;
  };

  export let actions: {
    submitInvitation: () => void | Promise<void>;
    submitBootstrap: () => void | Promise<void>;
    submitLogin: () => void | Promise<void>;
    submitRecovery: () => void | Promise<void>;
  };
</script>

<nav class="topbar">
  <div class="topbar-left">
    <button type="button" class="topbar-logo topbar-logo-btn" aria-label="Go to dashboard" on:click={() => window.location.reload()}>
      T<span>·</span>Cube
    </button>
    <div class="topbar-session">Local parent dashboard</div>
  </div>
</nav>

<div class="body auth-body">
  <section class:error={state.messageType === "error"} class:success={state.messageType === "success"} class="notice" aria-live="polite">
    {state.message}
  </section>

  {#if state.invitationCodeFromUrl}
    <section class="card auth-card">
      <div class="sec-hdr">
        <div class="sec-title"><User size={16} strokeWidth={1.5} aria-hidden="true" />Accept manager access</div>
      </div>
      <form class="form-stack" on:submit|preventDefault={actions.submitInvitation}>
        <label>Invitation code <input bind:value={state.inviteForm.code} autocomplete="off" /></label>
        <label>Username <input bind:value={state.inviteForm.username} autocomplete="username" /></label>
        <label>Display name <input bind:value={state.inviteForm.display_name} /></label>
        <label>Password <input bind:value={state.inviteForm.password} type="password" autocomplete="new-password" /></label>
        <button type="submit" class="btn-primary" disabled={state.busy}>Create manager account</button>
      </form>
    </section>
  {/if}

  {#if state.session?.bootstrap_required}
    <section class="card auth-card">
      <div class="sec-hdr">
        <div class="sec-title"><ShieldCheck size={16} strokeWidth={1.5} aria-hidden="true" />Create local owner</div>
      </div>
      <form class="form-stack" on:submit|preventDefault={actions.submitBootstrap}>
        <label>Username <input bind:value={state.bootstrapForm.username} autocomplete="username" /></label>
        <label>Display name <input bind:value={state.bootstrapForm.display_name} /></label>
        <label>Password <input bind:value={state.bootstrapForm.password} type="password" autocomplete="new-password" /></label>
        <button type="submit" class="btn-primary" disabled={state.busy}>Create owner</button>
      </form>
    </section>
  {:else}
    <section class="card auth-card">
      <div class="sec-hdr">
        <div class="sec-title"><LogIn size={16} strokeWidth={1.5} aria-hidden="true" />Log in</div>
      </div>
      <form class="form-stack" on:submit|preventDefault={actions.submitLogin}>
        <label>Username <input bind:value={state.loginForm.username} autocomplete="username" /></label>
        <label>Password <input bind:value={state.loginForm.password} type="password" autocomplete="current-password" /></label>
        <button type="submit" class="btn-primary" disabled={state.busy}>Log in</button>
      </form>
    </section>

    <section class="card auth-card">
      <div class="sec-hdr">
        <div class="sec-title"><KeyRound size={16} strokeWidth={1.5} aria-hidden="true" />Reset password</div>
      </div>
      <form class="form-stack" on:submit|preventDefault={actions.submitRecovery}>
        <label>Recovery code <input bind:value={state.recoveryForm.code} autocomplete="off" /></label>
        <label>New password <input bind:value={state.recoveryForm.password} type="password" autocomplete="new-password" /></label>
        <button type="submit" class="btn-secondary" disabled={state.busy}>Reset password</button>
      </form>
    </section>
  {/if}
</div>
