<script lang="ts">
  import { ChevronRight, ChevronUp, KeyRound, LogIn, ShieldCheck, User } from "@lucide/svelte";
  import type { AuthSession } from "../api";
  import type { MessageType } from "../types";
  import { isIpLiteralHost } from "../view-utils";

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

  // Auto-expand certificate help when parents browse by raw IP, the
  // strongest signal that the cube's root CA is not trusted yet.
  let certHelpOpen = isIpLiteralHost(window.location.hostname);
</script>

<nav class="topbar">
  <div class="topbar-left">
    <div class="topbar-logo">
      T<span>·</span>Cube
    </div>
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

  <section class="card auth-card">
    <button type="button" class="sec-hdr auth-cert-toggle" aria-expanded={certHelpOpen} on:click={() => (certHelpOpen = !certHelpOpen)}>
      <div class="sec-title"><ShieldCheck size={16} strokeWidth={1.5} aria-hidden="true" />Secure this device</div>
      {#if certHelpOpen}<ChevronUp size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<ChevronRight size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
    </button>
    {#if certHelpOpen}
      <div class="form-stack">
        <p class="hint auth-cert-steps">The cube secures this dashboard with its own local certificate. Trust it once on each parent phone or laptop to remove browser warnings and unlock microphone recording.</p>
        <a class="btn-secondary auth-cert-download" href="/ca/root.crt" download="tcube-root-ca.crt">Download cube certificate</a>
        <p class="hint auth-cert-steps">iPhone/iPad: allow the download, install the profile under Settings, General, VPN &amp; Device Management, then enable full trust under Settings, General, About, Certificate Trust Settings.</p>
        <p class="hint auth-cert-steps">Android: install the file under Settings, Security, Encryption &amp; credentials, Install a certificate, CA certificate.</p>
        <p class="hint auth-cert-steps">macOS: open the file with Keychain Access, add it to the System keychain, and set it to Always Trust.</p>
        <p class="hint auth-cert-steps">Once trusted, open https://tcube.local/ from the home network.</p>
      </div>
    {/if}
  </section>
</div>
