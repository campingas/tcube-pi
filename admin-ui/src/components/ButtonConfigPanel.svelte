<script lang="ts">
  import type { ButtonMode } from "../api";
  import { modeLabel } from "../button-mode";
  import type { RecordedWav } from "../audio";
  import type { ButtonConfig, ContentState } from "../types";
  import ContentAddTabs from "./ContentAddTabs.svelte";
  import ContentDraftQueue from "./ContentDraftQueue.svelte";
  import ContentList from "./ContentList.svelte";

  export let button: ButtonConfig | null;
  export let content: ContentState | null;
  export let modes: ButtonMode[] = [];
  export let languages: string[] = [];
  export let providers: string[] = [];
  export let busy = false;
  export let selectedTab: "record" | "upload" | "generate";
  export let draftForm: { title: string; text: string; language: string; provider: string; voice: string };
  export let recorder: MediaRecorder | null = null;
  export let recordSeconds = 0;
  export let recordedWav: RecordedWav | null = null;
  export let uploadFile: File | null = null;
  export let uploadPreviewUrl: string | null = null;
  export let setTab: (tab: "record" | "upload" | "generate") => void;
  export let saveMode: (button: ButtonConfig) => void | Promise<void>;
  export let activate: (id: string) => void | Promise<void>;
  export let trash: (id: string) => void | Promise<void>;
  export let clearGenerated: () => void | Promise<void>;
  export let startRecording: () => void | Promise<void>;
  export let stopRecording: () => void;
  export let revokeRecording: () => void;
  export let submitRecording: () => void | Promise<void>;
  export let chooseUpload: (event: Event) => void;
  export let submitUpload: () => void | Promise<void>;
  export let submitGeneration: () => void | Promise<void>;
  export let minutes: (seconds: number) => string;
</script>

<section class="neo-surface drilldown-panel button-config-panel">
  {#if button}
    <div class="drilldown-hero">
      <div>
        <p class="terminal-kicker">Face {button.id} control</p>
        <h2 class="drilldown-title">Button {button.id} configuration</h2>
        <p class="muted">Current lane: {button.mode === "language" ? button.language : modeLabel(button.mode)}</p>
      </div>
      <form class="mode-form" on:submit|preventDefault={() => saveMode(button)}>
        <label>Mode
          <select class="neo-field" bind:value={button.mode}>
            {#each modes as mode}
              <option value={mode}>{modeLabel(mode)}</option>
            {/each}
          </select>
        </label>
        {#if button.mode === "language"}
          <label>Language
            <select class="neo-field" bind:value={button.language}>
              {#each languages as language}
                <option value={language}>{language}</option>
              {/each}
            </select>
          </label>
        {/if}
        <button type="submit" class="neo-button" disabled={busy}>Save mode</button>
      </form>
    </div>

    {#if button.contentType}
      <div class="content-layout">
        <ContentList
          items={content?.active ?? []}
          loading={Boolean(content?.loading)}
          error={content?.error ?? null}
          {busy}
          {trash}
        />
        <ContentDraftQueue
          items={content?.inactive ?? []}
          loading={Boolean(content?.loading)}
          error={content?.error ?? null}
          {busy}
          canClearGenerated={button.contentType === "language"}
          {activate}
          {trash}
          clearGenerated={clearGenerated}
        />
      </div>
      <ContentAddTabs
        selectedButton={button}
        {selectedTab}
        {setTab}
        {draftForm}
        {languages}
        {providers}
        {busy}
        {recorder}
        {recordSeconds}
        {recordedWav}
        {uploadFile}
        {uploadPreviewUrl}
        {startRecording}
        {stopRecording}
        {revokeRecording}
        {submitRecording}
        {chooseUpload}
        {submitUpload}
        {submitGeneration}
        {minutes}
      />
    {:else}
      <div class="holo-inset empty-state">
        <p class="terminal-kicker">No content lane</p>
        <p class="muted">Set this face to Language, Animals, or Music to manage active content and drafts.</p>
      </div>
    {/if}
  {:else}
    <div class="holo-inset empty-state">
      <p class="terminal-kicker">No configurable face</p>
      <p class="muted">Configure one cube face as a content mode to open the review workspace.</p>
    </div>
  {/if}
</section>

