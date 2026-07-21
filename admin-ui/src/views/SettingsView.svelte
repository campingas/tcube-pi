<script lang="ts">
  import {
    TriangleAlert,
    ChevronRight,
    ChevronUp,
    Copy,
    Cuboid,
    KeyRound,
    Link,
    Lock,
    LogOut,
    Plus,
    Volume2,
    VolumeX,
    Timer,
    Usb,
    User,
    Users,
    Wifi,
    X
  } from "@lucide/svelte";
  import type { AudioSettings, AuthSession, PomodoroPreset, PomodoroSettings, RecoveryCode, ServiceStatus, SetupReview } from "../api";
  import { volumeLabel } from "../audio-settings-controller";
  import { pomodoroCanEnable, pomodoroTriggerInstruction, recommendationForAge } from "../focus-routine-controller";
  import type { PomodoroForm } from "../focus-routine-controller";
  import type { MessageType } from "../types";
  import TopBar from "../components/TopBar.svelte";

  export let state: {
    session: AuthSession | null;
    status: ServiceStatus | null;
    setup: SetupReview | null;
    pomodoro: PomodoroSettings | null;
    pomodoroForm: PomodoroForm;
    audioSettings: AudioSettings | null;
    audioVolume: number;
    audioSaving: boolean;
    audioMessage: string | null;
    audioError: string | null;
    message: string;
    messageType: MessageType;
    roleLabel: string;
    isOwner: boolean;
    busy: boolean;
    cubeName: string;
    wifiForm: { ssid: string; dashboard_ip: string };
    settingsCubeNameOpen: boolean;
    settingsWifiOpen: boolean;
    settingsRecoveryOpen: boolean;
    recoveryCode: RecoveryCode | null;
    invitation: { code: string; expires_at: string } | null;
    totalUnused: number;
    factoryResetPromptOpen: boolean;
    factoryResetConfirmation: string;
  };

  export let actions: {
    goHome: () => void;
    openSettings: () => void;
    setSettingsCubeNameOpen: (open: boolean) => void;
    setSettingsWifiOpen: (open: boolean) => void;
    setSettingsRecoveryOpen: (open: boolean) => void;
    saveCubeName: (value: string) => void | Promise<void>;
    verifyWifi: (ssid: string, dashboardIp: string) => void | Promise<void>;
    setPomodoroForm: (form: PomodoroForm) => void;
    applyPomodoroAge: (age: string) => void;
    applyPomodoroPreset: (preset: PomodoroPreset) => void;
    updatePomodoroCustom: (patch: Partial<Omit<PomodoroForm, "preset">>) => void;
    savePomodoro: () => void | Promise<void>;
    setAudioVolume: (volumePercent: number) => void;
    saveAudioVolume: (volumePercent: number) => void | Promise<void>;
    createRecoveryCode: () => void | Promise<void>;
    copyText: (value: string, label: string) => void | Promise<void>;
    createManagerInvitation: () => void | Promise<void>;
    clearAllUnusedContent: () => void | Promise<void>;
    openFactoryResetPrompt: () => void;
    logout: () => void | Promise<void>;
    dismissInvitation: () => void;
    dismissRecoveryCode: () => void;
    setFactoryResetConfirmation: (value: string) => void;
    cancelFactoryReset: () => void;
    confirmFactoryReset: () => void | Promise<void>;
  };

  function invitationUrl(code: string) {
    return `${window.location.origin}/?invite=${encodeURIComponent(code)}`;
  }

  let cubeName = state.cubeName;
  let wifiSsid = state.wifiForm.ssid;
  let wifiDashboardIp = state.wifiForm.dashboard_ip;
  $: pomodoroAgeValue = Number(state.pomodoroForm.childAgeYears);
  $: pomodoroRecommendation = recommendationForAge(
    state.pomodoroForm.childAgeYears.trim() && Number.isInteger(pomodoroAgeValue) && pomodoroAgeValue >= 3 && pomodoroAgeValue <= 18 ? pomodoroAgeValue : null
  );
  $: pomodoroEnableAllowed = pomodoroCanEnable(state.pomodoroForm);
  $: triggerInstruction = pomodoroTriggerInstruction(state.pomodoro);
  $: pomodoroStatus = state.pomodoro?.enabled && state.pomodoro?.validated_at ? "Enabled" : state.pomodoro?.validated_at ? "Saved" : "Not saved";
</script>

<TopBar
  session={state.session}
  roleLabel={state.roleLabel}
  roleClass={state.roleLabel === "owner" ? "owner" : state.roleLabel === "manager" ? "admin" : "member"}
  title="Settings"
  subtitle={state.setup?.cube_name || ""}
  showBack={true}
  goHome={actions.goHome}
  goBack={actions.goHome}
  openSettings={actions.openSettings}
/>

<div class="body settings-body">
  <section class:error={state.messageType === "error"} class:success={state.messageType === "success"} class="notice" aria-live="polite">
    {state.message}
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Cube</div>
    <div class="settings-group-card">
      <button
        type="button"
        class:expanded={state.settingsCubeNameOpen}
        class="settings-row"
        on:click={() => actions.setSettingsCubeNameOpen(!state.settingsCubeNameOpen)}
        disabled={!state.isOwner}
      >
        <div class="settings-row-icon si-coral"><Cuboid size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">Cube name</div>
        </div>
        <div class="settings-row-right">
          <span class="settings-row-value">{state.setup?.cube_name ?? state.cubeName}</span>
          {#if state.settingsCubeNameOpen}<ChevronUp size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<ChevronRight size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
        </div>
      </button>
      {#if state.settingsCubeNameOpen}
        <form class="settings-edit" on:submit|preventDefault={() => actions.saveCubeName(cubeName)}>
          <label class="field-label">Display name
            <input class="settings-input" bind:value={cubeName} maxlength="32" disabled={!state.isOwner} />
          </label>
          <div class="settings-hint">Shown in this dashboard and in the activity log.</div>
          <div class="settings-row-actions">
            <button type="button" class="settings-cancel-btn" on:click={() => (cubeName = state.setup?.cube_name || "T-Cube")} disabled={state.busy}>Cancel</button>
            <button type="submit" class="settings-save-btn" disabled={state.busy || !state.isOwner}>Save name</button>
          </div>
        </form>
      {/if}

      <button
        type="button"
        class:expanded={state.settingsWifiOpen}
        class="settings-row"
        on:click={() => actions.setSettingsWifiOpen(!state.settingsWifiOpen)}
        disabled={!state.isOwner}
      >
        <div class="settings-row-icon si-teal"><Wifi size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">Wi-Fi</div>
          <div class="settings-row-desc">{state.setup?.wifi_verified ? state.setup?.wifi_ssid || state.wifiForm.ssid || "wi-fi" : "wi-fi"} · {state.setup?.wifi_verified ? state.setup?.dashboard_ip || "192.168.0.1" : "192.168.0.1"}</div>
        </div>
        <div class="settings-row-right">
          <span class:bs-teal={Boolean(state.setup?.wifi_verified)} class:bs-amber={!Boolean(state.setup?.wifi_verified)} class="settings-badge">{state.setup?.wifi_verified ? "Verified" : "Pending"}</span>
          {#if state.settingsWifiOpen}<ChevronUp size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<ChevronRight size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
        </div>
      </button>
      {#if state.settingsWifiOpen}
        <form class="settings-edit" on:submit|preventDefault={() => actions.verifyWifi(wifiSsid, wifiDashboardIp)}>
          <label class="field-label">Wi-Fi SSID
            <input class="settings-input" bind:value={wifiSsid} placeholder="Home Wi-Fi" disabled={!state.isOwner} />
          </label>
          <label class="field-label">Dashboard IP
            <input class="settings-input" bind:value={wifiDashboardIp} placeholder="192.168.1.10" disabled={!state.isOwner} />
          </label>
          <div class="settings-row-actions">
            <button type="button" class="settings-cancel-btn" on:click={() => actions.setSettingsWifiOpen(false)} disabled={state.busy}>Cancel</button>
            <button type="submit" class="settings-save-btn" disabled={state.busy || !state.isOwner}>Mark verified</button>
          </div>
        </form>
      {/if}

      <div class="settings-row no-tap">
        <div class:si-teal={Boolean(state.status?.usb_connected)} class:si-muted={!Boolean(state.status?.usb_connected)} class="settings-row-icon">
          <Usb size={17} strokeWidth={1.5} aria-hidden="true" />
        </div>
        <div class="settings-row-body">
          <div class="settings-row-title">USB address</div>
          <div class="settings-row-desc">Available when connected via USB-C OTG</div>
        </div>
        <div class="settings-row-right">
          <span class="settings-row-value">{state.status?.usb_address ?? "Not connected"}</span>
        </div>
      </div>
    </div>
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Sound · Owner only</div>
    <div class="settings-group-card sound-settings-card">
      <div class="settings-row no-tap">
        <div class:si-muted={state.audioVolume === 0} class:si-teal={state.audioVolume > 0} class="settings-row-icon">
          {#if state.audioVolume === 0}<VolumeX size={17} strokeWidth={1.5} aria-hidden="true" />{:else}<Volume2 size={17} strokeWidth={1.5} aria-hidden="true" />{/if}
        </div>
        <div class="settings-row-body">
          <div class="settings-row-title">Device volume</div>
          <div class="settings-row-desc">Controls every sound played by this cube.</div>
        </div>
        <div class="settings-row-right">
          <span class="settings-row-value" data-testid="audio-volume-label">{volumeLabel(state.audioVolume)}</span>
        </div>
      </div>
      <div class="sound-volume-control">
        <label class="field-label" for="device-volume">Master volume</label>
        <input
          id="device-volume"
          data-testid="audio-volume-slider"
          type="range"
          min="0"
          max="100"
          step="5"
          value={state.audioVolume}
          disabled={!state.isOwner || state.audioSaving}
          aria-describedby="audio-volume-status"
          on:input={(event) => actions.setAudioVolume(Number((event.currentTarget as HTMLInputElement).value))}
          on:change={(event) => actions.saveAudioVolume(Number((event.currentTarget as HTMLInputElement).value))}
        />
        <div id="audio-volume-status" class:error={Boolean(state.audioError)} class="sound-volume-status" aria-live="polite">
          {state.audioError ?? (state.audioSaving ? "Saving…" : state.audioMessage ?? (state.isOwner ? "Saved when you release the slider." : "Only an owner can change device volume."))}
        </div>
      </div>
    </div>
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Focus routine · Owner only</div>
    <div class="settings-group-card focus-routine-card">
      <div class="settings-row no-tap">
        <div class:si-teal={state.pomodoro?.enabled} class:si-muted={!state.pomodoro?.enabled} class="settings-row-icon">
          <Timer size={17} strokeWidth={1.5} aria-hidden="true" />
        </div>
        <div class="settings-row-body">
          <div class="settings-row-title">Focus routine</div>
          <div class="settings-row-desc">{triggerInstruction}</div>
        </div>
        <div class="settings-row-right">
          <span class:bs-teal={state.pomodoro?.enabled} class:bs-amber={!state.pomodoro?.validated_at} class:bs-muted={!state.pomodoro?.enabled && state.pomodoro?.validated_at} class="settings-badge">{pomodoroStatus}</span>
        </div>
      </div>

      <div class="focus-routine-grid">
        <label class="field-label">Child age
          <input
            class="settings-input"
            type="number"
            min="3"
            max="18"
            inputmode="numeric"
            value={state.pomodoroForm.childAgeYears}
            placeholder="Age"
            disabled={!state.isOwner}
            on:input={(event) => actions.applyPomodoroAge((event.currentTarget as HTMLInputElement).value)}
          />
        </label>

        <label class="focus-toggle">
          <input
            type="checkbox"
            checked={state.pomodoroForm.enabled}
            disabled={!state.isOwner || !pomodoroEnableAllowed}
            on:change={(event) => actions.setPomodoroForm({ ...state.pomodoroForm, enabled: (event.currentTarget as HTMLInputElement).checked })}
          />
          <span>Enable after save</span>
        </label>
      </div>

      <div class="focus-recommendation">
        <div>
          <div class="focus-rec-title">Recommended plan</div>
          <div class="focus-rec-copy">{pomodoroRecommendation.reason}</div>
        </div>
        <div class="focus-rec-values">{pomodoroRecommendation.focus_minutes}/{pomodoroRecommendation.break_minutes} x{pomodoroRecommendation.cycles}</div>
      </div>

      <div class="focus-presets" role="radiogroup" aria-label="Focus routine presets">
        {#each ["mini", "focus", "full", "custom"] as preset}
          <button
            type="button"
            class:active={state.pomodoroForm.preset === preset}
            role="radio"
            aria-checked={state.pomodoroForm.preset === preset}
            disabled={!state.isOwner}
            on:click={() => actions.applyPomodoroPreset(preset as PomodoroPreset)}
          >
            {preset === "mini" ? "Mini" : preset === "focus" ? "Focus" : preset === "full" ? "Full" : "Custom"}
          </button>
        {/each}
      </div>

      <div class="focus-routine-grid compact">
        <label class="field-label">Focus minutes
          <input
            class="settings-input"
            type="number"
            min="5"
            max="60"
            value={state.pomodoroForm.focusMinutes}
            disabled={!state.isOwner}
            on:input={(event) => actions.updatePomodoroCustom({ focusMinutes: Number((event.currentTarget as HTMLInputElement).value) })}
          />
        </label>
        <label class="field-label">Break minutes
          <input
            class="settings-input"
            type="number"
            min="1"
            max="30"
            value={state.pomodoroForm.breakMinutes}
            disabled={!state.isOwner}
            on:input={(event) => actions.updatePomodoroCustom({ breakMinutes: Number((event.currentTarget as HTMLInputElement).value) })}
          />
        </label>
        <label class="field-label">Cycles
          <input
            class="settings-input"
            type="number"
            min="1"
            max="8"
            value={state.pomodoroForm.cycles}
            disabled={!state.isOwner}
            on:input={(event) => actions.updatePomodoroCustom({ cycles: Number((event.currentTarget as HTMLInputElement).value) })}
          />
        </label>
      </div>

      <div class="settings-hint">Focus audio uses a generated soft stereo tone. Breaks stay silent except for short transition chimes.</div>
      <div class="settings-row-actions">
        <button type="button" class="settings-save-btn" on:click={actions.savePomodoro} disabled={state.busy || !state.isOwner}>
          Save focus routine
        </button>
      </div>
    </div>
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Account</div>
    <div class="settings-group-card">
      <div class="settings-row no-tap">
        <div class="settings-row-icon si-sage"><User size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">{state.session?.account?.display_name || state.session?.account?.username}</div>
          <div class="settings-row-desc">Signed in</div>
        </div>
        <div class="settings-row-right">
          <span class="settings-badge bs-teal">{state.roleLabel}</span>
        </div>
      </div>

      <button type="button" class="settings-row" disabled title="Password changes are not exposed by the local API yet.">
        <div class="settings-row-icon si-muted"><Lock size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">Change password</div>
          <div class="settings-row-desc">Use a recovery code from the login screen for now.</div>
        </div>
        <div class="settings-row-right">
          <span class="settings-badge bs-muted">Soon</span>
        </div>
      </button>

      <button type="button" class:expanded={state.settingsRecoveryOpen} class="settings-row" on:click={() => actions.setSettingsRecoveryOpen(!state.settingsRecoveryOpen)}>
        <div class="settings-row-icon si-amber"><KeyRound size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">Recovery code</div>
        </div>
        <div class="settings-row-right">
          {#if state.recoveryCode}<span class="settings-badge bs-amber">Created</span>{/if}
          {#if state.settingsRecoveryOpen}<ChevronUp size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<ChevronRight size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
        </div>
      </button>
      {#if state.settingsRecoveryOpen}
        <div class="settings-recovery-block">
          {#if state.recoveryCode}
            <div class="settings-secret-code">{state.recoveryCode.code}</div>
            <div class="settings-warning"><TriangleAlert size={14} strokeWidth={1.5} aria-hidden="true" />Store this once. It expires {state.recoveryCode.expires_at} and can reset the account password.</div>
            <button type="button" class="settings-copy-btn" on:click={() => actions.copyText(state.recoveryCode?.code ?? "", "Recovery code")}>
              <Copy size={15} strokeWidth={1.5} aria-hidden="true" />Copy recovery code
            </button>
          {:else}
            <button type="button" class="settings-copy-btn" on:click={actions.createRecoveryCode} disabled={state.busy}>
              <Copy size={15} strokeWidth={1.5} aria-hidden="true" />Create recovery code
            </button>
          {/if}
        </div>
      {/if}
    </div>
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Manager invitations · Owner only</div>
    <div class="settings-group-card">
      <div class="settings-row no-tap">
        <div class="settings-row-icon si-violet"><Users size={17} strokeWidth={1.5} aria-hidden="true" /></div>
        <div class="settings-row-body">
          <div class="settings-row-title">Invite a manager</div>
          <div class="settings-row-desc">Managers can add and manage content but cannot change setup or invite others.</div>
        </div>
      </div>

      {#if state.invitation}
        <div class="settings-invite-item">
          <div class="settings-invite-icon"><Link size={17} strokeWidth={1.5} aria-hidden="true" /></div>
          <div class="settings-invite-body">
            <div class="settings-invite-code">{state.invitation.code}</div>
            <div class="settings-invite-meta">Expires {state.invitation.expires_at} · single use</div>
          </div>
          <div class="settings-row-right">
            <button type="button" class="settings-square-btn violet" aria-label="Copy invite link" on:click={() => actions.copyText(invitationUrl(state.invitation?.code ?? ""), "Invitation link")}>
              <Copy size={15} strokeWidth={1.5} aria-hidden="true" />
            </button>
            <button type="button" class="settings-square-btn" aria-label="Dismiss invitation" on:click={actions.dismissInvitation}>
              <X size={15} strokeWidth={1.5} aria-hidden="true" />
            </button>
          </div>
        </div>
      {/if}

      <button type="button" class="settings-new-invite-btn" on:click={actions.createManagerInvitation} disabled={state.busy || !state.isOwner || !state.setup?.device_id}>
        <Plus size={16} strokeWidth={1.5} aria-hidden="true" />Create new invitation link
      </button>
    </div>
  </section>

  <section class="settings-group">
    <div class="settings-group-label">Danger zone</div>
    <div class="settings-danger-card">
      <div class="settings-danger-header">
        <TriangleAlert size={16} strokeWidth={1.5} aria-hidden="true" />
        <div>Irreversible actions</div>
      </div>
      <div class="settings-danger-row">
        <div class="settings-danger-body">
          <div class="settings-danger-title">Clear all unused content</div>
          <div class="settings-danger-desc">Removes audio no longer used by the current button setup. Active setup content and drafts stay available.</div>
        </div>
        <button type="button" class="settings-danger-btn" on:click={actions.clearAllUnusedContent} disabled={state.busy || !state.isOwner || state.totalUnused === 0}>
          Clear unused content
        </button>
      </div>
      <div class="settings-danger-row">
        <div class="settings-danger-body">
          <div class="settings-danger-title">Factory reset</div>
          <div class="settings-danger-desc">Reset setup, accounts, and content back to defaults.</div>
        </div>
        <button type="button" class="settings-danger-btn" on:click={actions.openFactoryResetPrompt} disabled={state.busy || !state.isOwner}>
          Factory reset
        </button>
      </div>
    </div>
  </section>

  <div class="settings-logout-row">
    <button type="button" class="settings-logout-btn" on:click={actions.logout} disabled={state.busy}>
      <LogOut size={17} strokeWidth={1.5} aria-hidden="true" />Sign out of this device
    </button>
  </div>

  <div class="settings-version-footer">
    tcube-pi · {state.status?.service ?? "admin"}<br />
    <span>{state.status?.hostname ?? "host pending"} · {state.status?.mode ?? "local"}</span>
  </div>
</div>

{#if state.factoryResetPromptOpen}
  <div class="trash-backdrop" role="presentation" on:click={actions.cancelFactoryReset}>
    <div
      class="trash-dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="factory-reset-dialog-title"
      aria-describedby="factory-reset-dialog-desc"
      on:click|stopPropagation
      on:keydown={(event) => {
        if (event.key === "Escape") actions.cancelFactoryReset();
      }}
    >
      <div class="trash-dialog-icon"><TriangleAlert size={22} strokeWidth={1.5} aria-hidden="true" /></div>
      <div class="trash-dialog-body">
        <div id="factory-reset-dialog-title" class="trash-dialog-title">Factory reset this cube?</div>
        <div id="factory-reset-dialog-desc" class="trash-dialog-desc">This deletes setup, accounts, sessions, activity, drafts, and parent-created audio. Type FACTORY RESET to continue.</div>
        <label class="field-label factory-reset-label">Confirmation
          <input
            class="settings-input"
            aria-label="Factory reset confirmation"
            value={state.factoryResetConfirmation}
            autocomplete="off"
            placeholder="FACTORY RESET"
            on:input={(event) => actions.setFactoryResetConfirmation((event.currentTarget as HTMLInputElement).value)}
          />
        </label>
      </div>
      <div class="trash-dialog-actions">
        <button type="button" class="btn-secondary" on:click={actions.cancelFactoryReset} disabled={state.busy}>Cancel</button>
        <button type="button" class="btn-primary trash-confirm" on:click={actions.confirmFactoryReset} disabled={state.busy || state.factoryResetConfirmation !== "FACTORY RESET"}>
          <X size={16} strokeWidth={1.5} aria-hidden="true" />Factory reset
        </button>
      </div>
    </div>
  </div>
{/if}
