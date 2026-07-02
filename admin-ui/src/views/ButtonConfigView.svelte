<script lang="ts">
  import { TriangleAlert, Check, Languages, Minus, Music, PawPrint, Play, SlidersHorizontal, Upload, Wrench } from "@lucide/svelte";
  import type { ActiveContentItem, AuthSession, ButtonMode } from "../api";
  import type { RecordedWav } from "../audio";
  import type { ButtonConfigViewModel } from "../button-config-controller";
  import type { DraftForm } from "../types";
  import TopBar from "../components/TopBar.svelte";
  import ContentAddTabs from "../components/ContentAddTabs.svelte";
  import ContentDraftQueue from "../components/ContentDraftQueue.svelte";
  import ContentList from "../components/ContentList.svelte";
  import { contentLabel, faceName, modeClass } from "../view-utils";
  import { modeLabel } from "../button-mode";

  export let state: ButtonConfigViewModel & {
    session: AuthSession | null;
    recorder: MediaRecorder | null;
    recordingStatus: "idle" | "recording" | "processing" | "ready" | "saving";
    recordSeconds: number;
    recordWaveform: number[];
    recordedWav: RecordedWav | null;
    uploadFile: File | null;
    uploadPreviewUrl: string | null;
    draggingUpload: boolean;
    generatedSpeechStatusLoading: boolean;
    generatedSpeechStatusError: string | null;
    generatedSpeechVoiceOptions: string[];
    trashPrompt: { id: string; title: string } | null;
  };

  export let actions: {
    goHome: () => void;
    openSettings: () => void;
    setSelectedButtonId: (id: number) => void;
    setContentListTab: (tab: "active" | "draft") => void;
    setContentTab: (tab: "record" | "upload" | "generate") => void;
    setSelectedMode: (mode: ButtonMode) => void;
    setSelectedLanguage: (language: string) => void;
    updateDraftForm: (patch: Partial<DraftForm>) => void;
    saveSelectedButtonMode: () => void | Promise<void>;
    activateSelectedContent: (id: string) => void | Promise<void>;
    trashSelectedContent: (id: string) => void | Promise<void>;
    startRecording: () => void | Promise<void>;
    stopRecording: () => void;
    revokeRecording: () => void;
    submitRecording: () => void | Promise<void>;
    chooseUpload: (event: Event) => void;
    clearUpload: () => void;
    dropUpload: (event: DragEvent) => void;
    setUploadDragging: (dragging: boolean) => void;
    submitUpload: () => void | Promise<void>;
    submitGeneration: () => void | Promise<void>;
    playContentPreview: (item: ActiveContentItem | { id: string; title: string; preview_url: string | null }) => void | Promise<void>;
    promptTrashContent: (item: { id: string; title: string }) => void;
    cancelTrashContent: () => void;
    confirmTrashContent: () => void | Promise<void>;
  };
</script>

<TopBar
  session={state.session}
  roleLabel={state.session?.cubes?.[0]?.role === "owner" ? "owner" : state.session?.cubes?.[0]?.role === "manager" ? "manager" : state.session?.cubes?.[0]?.role || "member"}
  roleClass={state.session?.cubes?.[0]?.role === "owner" ? "owner" : state.session?.cubes?.[0]?.role === "manager" ? "admin" : "member"}
  showBack={true}
  goHome={actions.goHome}
  goBack={actions.goHome}
  openSettings={actions.openSettings}
/>

<div class="face-strip-wrap">
  <p class="face-strip-label">Select a button</p>
  <div class="face-strip" role="tablist" aria-label="Cube faces" data-testid="button-selector">
    {#each state.buttons as button}
      <button
        type="button"
        class:selected={state.selectedButtonId === button.id}
        class="face-pill"
        data-testid={`button-selector-${button.id}`}
        role="tab"
        aria-selected={state.selectedButtonId === button.id}
        on:click={() => actions.setSelectedButtonId(button.id)}
      >
        <div class="face-pill-icon fpi-{modeClass(button.mode)}" data-testid={`button-selector-${button.id}-icon`}>
          {#if button.mode === "language"}
            <Languages size={16} strokeWidth={1.5} aria-hidden="true" />
          {:else if button.mode === "animals"}
            <PawPrint size={16} strokeWidth={1.5} aria-hidden="true" />
          {:else if button.mode === "music"}
            <Music size={16} strokeWidth={1.5} aria-hidden="true" />
          {:else if button.mode === "setup_help"}
            <Wrench size={16} strokeWidth={1.5} aria-hidden="true" />
          {:else}
            <Minus size={16} strokeWidth={1.5} aria-hidden="true" />
          {/if}
        </div>
        <div class="face-pill-name">{faceName(button.id)}</div>
        <div class="face-pill-count">{button.contentType ? `${button.activeCount} active` : contentLabel(button.mode, button.language)}</div>
      </button>
    {/each}
  </div>
</div>

<div class="body config-body">
  {#if state.selectedButton}
    <section class="section-card">
      <div class="face-hero">
        <div class="face-hero-icon fpi-{modeClass(state.selectedButton.mode)}" data-testid="selected-button-hero-icon">
          {#if state.selectedButton.mode === "language"}
            <Languages size={22} strokeWidth={1.5} aria-hidden="true" />
          {:else if state.selectedButton.mode === "animals"}
            <PawPrint size={22} strokeWidth={1.5} aria-hidden="true" />
          {:else if state.selectedButton.mode === "music"}
            <Music size={22} strokeWidth={1.5} aria-hidden="true" />
          {:else if state.selectedButton.mode === "setup_help"}
            <Wrench size={22} strokeWidth={1.5} aria-hidden="true" />
          {:else}
            <Minus size={22} strokeWidth={1.5} aria-hidden="true" />
          {/if}
        </div>
        <div class="face-hero-info">
          <div class="face-hero-name">{faceName(state.selectedButton.id)} · {state.selectedButton.mode === "language" ? "Language" : contentLabel(state.selectedButton.mode, state.selectedButton.language)}</div>
          <div class="face-hero-sub">{state.selectedButton.mode === "language" ? `${state.selectedButton.language} · ` : ""}Button {state.selectedButton.id}</div>
        </div>
        <div class:active-badge={Boolean(state.selectedButton.contentType)} class:disabled-badge={!state.selectedButton.contentType}>
          <span class="active-dot"></span>
          {state.selectedButton.contentType ? "Active" : "No content"}
        </div>
      </div>
      <div class="stats-row">
        <div class="stat-cell">
          <div class="stat-num stat-active">{state.selectedButton.activeCount}</div>
          <div class="stat-lbl">Active</div>
        </div>
        <div class="stat-cell">
          <div class="stat-num stat-draft">{state.selectedButton.draftCount}</div>
          <div class="stat-lbl">Draft</div>
        </div>
        <div class="stat-cell">
          <div class="stat-num">{state.events.filter((event) => event.button_id === state.selectedButton?.id).length}</div>
          <div class="stat-lbl">Recent plays</div>
        </div>
      </div>
    </section>

    <section class="section-card">
      <div class="sc-header">
        <div class="sc-title"><SlidersHorizontal size={16} strokeWidth={1.5} aria-hidden="true" />Mode</div>
      </div>
      <div class="mode-grid" role="radiogroup" aria-label="Button mode">
        {#each ["language", "animals", "music", "setup_help", "disabled"] as mode, index}
          <button
            type="button"
            class:selected-mode={state.selectedButton.mode === mode}
            class:mode-cell-5th={index === 4}
            class:mode-cell={index !== 4}
            data-testid={`button-mode-${mode}`}
            role="radio"
            aria-checked={state.selectedButton.mode === mode}
            on:click={() => actions.setSelectedMode(mode as ButtonMode)}
          >
            {#if mode === "language"}
              <Languages size={18} strokeWidth={1.5} aria-hidden="true" />
            {:else if mode === "animals"}
              <PawPrint size={18} strokeWidth={1.5} aria-hidden="true" />
            {:else if mode === "music"}
              <Music size={18} strokeWidth={1.5} aria-hidden="true" />
            {:else if mode === "setup_help"}
              <Wrench size={18} strokeWidth={1.5} aria-hidden="true" />
            {:else}
              <Minus size={18} strokeWidth={1.5} aria-hidden="true" />
            {/if}
            {modeLabel(mode as ButtonMode)}
          </button>
        {/each}
      </div>
      {#if state.selectedButton.mode === "language"}
        <div class="lang-pad">
          <label class="field-label">Language
            <select class="lang-select" value={state.selectedButton.language} aria-label="Select language for this button" on:change={(event) => actions.setSelectedLanguage((event.currentTarget as HTMLSelectElement).value)}>
              {#each ["English", "French", "Vietnamese", "Spanish", "German", "Italian", "Portuguese", "Dutch", "Arabic", "Hindi"] as language}
                <option value={language}>{language}</option>
              {/each}
            </select>
          </label>
        </div>
      {/if}
      <div class="mode-save-row">
        <div class="mode-save-note">Changes apply on the next button press.</div>
        <button type="button" class="btn-primary mode-save-btn" on:click={actions.saveSelectedButtonMode} disabled={state.busy}>
          <Check size={16} strokeWidth={1.5} aria-hidden="true" />Save mode
        </button>
      </div>
    </section>

      {#if state.selectedButton.contentType}
        <section class="section-card">
          <div class="content-tabs" role="tablist">
            <button type="button" class:active-tab={state.contentListTab === "active"} class="ctab" role="tab" aria-selected={state.contentListTab === "active"} on:click={() => actions.setContentListTab("active")}>
              Active <span class="ctab-count cc-active">{state.selectedContent?.active.length ?? 0}</span>
            </button>
            <button type="button" class:active-tab={state.contentListTab === "draft"} class="ctab" role="tab" aria-selected={state.contentListTab === "draft"} on:click={() => actions.setContentListTab("draft")}>
              Drafts <span class="ctab-count cc-draft">{state.selectedContent?.inactive.length ?? 0}</span>
            </button>
          </div>
          {#if state.selectedContent?.error}
            <div class="content-api-error" role="alert">
              <TriangleAlert size={15} strokeWidth={1.5} aria-hidden="true" />
              <span>{state.selectedContent.error}</span>
            </div>
          {/if}
          {#if state.contentListTab === "active"}
            <ContentList
              items={state.selectedContent?.active ?? []}
              loading={Boolean(state.selectedContent?.loading)}
              error={state.selectedContent?.error ?? null}
              busy={state.busy}
              contentDurations={state.contentDurations}
              onPreview={actions.playContentPreview}
              onTrash={actions.promptTrashContent}
            />
          {:else}
            <ContentDraftQueue
              items={state.selectedContent?.inactive ?? []}
              loading={Boolean(state.selectedContent?.loading)}
              error={state.selectedContent?.error ?? null}
              busy={state.busy}
              activate={actions.activateSelectedContent}
              trash={actions.trashSelectedContent}
              preview={actions.playContentPreview}
            />
          {/if}
        </section>

      <section class="section-card">
        <div class="sc-header">
          <div class="sc-title"><Play size={16} strokeWidth={1.5} aria-hidden="true" />Add content</div>
          <div class="sc-meta">saves as draft</div>
        </div>
        <ContentAddTabs
          selectedTab={state.selectedTab}
          setTab={actions.setContentTab}
          selectedButton={state.selectedButton}
          draftForm={state.draftForm}
          updateDraftForm={actions.updateDraftForm}
          providers={["auto", "voxtral", "vietnamese-vits"]}
          busy={state.busy}
          recorder={state.recorder}
          recordSeconds={state.recordSeconds}
          recordedWav={state.recordedWav}
          uploadFile={state.uploadFile}
          uploadPreviewUrl={state.uploadPreviewUrl}
          startRecording={actions.startRecording}
          stopRecording={actions.stopRecording}
          revokeRecording={actions.revokeRecording}
          submitRecording={actions.submitRecording}
          chooseUpload={actions.chooseUpload}
          clearUpload={actions.clearUpload}
          dropUpload={actions.dropUpload}
          setUploadDragging={actions.setUploadDragging}
          submitUpload={actions.submitUpload}
          submitGeneration={actions.submitGeneration}
          minutes={(seconds) => `${Math.floor(seconds / 60)}:${Math.floor(seconds % 60).toString().padStart(2, "0")}`}
          recordingStatus={state.recordingStatus}
          recordWaveform={state.recordWaveform}
          generatedSpeechDisabled={state.generatedSpeechDisabled}
          generatedSpeechStatusLoading={state.generatedSpeechStatusLoading}
          generatedSpeechStatusError={state.generatedSpeechStatusError}
          voiceOptions={state.generatedSpeechVoiceOptions}
          draggingUpload={state.draggingUpload}
        />
      </section>
    {:else}
      <section class="section-card empty-state">
        <Minus size={24} strokeWidth={1.5} aria-hidden="true" />
        <strong>No content lane</strong>
        <p>Set this button to Language, Animals, or Music before adding active content or drafts.</p>
      </section>
    {/if}
  {/if}
</div>

{#if state.trashPrompt}
  <div class="trash-backdrop" role="presentation" on:click={actions.cancelTrashContent}>
    <div
      class="trash-dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="trash-dialog-title"
      aria-describedby="trash-dialog-desc"
      on:click|stopPropagation
      on:keydown={(event) => {
        if (event.key === "Escape") actions.cancelTrashContent();
      }}
    >
      <div class="trash-dialog-icon"><TriangleAlert size={22} strokeWidth={1.5} aria-hidden="true" /></div>
      <div class="trash-dialog-body">
        <div id="trash-dialog-title" class="trash-dialog-title">Move audio to trash?</div>
        <div id="trash-dialog-desc" class="trash-dialog-desc">{state.trashPrompt.title} will be removed from the active list and deleted from disk.</div>
      </div>
      <div class="trash-dialog-actions">
        <button type="button" class="btn-secondary" on:click={actions.cancelTrashContent}>Cancel</button>
        <button type="button" class="btn-primary trash-confirm" on:click={actions.confirmTrashContent}>
          <TriangleAlert size={16} strokeWidth={1.5} aria-hidden="true" />Move to trash
        </button>
      </div>
    </div>
  </div>
{/if}
