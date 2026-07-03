import type { ButtonMode, ContentType } from "./api";

const modeLabels: Record<ButtonMode, string> = {
  language: "Language",
  animals: "Animals",
  music: "Music",
  soundbox: "SoundBox",
  setup_help: "Setup help",
  disabled: "Disabled"
};

export function modeLabel(mode: ButtonMode) {
  return modeLabels[mode] ?? mode;
}

export function contentTypeLabel(contentType: ContentType | null) {
  if (!contentType) return "No content";
  return modeLabel(contentType);
}

export function defaultMode(id: number) {
  if (id === 1) return "language:English";
  if (id === 2) return "animals";
  if (id === 3) return "music";
  return "setup_help";
}

export function splitMode(raw: string): { mode: ButtonMode; language: string } {
  const [modeValue, language = "English"] = raw.split(":");
  const mode = isButtonMode(modeValue) ? modeValue : "disabled";
  return { mode, language };
}

export function contentTypeForMode(mode: ButtonMode): ContentType | null {
  return isContentType(mode) ? mode : null;
}

function isButtonMode(value: string): value is ButtonMode {
  return ["language", "animals", "music", "soundbox", "setup_help", "disabled"].includes(value);
}

function isContentType(value: ButtonMode): value is ContentType {
  return ["language", "animals", "music"].includes(value);
}

