<script lang="ts">
  import { Check, FileAudio, Mic, Play, Upload, WandSparkles, X } from "@lucide/svelte";
  import type { RecordedWav } from "../audio";
  import { recordingHint, recordingSaveHint, uploadFileSize, uploadHint, uploadStepLabel } from "../recording-controller";
  import type { ButtonConfig, DraftForm } from "../types";

  export let selectedTab: "record" | "upload" | "generate";
  export let setTab: (tab: "record" | "upload" | "generate") => void;
  export let selectedButton: ButtonConfig;
  export let draftForm: DraftForm;
  export let updateDraftForm: (patch: Partial<DraftForm>) => void;
  export let providers: string[] = [];
  export let busy = false;
  export let recorder: MediaRecorder | null = null;
  export let recordSeconds = 0;
  export let recordedWav: RecordedWav | null = null;
  export let uploadFile: File | null = null;
  export let uploadPreviewUrl: string | null = null;
  export let startRecording: () => void | Promise<void>;
  export let stopRecording: () => void;
  export let revokeRecording: () => void;
  export let submitRecording: () => void | Promise<void>;
  export let chooseUpload: (event: Event) => void;
  export let clearUpload: () => void;
  export let dropUpload: (event: DragEvent) => void;
  export let setUploadDragging: (dragging: boolean) => void;
  export let submitUpload: () => void | Promise<void>;
  export let submitGeneration: () => void | Promise<void>;
  export let minutes: (seconds: number) => string;
  export let recordingStatus: "idle" | "recording" | "processing" | "ready" | "saving";
  export let recordWaveform: number[] = [];
  export let generatedSpeechDisabled = false;
  export let generatedSpeechStatusLoading = false;
  export let generatedSpeechStatusError: string | null = null;
  export let voiceOptions: string[] = [];
  export let draggingUpload = false;

  $: isLanguageButton = selectedButton.contentType === "language";
  $: mediaTitleReady = isLanguageButton || Boolean(draftForm.title.trim());
  $: languageTextReady = !isLanguageButton || Boolean(draftForm.text.trim());
  $: recordingSaveDisabled = busy || !recordedWav || !mediaTitleReady || !languageTextReady;
  $: uploadSaveDisabled = busy || !uploadFile || !mediaTitleReady || !languageTextReady;
  $: uploadReadyToSave = Boolean(uploadFile && mediaTitleReady && languageTextReady);
</script>

<section class="content-input-surface">
  <div class="add-tabs" role="tablist" aria-label="Add content">
    <button
      type="button"
      role="tab"
      class:active-atab={selectedTab === "record"}
      class="atab"
      aria-selected={selectedTab === "record"}
      on:click={() => setTab("record")}
    >
      <Mic size={15} strokeWidth={1.5} aria-hidden="true" />
      Record
    </button>
    <button
      type="button"
      role="tab"
      class:active-atab={selectedTab === "upload"}
      class="atab"
      aria-selected={selectedTab === "upload"}
      on:click={() => setTab("upload")}
    >
      <Upload size={15} strokeWidth={1.5} aria-hidden="true" />
      Upload
    </button>
    <button
      type="button"
      role="tab"
      class:active-atab={selectedTab === "generate"}
      class="atab"
      aria-selected={selectedTab === "generate"}
      on:click={() => setTab("generate")}
      disabled={selectedButton.contentType !== "language"}
    >
      <WandSparkles size={15} strokeWidth={1.5} aria-hidden="true" />
      Generate
    </button>
  </div>

  {#if selectedTab !== "generate" && !isLanguageButton}
    <div class="add-body add-meta-grid">
      <label>Title or label
        <input class="neo-field" value={draftForm.title} placeholder="Roar" on:input={(event) => updateDraftForm({ title: (event.currentTarget as HTMLInputElement).value })} />
      </label>
    </div>
  {/if}

  {#if selectedTab === "record"}
    <div class:recording-active={recordingStatus === "recording"} class:recording-ready={Boolean(recordedWav)} class="record-zone" data-testid="record-zone">
      <button type="button" class:recording={recordingStatus === "recording"} class="record-btn-big" data-testid="record-toggle" on:click={() => (recorder ? stopRecording() : startRecording())} disabled={busy || recordingStatus === "processing" || recordingStatus === "saving"} aria-label={recorder ? "Stop recording" : "Start recording"}>
        {#if recorder}
          <span class="record-stop-dot" aria-hidden="true"></span>
        {:else}
          <Mic size={24} strokeWidth={1.5} aria-hidden="true" />
        {/if}
      </button>
      {#if isLanguageButton}
        <label class="field-label">
          <input class="neo-field" aria-label="Text spoken" value={draftForm.text} placeholder="Write the text spoken here" on:input={(event) => updateDraftForm({ text: (event.currentTarget as HTMLInputElement).value })} />
        </label>
      {/if}
      <div class="record-step" data-testid="record-status">{recordingHint(recordingStatus, recordSeconds, Boolean(recordedWav))}</div>
      {#if recordWaveform.length}
        <div class="record-wave" aria-label="Live microphone level" data-testid="record-waveform">
          {#each recordWaveform as level}
            <span style={`height: ${Math.max(8, Math.round(level * 100))}%`}></span>
          {/each}
        </div>
      {/if}
      <p class="record-hint">Microphone input is converted to WAV before upload. Recording requires HTTPS or localhost.</p>
      <p class="record-hint">After recording, preview the audio here before saving.</p>
      {#if recordedWav}
        <audio controls src={recordedWav.url}></audio>
        <p class="hint">Duration {minutes(recordedWav.durationSeconds)}</p>
        <p class="muted">{recordingSaveHint(selectedButton.contentType, draftForm.text, Boolean(recordedWav))}</p>
        <div class="add-action-row">
          <button type="button" class="btn-secondary" on:click={revokeRecording} disabled={busy}>
            <X size={15} strokeWidth={1.5} aria-hidden="true" />
            Discard
          </button>
          <button type="button" class="btn-primary" on:click={submitRecording} disabled={recordingSaveDisabled}>
            <Play size={15} strokeWidth={1.5} aria-hidden="true" />
            Save recording
          </button>
        </div>
      {/if}
    </div>
  {:else if selectedTab === "upload"}
    <div
      class:dragging={draggingUpload}
      class:upload-ready={Boolean(uploadFile)}
      class="upload-zone"
      data-testid="upload-zone"
      role="group"
      aria-label="Upload audio draft"
      on:dragover|preventDefault={() => setUploadDragging(true)}
      on:dragleave={() => setUploadDragging(false)}
      on:drop={dropUpload}
    >
      <div class="upload-step-row" role="list" aria-label="Upload steps">
        <div class:done={Boolean(uploadFile)} class:current={!uploadFile} class="upload-step" role="listitem">
          {#if uploadFile}<Check size={13} strokeWidth={1.5} aria-hidden="true" />{:else}1{/if}
          <span>Choose</span>
        </div>
        <div class:done={uploadReadyToSave} class:current={Boolean(uploadFile) && !uploadReadyToSave} class="upload-step" role="listitem">
          {#if uploadReadyToSave}<Check size={13} strokeWidth={1.5} aria-hidden="true" />{:else}2{/if}
          <span>Review</span>
        </div>
        <div class:current={uploadReadyToSave} class="upload-step" role="listitem">
          3
          <span>Save Draft</span>
        </div>
      </div>

      <label class="upload-picker" aria-label="Choose audio file">
        <span class="upload-icon-big">
          <FileAudio size={24} strokeWidth={1.5} aria-hidden="true" />
        </span>
        <strong>{uploadStepLabel(uploadFile, uploadReadyToSave)}</strong>
        <span>{uploadHint(uploadFile, uploadReadyToSave)}</span>
        <input class="file-field" type="file" accept="audio/mpeg,audio/mp3,audio/wav,.mp3,.wav" on:change={chooseUpload} />
      </label>

      {#if uploadFile}
        <div class="upload-review">
          <div class="upload-file-row">
            <div class="upload-file-icon">
              <FileAudio size={18} strokeWidth={1.5} aria-hidden="true" />
            </div>
            <div class="upload-file-meta">
              <strong title={uploadFile.name}>{uploadFile.name}</strong>
              <span>{uploadFileSize(uploadFile.size)} · MP3 or WAV</span>
            </div>
            <button type="button" class="upload-clear" on:click={clearUpload} aria-label="Choose another file" disabled={busy}>
              <X size={16} strokeWidth={1.5} aria-hidden="true" />
            </button>
          </div>
          {#if uploadPreviewUrl}
            <audio controls src={uploadPreviewUrl}></audio>
          {/if}
          {#if isLanguageButton}
            <label class="field-label upload-text-field">Text spoken
              <input class="neo-field" value={draftForm.text} placeholder="Short phrase" on:input={(event) => updateDraftForm({ text: (event.currentTarget as HTMLInputElement).value })} />
            </label>
          {/if}
          <p class="muted upload-draft-note">Drafts are not heard by the child until you activate them.</p>
        </div>
      {/if}
      <button type="button" class="btn-primary" on:click={submitUpload} disabled={uploadSaveDisabled}>
        <Upload size={15} strokeWidth={1.5} aria-hidden="true" />
        Save Draft
      </button>
    </div>
  {:else}
    <form class="add-body generate-body" on:submit|preventDefault={submitGeneration}>
      <label class="gen-field">Text to speech
        <input class="neo-field" value={draftForm.text} placeholder="Short phrase" disabled={generatedSpeechDisabled} on:input={(event) => updateDraftForm({ text: (event.currentTarget as HTMLInputElement).value })} />
      </label>
      <div class="gen-row">
        <label class="gen-field">Provider
          <select class="neo-field" value={draftForm.provider} disabled={busy} on:change={(event) => updateDraftForm({ provider: (event.currentTarget as HTMLSelectElement).value })}>
            {#each providers as provider}
              <option value={provider}>{provider}</option>
            {/each}
          </select>
        </label>
        <label class="gen-field">Voice
          {#if voiceOptions.length}
            <select class="neo-field" value={draftForm.voice} disabled={generatedSpeechDisabled} on:change={(event) => updateDraftForm({ voice: (event.currentTarget as HTMLSelectElement).value })}>
              {#each voiceOptions as voice}
                <option value={voice}>{voice}</option>
              {/each}
            </select>
          {:else}
            <span class="voice-placeholder">{generatedSpeechDisabled ? "Unavailable until provider is online." : "Provider default voice will be used."}</span>
          {/if}
        </label>
      </div>
      {#if generatedSpeechStatusError}
        <div class="content-api-error" role="status" data-testid="tts-status-error">{generatedSpeechStatusError}</div>
      {:else if generatedSpeechStatusLoading}
        <div class="content-api-error" role="status" data-testid="tts-status-loading">Checking generated speech service...</div>
      {:else if generatedSpeechDisabled}
        <div class="content-api-error" role="alert" data-testid="tts-offline-notice">TTS provider is offline or unreachable. Start the local TTS service before generating speech.</div>
      {/if}
      <p class="muted composer-note">Generated audio is saved as an inactive draft. Review and activate it before the cube can play it.</p>
      <button type="submit" class="btn-primary" disabled={busy || selectedButton.contentType !== "language" || generatedSpeechDisabled}>
        <WandSparkles size={15} strokeWidth={1.5} aria-hidden="true" />
        Generate speech
      </button>
    </form>
  {/if}
</section>
