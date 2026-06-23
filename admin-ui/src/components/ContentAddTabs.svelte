<script lang="ts">
  import type { RecordedWav } from "../audio";
  import type { ButtonConfig } from "../types";

  export let selectedTab: "record" | "upload" | "generate";
  export let setTab: (tab: "record" | "upload" | "generate") => void;
  export let selectedButton: ButtonConfig;
  export let draftForm: { title: string; text: string; language: string; provider: string; voice: string };
  export let languages: string[] = [];
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
  export let submitUpload: () => void | Promise<void>;
  export let submitGeneration: () => void | Promise<void>;
  export let minutes: (seconds: number) => string;
</script>

<section class="content-input-surface">
  <div class="content-input-tabs" role="tablist" aria-label="Add content">
    <button
      type="button"
      role="tab"
      class="content-input-tab"
      aria-selected={selectedTab === "record"}
      on:click={() => setTab("record")}
    >
      Record
    </button>
    <button
      type="button"
      role="tab"
      class="content-input-tab"
      aria-selected={selectedTab === "upload"}
      on:click={() => setTab("upload")}
    >
      Upload
    </button>
    <button
      type="button"
      role="tab"
      class="content-input-tab"
      aria-selected={selectedTab === "generate"}
      on:click={() => setTab("generate")}
      disabled={selectedButton.contentType !== "language"}
    >
      Generate
    </button>
  </div>

  <div class="composer-fields">
    <label>Title or label <input class="neo-field" bind:value={draftForm.title} placeholder="Hello baby" /></label>
    <label>Text <input class="neo-field" bind:value={draftForm.text} placeholder={selectedButton.contentType === "music" ? "Optional" : "Short phrase"} /></label>
    <label>Language
      <select class="neo-field" bind:value={draftForm.language} disabled={selectedButton.contentType === "language"}>
        {#each languages as language}
          <option value={language}>{language}</option>
        {/each}
      </select>
    </label>
  </div>

  {#if selectedTab === "record"}
    <div class="record-box">
      <p class="muted">Microphone input is converted to WAV before upload. Recording requires HTTPS or localhost.</p>
      <div class="button-row">
        <button type="button" class="neo-button" on:click={startRecording} disabled={Boolean(recorder) || busy}>Start</button>
        <button type="button" class="neo-button secondary" on:click={stopRecording} disabled={!recorder}>Stop {recordSeconds ? minutes(recordSeconds) : ""}</button>
        <button type="button" class="neo-button secondary" on:click={revokeRecording} disabled={!recordedWav}>Discard</button>
      </div>
      {#if recordedWav}
        <audio controls src={recordedWav.url}></audio>
        <p class="hint">Duration {minutes(recordedWav.durationSeconds)}</p>
        <button type="button" class="neo-button" on:click={submitRecording} disabled={busy}>Upload recording</button>
      {/if}
    </div>
  {:else if selectedTab === "upload"}
    <div class="record-box">
      <input class="neo-field file-field" type="file" accept="audio/mpeg,audio/mp3,audio/wav,.mp3,.wav" on:change={chooseUpload} />
      {#if uploadPreviewUrl}
        <audio controls src={uploadPreviewUrl}></audio>
      {/if}
      <button type="button" class="neo-button" on:click={submitUpload} disabled={busy || !uploadFile}>Upload draft</button>
    </div>
  {:else}
    <form class="composer-fields" on:submit|preventDefault={submitGeneration}>
      <label>Provider
        <select class="neo-field" bind:value={draftForm.provider}>
          {#each providers as provider}
            <option value={provider}>{provider}</option>
          {/each}
        </select>
      </label>
      <label>Voice <input class="neo-field" bind:value={draftForm.voice} placeholder="Optional" /></label>
      <p class="muted composer-note">Generated audio is saved as an inactive draft. Review and activate it before the cube can play it.</p>
      <button type="submit" class="neo-button" disabled={busy || selectedButton.contentType !== "language"}>Generate draft</button>
    </form>
  {/if}
</section>
