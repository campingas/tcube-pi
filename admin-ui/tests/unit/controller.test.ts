import assert from "node:assert/strict";
import { describe, test } from "node:test";
import {
  buttonConfigFooterDisabled,
  buttonConfigFooterLabel,
  updateDraftFormValue
} from "../../src/button-config-controller.ts";
import {
  generatedSpeechDisabled,
  generatedSpeechOfflineStatus,
  generatedSpeechStatusKey,
  isSpeechProviderOfflineMessage,
  menuGeneratedSpeechStatusKey,
  nextGeneratedSpeechBackoff
} from "../../src/generated-speech-health.ts";
import {
  defaultDraftTitle,
  initialRecordWaveform,
  mediaDraftValidationError,
  recordingHint,
  recordingSaveHint,
  recordingStatusAfterRevoke,
  recordingStatusAfterSave,
  recordingStatusAfterStop,
  shouldBlockRecordingStart,
  validateUploadFile,
  waveformLevels
} from "../../src/recording-controller.ts";
import type { ButtonConfig, DraftForm } from "../../src/types.ts";

const languageButton: ButtonConfig = {
  id: 1,
  mode: "language",
  language: "French",
  contentType: "language"
};

const baseDraft: DraftForm = {
  title: "",
  text: "",
  language: "English",
  provider: "auto",
  voice: ""
};

function footerState(overrides = {}) {
  return {
    setup: null,
    message: "",
    messageType: "info",
    buttons: [{ ...languageButton, activeCount: 0, draftCount: 0 }],
    selectedButtonId: 1,
    selectedButton: { ...languageButton, activeCount: 0, draftCount: 0 },
    selectedContent: null,
    selectedTab: "record",
    contentListTab: "active",
    draftForm: baseDraft,
    recordedWav: null,
    uploadFile: null,
    contentDurations: {},
    events: [],
    generatedSpeechDisabled: false,
    busy: false,
    ...overrides
  } as Parameters<typeof buttonConfigFooterLabel>[0];
}

describe("button config controller", () => {
  test("requires recorded language audio and text before enabling save recording", () => {
    assert.equal(buttonConfigFooterLabel(footerState()), "Save recording");
    assert.equal(buttonConfigFooterDisabled(footerState()), true);

    const withAudio = footerState({ recordedWav: {} });
    assert.equal(buttonConfigFooterDisabled(withAudio), true);

    const withAudioAndText = footerState({
      recordedWav: {},
      draftForm: { ...baseDraft, text: "Bonjour" }
    });
    assert.equal(buttonConfigFooterDisabled(withAudioAndText), false);
  });

  test("routes generated speech footer through save draft and respects health disabled state", () => {
    const ready = footerState({
      selectedTab: "generate",
      draftForm: { ...baseDraft, text: "Bonjour" },
      generatedSpeechDisabled: false
    });
    assert.equal(buttonConfigFooterLabel(ready), "Save draft");
    assert.equal(buttonConfigFooterDisabled(ready), false);

    assert.equal(buttonConfigFooterDisabled({ ...ready, generatedSpeechDisabled: true }), true);
  });

  test("patches draft forms without mutating the original object", () => {
    const next = updateDraftFormValue(baseDraft, { text: "Bonjour", provider: "voxtral" });
    assert.deepEqual(next, { ...baseDraft, text: "Bonjour", provider: "voxtral" });
    assert.equal(baseDraft.text, "");
    assert.notEqual(next, baseDraft);
  });
});

describe("generated speech health controller", () => {
  test("builds route-specific and menu status keys", () => {
    assert.equal(generatedSpeechStatusKey(languageButton, "generate", { ...baseDraft, provider: "voxtral" }), "voxtral:French");
    assert.equal(generatedSpeechStatusKey(languageButton, "record", baseDraft), "");
    assert.equal(menuGeneratedSpeechStatusKey(true, [languageButton]), "auto:French");
    assert.equal(menuGeneratedSpeechStatusKey(false, [languageButton]), "");
  });

  test("calculates disabled state and backoff transitions", () => {
    assert.equal(generatedSpeechDisabled("auto:French", null, true), true);
    assert.equal(generatedSpeechDisabled("auto:French", { online: false } as never, false), true);
    assert.equal(generatedSpeechDisabled("auto:French", { online: true } as never, false), false);

    assert.equal(nextGeneratedSpeechBackoff(true, 60, false), 30);
    assert.equal(nextGeneratedSpeechBackoff(false, 60, false), 120);
    assert.equal(nextGeneratedSpeechBackoff(false, 60, true), 30);
  });

  test("normalizes offline status and offline error matching", () => {
    const status = generatedSpeechOfflineStatus("voxtral:French", "connection refused", new Date("2026-06-30T00:00:00.000Z"));
    assert.equal(status.online, false);
    assert.equal(status.provider, "voxtral");
    assert.equal(status.checked_at, "2026-06-30T00:00:00.000Z");
    assert.match(status.message ?? "", /TTS provider is offline or unreachable/);

    assert.equal(isSpeechProviderOfflineMessage("failed to connect to speech provider"), true);
    assert.equal(isSpeechProviderOfflineMessage("validation failed"), false);
  });
});

describe("recording controller", () => {
  test("validates upload type and size", () => {
    assert.equal(validateUploadFile(null).ok, false);
    assert.deepEqual(validateUploadFile({ name: "sound.txt", size: 100 }), {
      ok: false,
      error: "Upload failed. File must be MP3 or WAV."
    });
    assert.deepEqual(validateUploadFile({ name: "sound.wav", size: 25 * 1024 * 1024 + 1 }), {
      ok: false,
      error: "Upload failed. File must be 25 MB or smaller."
    });
    assert.deepEqual(validateUploadFile({ name: "sound.MP3", size: 25 * 1024 * 1024 }), { ok: true });
  });

  test("calculates recording status transitions and hints", () => {
    assert.equal(shouldBlockRecordingStart("processing"), true);
    assert.equal(shouldBlockRecordingStart("idle"), false);
    assert.equal(recordingStatusAfterStop("recording"), "processing");
    assert.equal(recordingStatusAfterStop("inactive"), null);
    assert.equal(recordingStatusAfterRevoke(false), "idle");
    assert.equal(recordingStatusAfterRevoke(true), null);
    assert.equal(recordingStatusAfterSave(false), "idle");
    assert.equal(recordingStatusAfterSave(true), "ready");

    assert.equal(recordingHint("idle", 0, false), "Tap record, then speak clearly near your phone.");
    assert.equal(recordingHint("recording", 65, false), "Recording 1:05. Tap again to stop.");
    assert.equal(recordingHint("ready", 1, true), "Preview 0:01, then save it as a draft.");
    assert.equal(recordingSaveHint("language", "", true), "Enter the text spoken before saving this recording.");
    assert.equal(recordingSaveHint("music", "", true), "Saving creates an inactive draft for review.");
  });

  test("validates media draft requirements and default titles", () => {
    assert.equal(mediaDraftValidationError(null, baseDraft), "Choose a content button first.");
    assert.equal(
      mediaDraftValidationError(languageButton, baseDraft),
      "Save draft failed. Enter the text spoken in the recording or upload."
    );
    assert.equal(mediaDraftValidationError(languageButton, { ...baseDraft, text: "Bonjour" }), null);
    assert.equal(
      mediaDraftValidationError({ id: 3, mode: "music", language: "English", contentType: "music" }, baseDraft),
      "Save draft failed. Enter a title for this audio."
    );
    assert.equal(defaultDraftTitle("recording.wav"), "Recorded audio");
    assert.equal(defaultDraftTitle("hello_world.mp3"), "hello world");
  });

  test("creates stable waveform defaults and normalized live levels", () => {
    assert.deepEqual(initialRecordWaveform(3), [0.12, 0.12, 0.12]);
    assert.deepEqual(waveformLevels(new Uint8Array([128, 128, 128, 128]), 2), [0.08, 0.08]);
    assert.deepEqual(waveformLevels(new Uint8Array([0, 255]), 2), [1, 1]);
    assert.deepEqual(waveformLevels(new Uint8Array([128]), 0), []);
  });
});
