import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { updateDraftFormValue } from "../../src/button-config-controller.ts";
import {
  generatedSpeechDisabled,
  generatedSpeechOfflineStatus,
  generatedSpeechStatusKey,
  generatedSpeechVoices,
  isSpeechProviderOfflineMessage,
  menuGeneratedSpeechStatusKey,
  nextGeneratedSpeechBackoff,
  preferredGeneratedSpeechVoice
} from "../../src/generated-speech-health.ts";
import {
  applyAgeRecommendation,
  applyPreset,
  pomodoroCanEnable,
  pomodoroPayload,
  recommendationForAge,
  settingsToPomodoroForm
} from "../../src/focus-routine-controller.ts";
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
  uploadFileSize,
  uploadHint,
  uploadStepLabel,
  validateUploadFile,
  waveformLevels
} from "../../src/recording-controller.ts";
import { contentTypeForMode, modeLabel, splitMode } from "../../src/button-mode.ts";
import type { ButtonConfig, DraftForm } from "../../src/types.ts";
import { contentLabel, isIpLiteralHost, modeClass } from "../../src/view-utils.ts";

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

describe("button config controller", () => {
  test("patches draft forms without mutating the original object", () => {
    const next = updateDraftFormValue(baseDraft, { text: "Bonjour", provider: "voxtral" });
    assert.deepEqual(next, { ...baseDraft, text: "Bonjour", provider: "voxtral" });
    assert.equal(baseDraft.text, "");
    assert.notEqual(next, baseDraft);
  });
});

describe("view utils", () => {
  test("detects IP literal hosts for certificate trust help", () => {
    assert.equal(isIpLiteralHost("192.168.50.25"), true);
    assert.equal(isIpLiteralHost("10.55.0.1"), true);
    assert.equal(isIpLiteralHost("[::1]"), true);
    assert.equal(isIpLiteralHost("tcube.local"), false);
    assert.equal(isIpLiteralHost("localhost"), false);
    assert.equal(isIpLiteralHost(""), false);
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
    assert.deepEqual(status.voices, []);

    assert.equal(isSpeechProviderOfflineMessage("failed to connect to speech provider"), true);
    assert.equal(isSpeechProviderOfflineMessage("validation failed"), false);
  });

  test("exposes online voices and chooses a safe default", () => {
    assert.deepEqual(generatedSpeechVoices(null), []);
    assert.deepEqual(
      generatedSpeechVoices({ online: true, voices: ["casual_female", "neutral_male"] } as never),
      ["casual_female", "neutral_male"]
    );
    assert.deepEqual(generatedSpeechVoices({ online: false, voices: ["neutral_male"] } as never), []);
    assert.equal(preferredGeneratedSpeechVoice(["casual_female", "neutral_male"], ""), "neutral_male");
    assert.equal(preferredGeneratedSpeechVoice(["casual_female", "neutral_male"], "casual_female"), "casual_female");
    assert.equal(preferredGeneratedSpeechVoice(["casual_female"], "missing"), "casual_female");
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

  test("describes upload steps and file sizes", () => {
    assert.equal(uploadStepLabel(null, false), "Choose file");
    assert.equal(uploadStepLabel({ name: "hello.wav", size: 1536 }, false), "Review details");
    assert.equal(uploadStepLabel({ name: "hello.wav", size: 1536 }, true), "Save Draft");
    assert.equal(uploadHint(null, false), "Choose an MP3 or WAV under 25 MB.");
    assert.equal(uploadHint({ name: "hello.wav", size: 1536 }, false), "Preview the file, then add the required details.");
    assert.equal(
      uploadHint({ name: "hello.wav", size: 1536 }, true),
      "Saving creates a Draft. The child cannot hear it until you activate it."
    );
    assert.equal(uploadFileSize(512), "512 B");
    assert.equal(uploadFileSize(1536), "1.5 KB");
    assert.equal(uploadFileSize(2 * 1024 * 1024), "2.0 MB");
    assert.equal(uploadFileSize(12 * 1024 * 1024), "12 MB");
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

describe("focus routine controller", () => {
  test("maps child age to local Pomodoro recommendations", () => {
    assert.equal(recommendationForAge(null).focus_minutes, 10);
    assert.equal(recommendationForAge(4).focus_minutes, 8);
    assert.equal(recommendationForAge(7).break_minutes, 4);
    assert.equal(recommendationForAge(10).preset, "focus");
    assert.equal(recommendationForAge(13).cycles, 4);
  });

  test("age selection applies the recommended plan", () => {
    const form = settingsToPomodoroForm(null);
    const next = applyAgeRecommendation(form, "10");
    assert.deepEqual(next, {
      enabled: false,
      childAgeYears: "10",
      focusMinutes: 20,
      breakMinutes: 5,
      cycles: 3,
      preset: "focus"
    });
    assert.equal(pomodoroCanEnable(next), true);
  });

  test("custom edits persist as validated payload fields", () => {
    const form = applyPreset(settingsToPomodoroForm(null), "full");
    const payload = pomodoroPayload({ ...form, enabled: true, childAgeYears: "14", focusMinutes: 26 });
    assert.deepEqual(payload, {
      enabled: true,
      child_age_years: 14,
      focus_minutes: 26,
      break_minutes: 5,
      cycles: 4,
      preset: "full"
    });
  });

  test("unset age cannot enable the routine", () => {
    const payload = pomodoroPayload({ ...settingsToPomodoroForm(null), enabled: true });
    assert.equal(payload.enabled, false);
    assert.equal(payload.child_age_years, null);
  });
});

describe("soundbox mode", () => {
  test("splitMode parses soundbox as a valid button mode", () => {
    assert.deepEqual(splitMode("soundbox"), { mode: "soundbox", language: "English" });
  });

  test("soundbox has no content lane", () => {
    assert.equal(contentTypeForMode("soundbox"), null);
  });

  test("soundbox labels and classes", () => {
    assert.equal(modeLabel("soundbox"), "SoundBox");
    assert.equal(modeClass("soundbox"), "soundbox");
    assert.equal(contentLabel("soundbox", "English"), "SoundBox");
  });
});
