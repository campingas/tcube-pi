import type { GeneratedSpeechStatus } from "./api";
import type { ButtonConfig, DraftForm } from "./types";

export const generatedSpeechMinBackoffSeconds = 30;
export const generatedSpeechMaxBackoffSeconds = 120;

export function primaryLanguageForTts(buttons: ButtonConfig[]) {
  return buttons.find((button) => button.contentType === "language")?.language || "English";
}

export function generatedSpeechStatusKey(
  selectedButton: ButtonConfig | null,
  selectedTab: "record" | "upload" | "generate",
  draftForm: DraftForm
) {
  return selectedButton?.contentType === "language" && selectedTab === "generate"
    ? `${draftForm.provider}:${selectedButton.language}`
    : "";
}

export function menuGeneratedSpeechStatusKey(authenticated: boolean, buttons: ButtonConfig[]) {
  return authenticated ? `auto:${primaryLanguageForTts(buttons)}` : "";
}

export function generatedSpeechDisabled(key: string, status: GeneratedSpeechStatus | null, loading: boolean) {
  const offline = Boolean(key && status && !status.online);
  return offline || (loading && !status);
}

export function nextGeneratedSpeechBackoff(online: boolean, current: number, immediate: boolean) {
  if (online) return generatedSpeechMinBackoffSeconds;
  return Math.min(
    generatedSpeechMaxBackoffSeconds,
    immediate ? generatedSpeechMinBackoffSeconds : current * 2
  );
}

export function parseGeneratedSpeechStatusKey(key: string) {
  const [provider, language] = key.split(":");
  return {
    provider: provider || "auto",
    language: language || "English"
  };
}

export function speechProviderOfflineMessage(detail: string) {
  return detail.includes("TTS provider")
    ? detail
    : `TTS provider is offline or unreachable: ${detail}`;
}

export function generatedSpeechOfflineStatus(key: string, detail: string, checkedAt = new Date()): GeneratedSpeechStatus {
  const { provider } = parseGeneratedSpeechStatusKey(key);
  return {
    online: false,
    provider,
    checked_at: checkedAt.toISOString(),
    cached: false,
    cache_ttl_seconds: 20,
    next_check_after_seconds: generatedSpeechMinBackoffSeconds,
    message: speechProviderOfflineMessage(detail)
  };
}

export function isSpeechProviderOfflineMessage(text: string) {
  const normalized = text.toLowerCase();
  return normalized.includes("failed to connect to speech provider") || normalized.includes("tts provider is offline");
}
