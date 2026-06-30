import type { ContentType } from "./api";
import type { ButtonConfig, DraftForm } from "./types";

export type RecordingStatus = "idle" | "recording" | "processing" | "ready" | "saving";

export const maxUploadBytes = 25 * 1024 * 1024;
export const defaultWaveformBars = 24;
export const defaultWaveformLevel = 0.12;

export function initialRecordWaveform(length = defaultWaveformBars) {
  return Array.from({ length }, () => defaultWaveformLevel);
}

export function shouldBlockRecordingStart(status: RecordingStatus) {
  return status === "processing" || status === "saving";
}

export function recordingStatusAfterStop(recorderState: string | null | undefined): RecordingStatus | null {
  return recorderState && recorderState !== "inactive" ? "processing" : null;
}

export function recordingStatusAfterRevoke(hasRecorder: boolean): RecordingStatus | null {
  return hasRecorder ? null : "idle";
}

export function recordingStatusAfterSave(hasRecording: boolean): RecordingStatus {
  return hasRecording ? "ready" : "idle";
}

export function recordingHint(status: RecordingStatus, seconds: number, hasRecording: boolean, formatSeconds = minutes) {
  if (status === "recording") return `Recording ${formatSeconds(seconds)}. Tap again to stop.`;
  if (status === "processing") return "Preparing preview...";
  if (status === "saving") return "Saving recording as draft...";
  if (hasRecording) return `Preview ${formatSeconds(seconds)}, then save it as a draft.`;
  return "Tap record, then speak clearly near your phone.";
}

export function recordingSaveHint(contentType: ContentType | null, text: string, hasRecording: boolean) {
  if (!hasRecording) return "After recording, preview the audio here before saving.";
  if (contentType === "language" && !text.trim()) {
    return "Enter the text spoken before saving this recording.";
  }
  return "Saving creates an inactive draft for review.";
}

export function validateUploadFile(file: Pick<File, "name" | "size"> | null) {
  if (!file) return { ok: false as const, error: "Upload failed. Choose an MP3 or WAV file first." };
  const name = file.name.toLowerCase();
  if (!name.endsWith(".wav") && !name.endsWith(".mp3")) {
    return { ok: false as const, error: "Upload failed. File must be MP3 or WAV." };
  }
  if (file.size > maxUploadBytes) {
    return { ok: false as const, error: "Upload failed. File must be 25 MB or smaller." };
  }
  return { ok: true as const };
}

export function waveformLevels(data: Uint8Array, barCount: number) {
  if (barCount <= 0) return [];
  const segmentSize = Math.max(1, Math.floor(data.length / barCount));
  return Array.from({ length: barCount }, (_, index) => {
    const start = index * segmentSize;
    const end = Math.min(data.length, start + segmentSize);
    let peak = 0;
    for (let sampleIndex = start; sampleIndex < end; sampleIndex += 1) {
      peak = Math.max(peak, Math.abs(data[sampleIndex] - 128) / 128);
    }
    return Math.max(0.08, Math.min(1, peak * 2.4));
  });
}

export function mediaDraftValidationError(button: ButtonConfig | null, form: DraftForm) {
  if (!button?.contentType) {
    return "Choose a content button first.";
  }
  if (button.contentType === "language" && !form.text.trim()) {
    return "Save draft failed. Enter the text spoken in the recording or upload.";
  }
  if (button.contentType !== "language" && !form.title.trim()) {
    return "Save draft failed. Enter a title for this audio.";
  }
  return null;
}

export function defaultDraftTitle(filename: string) {
  if (filename === "recording.wav") return "Recorded audio";
  return filename.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").trim() || "Uploaded audio";
}

function minutes(seconds: number) {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${mins}:${secs}`;
}
