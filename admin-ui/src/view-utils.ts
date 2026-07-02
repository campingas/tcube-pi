import type { ButtonMode, ContentType } from "./api";

export const faceNames = ["Top", "Front left", "Front", "Front right", "Back"];

export function modeClass(mode: ButtonMode) {
  if (mode === "language") return "lang";
  if (mode === "animals") return "animal";
  if (mode === "music") return "music";
  if (mode === "setup_help") return "setup";
  return "off";
}

export function faceName(id: number) {
  return faceNames[id - 1] ?? `Button ${id}`;
}

export function contentLabel(mode: ButtonMode, language: string) {
  if (mode === "language") return shortLanguage(language);
  if (mode === "animals") return "Animals";
  if (mode === "music") return "Music";
  if (mode === "setup_help") return "Setup";
  return "Off";
}

export function shortLanguage(language: string) {
  if (language === "English") return "EN";
  if (language === "French") return "FR";
  if (language === "Vietnamese") return "VI";
  if (language === "Spanish") return "ES";
  if (language === "German") return "DE";
  return language.slice(0, 2).toUpperCase();
}

export function minutes(seconds: number) {
  const mins = Math.floor(seconds / 60);
  const secs = Math.floor(seconds % 60).toString().padStart(2, "0");
  return `${mins}:${secs}`;
}

export function formatDuration(seconds: number) {
  if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
  return minutes(seconds);
}

export function sourceLabel(source: string) {
  if (source === "generated") return "Generated";
  if (source === "uploaded") return "Uploaded";
  if (source === "recorded") return "Recorded";
  return "Default";
}

export function trimAudioTitle(title: string, maxLength = 32) {
  const clean = title.trim();
  if (clean.length <= maxLength) return clean;
  return `${clean.slice(0, maxLength - 1).trimEnd()}…`;
}

export function playCountLabel(count: number) {
  return count === 1 ? "1 play" : `${count} plays`;
}

export function contentPlaySummary(
  item: { id: string; source: string; play_count?: number },
  contentDurations: Record<string, number>
) {
  return `${sourceLabel(item.source)} · ${formatDuration(contentDurations[item.id] ?? 0)} · ${playCountLabel(item.play_count ?? 0)}`;
}

export function relativeTime(value: string) {
  const then = new Date(value).getTime();
  if (Number.isNaN(then)) return value;
  const seconds = Math.max(0, Math.floor((Date.now() - then) / 1000));
  if (seconds < 60) return "Just now";
  const mins = Math.floor(seconds / 60);
  if (mins < 60) return `${mins} min ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} hr ago`;
  const days = Math.floor(hours / 24);
  return `${days} day${days === 1 ? "" : "s"} ago`;
}

export function isIpLiteralHost(hostname: string) {
  if (!hostname) return false;
  if (hostname.startsWith("[") || hostname.includes(":")) return true;
  return /^\d{1,3}(\.\d{1,3}){3}$/.test(hostname);
}

export function contentTypeLabel(contentType: ContentType | null) {
  if (contentType === "language") return "Language";
  if (contentType === "animals") return "Animals";
  if (contentType === "music") return "Music";
  return "No playlist";
}
