<script lang="ts">
  import { onMount } from "svelte";
  import {
    acceptInvitation,
    activateContentItem,
    bootstrapOwner,
    clearUnusedGeneratedSpeech,
    completeSetup,
    createInvitation,
    createRecoveryCode,
    generateSpeech,
    getSession,
    getSetupReview,
    getStatus,
    listActiveContent,
    listInactiveContent,
    loginPassword,
    logout,
    recoverPassword,
    saveButtonMode,
    saveCubeName,
    saveMultipart,
    trashContentItem,
    verifyWifi
  } from "./api";
  import type {
    AuthSession,
    ButtonMode,
    ContentType,
    ServiceStatus,
    SetupReview
  } from "./api";
  import { blobToWav, canRecordAudio, isSecureRecorderContext } from "./audio";
  import type { RecordedWav } from "./audio";
  import { contentTypeForMode, defaultMode, splitMode } from "./button-mode";
  import ButtonConfigPanel from "./components/ButtonConfigPanel.svelte";
  import ButtonFaceCard from "./components/ButtonFaceCard.svelte";
  import NeoCard from "./components/NeoCard.svelte";
  import TerminalNotice from "./components/TerminalNotice.svelte";
  import TopBanner from "./components/TopBanner.svelte";
  import type { ButtonConfig, ContentState } from "./types";

  const modes: ButtonMode[] = ["language", "animals", "music", "setup_help", "disabled"];
  const contentTypes: ContentType[] = ["language", "animals", "music"];
  const languages = [
    "English",
    "French",
    "Vietnamese",
    "Spanish",
    "German",
    "Italian",
    "Portuguese",
    "Dutch",
    "Arabic",
    "Hindi"
  ];
  const providers = ["auto", "local", "hosted"];

  let status: ServiceStatus | null = null;
  let session: AuthSession | null = null;
  let setup: SetupReview | null = null;
  let loading = true;
  let busy = false;
  let message = "Loading T-Cube admin state.";
  let messageType: "info" | "success" | "error" = "info";

  let bootstrapForm = { username: "parent", display_name: "Parent Admin", password: "" };
  let loginForm = { username: "", password: "" };
  let recoveryForm = { code: "", password: "" };
  let inviteForm = { code: "", username: "", display_name: "", password: "" };
  let cubeName = "T-Cube";
  let wifiForm = { ssid: "", dashboard_ip: "" };
  let recoveryCode: { code: string; expires_at: string } | null = null;
  let invitation: { code: string; expires_at: string } | null = null;

  let selectedButtonId = 1;
  let selectedTab: "record" | "upload" | "generate" = "record";
  let contentState: Record<string, ContentState> = {};

  let draftForm = {
    title: "",
    text: "",
    language: "English",
    provider: "auto",
    voice: ""
  };
  let uploadFile: File | null = null;
  let uploadPreviewUrl: string | null = null;
  let recordedWav: RecordedWav | null = null;
  let recorder: MediaRecorder | null = null;
  let recordStartedAt = 0;
  let recordSeconds = 0;
  let recordTimer: number | null = null;

  $: buttons = buildButtonConfigs(setup);
  $: manageableButtons = buttons.filter((button) => button.contentType !== null);
  $: selectedButton =
    manageableButtons.find((button) => button.id === selectedButtonId) ?? manageableButtons[0] ?? null;
  $: selectedContent = selectedButton ? contentState[contentKey(selectedButton)] : null;
  $: currentRole = session?.cubes?.[0]?.role ?? "";
  $: isOwner = currentRole === "owner";
  $: invitationCodeFromUrl = new URLSearchParams(window.location.search).get("invite") ?? "";
  $: prerequisites = [
    { label: "Owner or manager account exists", complete: Boolean(setup?.admin_created) },
    { label: "Cube name saved", complete: Boolean(setup?.cube_name?.trim()) },
    { label: "Wi-Fi verified", complete: Boolean(setup?.wifi_verified) },
    { label: "Language content active", complete: count("language") > 0 },
    { label: "Animal content active", complete: count("animals") > 0 },
    { label: "Music content active", complete: count("music") > 0 }
  ];

  onMount(async () => {
    inviteForm.code = invitationCodeFromUrl;
    await refreshAll();
  });

  async function refreshAll() {
    loading = true;
    try {
      const [nextStatus, nextSession, nextSetup] = await Promise.all([
        getStatus(),
        getSession(),
        getSetupReview()
      ]);
      status = nextStatus;
      session = nextSession;
      setup = nextSetup;
      cubeName = setup.cube_name || "T-Cube";
      wifiForm.dashboard_ip = setup.dashboard_ip ?? "";
      setSuccess("Admin state refreshed.");
      await refreshVisibleContent();
    } catch (error) {
      setError(error);
    } finally {
      loading = false;
    }
  }

  async function run(action: () => Promise<unknown>, success: string) {
    busy = true;
    try {
      await action();
      setSuccess(success);
      await refreshAll();
    } catch (error) {
      setError(error);
    } finally {
      busy = false;
    }
  }

  async function refreshVisibleContent() {
    if (!setup) return;
    await Promise.all(buildButtonConfigs(setup).filter((button) => button.contentType).map(refreshContent));
  }

  async function refreshContent(button: ButtonConfig) {
    if (!button.contentType) return;
    const key = contentKey(button);
    contentState = {
      ...contentState,
      [key]: { active: [], inactive: [], loading: true, error: null }
    };
    try {
      const [active, inactive] = await Promise.all([
        listActiveContent(button.id, button.contentType, button.language),
        listInactiveContent(button.id, button.contentType, button.language)
      ]);
      contentState = {
        ...contentState,
        [key]: { active, inactive, loading: false, error: null }
      };
    } catch (error) {
      contentState = {
        ...contentState,
        [key]: { active: [], inactive: [], loading: false, error: errorText(error) }
      };
    }
  }

  async function startRecording() {
    if (!isSecureRecorderContext()) {
      setError("Browser recording requires HTTPS or localhost.");
      return;
    }
    if (!canRecordAudio()) {
      setError("This browser does not expose microphone recording APIs.");
      return;
    }
    revokeRecording();
    try {
      const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const chunks: BlobPart[] = [];
      recorder = new MediaRecorder(stream);
      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) chunks.push(event.data);
      };
      recorder.onstop = async () => {
        stream.getTracks().forEach((track) => track.stop());
        try {
          recordedWav = await blobToWav(new Blob(chunks, { type: recorder?.mimeType || "audio/webm" }));
          setSuccess("Recording ready for review.");
        } catch (error) {
          setError(error);
        } finally {
          stopTimer();
          recorder = null;
        }
      };
      recorder.start();
      recordStartedAt = Date.now();
      recordSeconds = 0;
      recordTimer = window.setInterval(() => {
        recordSeconds = Math.floor((Date.now() - recordStartedAt) / 1000);
      }, 250);
    } catch (error) {
      setError(error);
    }
  }

  function stopRecording() {
    recorder?.stop();
  }

  function stopTimer() {
    if (recordTimer !== null) {
      window.clearInterval(recordTimer);
      recordTimer = null;
    }
  }

  function revokeRecording() {
    if (recordedWav) URL.revokeObjectURL(recordedWav.url);
    recordedWav = null;
    recordSeconds = 0;
  }

  function chooseUpload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    uploadFile = input.files?.[0] ?? null;
    if (uploadPreviewUrl) URL.revokeObjectURL(uploadPreviewUrl);
    uploadPreviewUrl = uploadFile ? URL.createObjectURL(uploadFile) : null;
  }

  function mediaForm(file: Blob, filename: string) {
    if (!selectedButton?.contentType) {
      throw new Error("Choose an active content button first.");
    }
    const form = new FormData();
    form.set("content_type", selectedButton.contentType);
    form.set("button_id", String(selectedButton.id));
    form.set("title", draftForm.title.trim());
    form.set("text", draftForm.text.trim());
    form.set("language", selectedButton.contentType === "language" ? selectedButton.language : draftForm.language);
    form.set("audio_file", file, filename);
    return form;
  }

  async function submitRecording() {
    if (!recordedWav) {
      setError("Record and preview audio before upload.");
      return;
    }
    const wav = recordedWav;
    await run(async () => {
      await saveMultipart("/api/content/recordings", mediaForm(wav.blob, "recording.wav"));
      revokeRecording();
    }, "Recording saved to inactive review.");
  }

  async function submitUpload() {
    if (!uploadFile) {
      setError("Choose an MP3 or WAV file first.");
      return;
    }
    const file = uploadFile;
    await run(async () => {
      await saveMultipart("/api/content/uploads", mediaForm(file, file.name));
      uploadFile = null;
      if (uploadPreviewUrl) URL.revokeObjectURL(uploadPreviewUrl);
      uploadPreviewUrl = null;
    }, "Upload saved to inactive review.");
  }

  async function createManagerInvitation() {
    const deviceId = setup?.device_id;
    if (!deviceId) {
      throw new Error("Save the cube name before creating a manager invitation.");
    }
    invitation = await createInvitation(deviceId);
  }

  async function submitGeneration() {
    if (!selectedButton) return;
    await run(async () => {
      await generateSpeech({
        button_id: selectedButton.id,
        language: selectedButton.language,
        text: draftForm.text,
        provider: draftForm.provider,
        voice: draftForm.voice.trim() || undefined
      });
    }, "Generated speech saved to inactive review.");
  }

  function buildButtonConfigs(review: SetupReview | null): ButtonConfig[] {
    return [1, 2, 3, 4, 5].map((id) => {
      const raw = review?.button_modes?.[String(id)] ?? defaultMode(id);
      const { mode, language } = splitMode(raw);
      return {
        id,
        mode,
        language,
        contentType: contentTypeForMode(mode)
      };
    });
  }

  function contentKey(button: ButtonConfig) {
    return `${button.id}:${button.contentType ?? "none"}:${button.language}`;
  }

  function count(type: ContentType) {
    return setup?.active_counts?.[type] ?? 0;
  }

  function setSuccess(text: string) {
    message = text;
    messageType = "success";
  }

  function setError(error: unknown) {
    message = errorText(error);
    messageType = "error";
  }

  function errorText(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function fmt(value: unknown) {
    if (value === null || value === undefined || value === "") return "Not set";
    if (typeof value === "boolean") return value ? "Yes" : "No";
    return String(value);
  }

  function minutes(seconds: number) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60).toString().padStart(2, "0");
    return `${mins}:${secs}`;
  }

  function selectButton(id: number) {
    selectedButtonId = id;
  }

  async function saveSelectedButtonMode(button: ButtonConfig) {
    await run(() => saveButtonMode(button.id, button.mode, button.language), `Button ${button.id} saved.`);
  }

  async function trashSelectedContent(id: string) {
    await run(() => trashContentItem(id), "Content moved to trash.");
  }

  async function activateSelectedContent(id: string) {
    await run(() => activateContentItem(id), "Draft activated.");
  }

  async function clearSelectedGenerated() {
    if (!selectedButton) return;
    await run(
      () => clearUnusedGeneratedSpeech(selectedButton.id, selectedButton.language),
      "Unused generated speech moved to trash."
    );
  }
</script>

<svelte:head>
  <title>T-Cube Pi Admin</title>
</svelte:head>

<main class="admin-shell">
  <div class="page-haze" aria-hidden="true"></div>
  <div class="scanlines" aria-hidden="true"></div>

  <div class="admin-frame">
    <TopBanner {status} {setup} {loading} {busy} refresh={refreshAll} />
    <TerminalNotice type={messageType} {message} />

  {#if invitationCodeFromUrl && !session?.authenticated}
    <section class="neo-surface neo-card accent-panel">
      <div class="section-head">
        <div>
          <p class="terminal-kicker">Invitation</p>
          <h2>Accept manager access</h2>
        </div>
      </div>
      <form class="form-grid" on:submit|preventDefault={() => run(async () => (session = await acceptInvitation(inviteForm)), "Invitation accepted.")}>
        <label>Invitation code <input class="neo-field" bind:value={inviteForm.code} autocomplete="off" /></label>
        <label>Username <input class="neo-field" bind:value={inviteForm.username} autocomplete="username" /></label>
        <label>Display name <input class="neo-field" bind:value={inviteForm.display_name} /></label>
        <label>Password <input class="neo-field" bind:value={inviteForm.password} type="password" autocomplete="new-password" /></label>
        <button type="submit" class="neo-button" disabled={busy}>Create manager account</button>
      </form>
    </section>
  {/if}

    <section class="dashboard-layout">
      <aside class="side-rail">
        <NeoCard kicker="Account" title="Access">
      {#if session?.authenticated && session.account}
        <p class="identity">{session.account.display_name}</p>
            <p class="muted">@{session.account.username} · {currentRole || "member"}</p>
        <div class="button-row">
              <button type="button" class="neo-button secondary" on:click={() => run(async () => (recoveryCode = await createRecoveryCode()), "Recovery code created.")} disabled={busy}>Recovery code</button>
              <button type="button" class="neo-button" on:click={() => run(logout, "Logged out.")} disabled={busy}>Log out</button>
        </div>
        {#if recoveryCode}
          <div class="secret-box">
            <strong>{recoveryCode.code}</strong>
            <span>Expires {recoveryCode.expires_at}</span>
          </div>
        {/if}
      {:else if session?.bootstrap_required}
        <p class="muted">No owner exists yet. Create the first local owner to continue setup.</p>
        <form class="stack" on:submit|preventDefault={() => run(async () => (session = await bootstrapOwner(bootstrapForm)), "First owner created.")}>
              <label>Username <input class="neo-field" bind:value={bootstrapForm.username} autocomplete="username" /></label>
              <label>Display name <input class="neo-field" bind:value={bootstrapForm.display_name} /></label>
              <label>Password <input class="neo-field" bind:value={bootstrapForm.password} type="password" autocomplete="new-password" /></label>
              <button type="submit" class="neo-button" disabled={busy}>Create first owner</button>
        </form>
      {:else}
        <p class="muted">Sign in with a local cube account.</p>
        <form class="stack" on:submit|preventDefault={() => run(async () => (session = await loginPassword(loginForm)), "Logged in.")}>
              <label>Username <input class="neo-field" bind:value={loginForm.username} autocomplete="username" /></label>
              <label>Password <input class="neo-field" bind:value={loginForm.password} type="password" autocomplete="current-password" /></label>
              <button type="submit" class="neo-button" disabled={busy}>Log in</button>
        </form>
        <details class="recovery">
          <summary>Recover password</summary>
          <form class="stack" on:submit|preventDefault={() => run(() => recoverPassword(recoveryForm), "Password reset. Log in with the new password.")}>
                <label>Recovery code <input class="neo-field" bind:value={recoveryForm.code} autocomplete="off" /></label>
                <label>New password <input class="neo-field" bind:value={recoveryForm.password} type="password" autocomplete="new-password" /></label>
                <button type="submit" class="neo-button" disabled={busy}>Reset password</button>
          </form>
        </details>
      {/if}
        </NeoCard>

        <NeoCard kicker="Setup" title="Prerequisites">
      <ul class="check-list">
        {#each prerequisites as item}
              <li class:complete={item.complete}><span>{item.complete ? "OK" : "!"}</span>{item.label}</li>
        {/each}
      </ul>
          <button type="button" class="neo-button" on:click={() => run(completeSetup, "Setup marked complete.")} disabled={busy || !isOwner}>Complete setup</button>
      {#if !isOwner && session?.authenticated}
        <p class="hint">Owner role required for setup completion.</p>
      {/if}
        </NeoCard>

        <NeoCard kicker="Cube" title="Name and network">
      <form class="stack" on:submit|preventDefault={() => run(() => saveCubeName(cubeName), "Cube name saved.")}>
            <label>Cube name <input class="neo-field" bind:value={cubeName} /></label>
            <button type="submit" class="neo-button" disabled={busy || !isOwner}>Save cube name</button>
      </form>
      <form class="stack" on:submit|preventDefault={() => run(() => verifyWifi(wifiForm.ssid, wifiForm.dashboard_ip), "Wi-Fi marked verified.")}>
            <label>Wi-Fi SSID <input class="neo-field" bind:value={wifiForm.ssid} placeholder="Home Wi-Fi" /></label>
            <label>Dashboard IP <input class="neo-field" bind:value={wifiForm.dashboard_ip} placeholder="192.168.1.10" /></label>
            <button type="submit" class="neo-button secondary" disabled={busy || !isOwner}>Mark Wi-Fi verified</button>
      </form>
        </NeoCard>

        <NeoCard kicker="Owner tools" title="Manager invite">
      <p class="muted">Invitations target this cube and create manager access.</p>
          <button type="button" class="neo-button" on:click={() => run(createManagerInvitation, "Manager invitation created.")} disabled={busy || !isOwner || !setup?.device_id}>Create invitation</button>
      {#if invitation}
        <div class="secret-box">
          <strong>{invitation.code}</strong>
          <span>Expires {invitation.expires_at}</span>
        </div>
      {/if}
        </NeoCard>

        <NeoCard kicker="Runtime" title="Service facts">
          <dl class="facts compact">
            <dt>Service</dt><dd>{fmt(status?.service)}</dd>
            <dt>Mode</dt><dd>{fmt(status?.mode)}</dd>
            <dt>Dashboard</dt><dd>{fmt(setup?.dashboard_address)}</dd>
            <dt>USB</dt><dd>{fmt(status?.usb_address)}</dd>
          </dl>
        </NeoCard>
      </aside>

      <section class="main-deck">
        <section class="neo-surface button-array">
          <div class="button-array-header">
            <p class="button-array-kicker">Five-face setup matrix</p>
            <h2 class="button-array-title">Cube button configuration</h2>
          </div>
          <div class="button-card-grid">
            {#each buttons as button}
              <ButtonFaceCard
                {button}
                selected={selectedButtonId === button.id}
                activeCount={button.contentType ? count(button.contentType) : 0}
                disabled={!session?.authenticated}
                choose={selectButton}
              />
            {/each}
          </div>
        </section>

        <ButtonConfigPanel
          button={selectedButton}
          content={selectedContent}
          {modes}
          {languages}
          {providers}
          {busy}
          {selectedTab}
          setTab={(tab) => (selectedTab = tab)}
          {draftForm}
          {recorder}
          {recordSeconds}
          {recordedWav}
          {uploadFile}
          {uploadPreviewUrl}
          saveMode={saveSelectedButtonMode}
          activate={activateSelectedContent}
          trash={trashSelectedContent}
          clearGenerated={clearSelectedGenerated}
          {startRecording}
          {stopRecording}
          {revokeRecording}
          {submitRecording}
          {chooseUpload}
          {submitUpload}
          {submitGeneration}
          {minutes}
        />
      </section>
    </section>
  </div>
</main>
