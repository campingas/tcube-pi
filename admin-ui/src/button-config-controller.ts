import type { ActiveContentItem, ContentEmptyState, InactiveContentItem, RecentButtonEvent, SetupReview } from "./api";
import type { ButtonConfig, ContentState, DraftForm, MessageType } from "./types";

export type ButtonView = ButtonConfig & { activeCount: number; draftCount: number };

export type SelectedContent = {
  active: ActiveContentItem[];
  inactive: InactiveContentItem[];
  activeEmptyState: ContentEmptyState | null;
  inactiveEmptyState: ContentEmptyState | null;
  loading: boolean;
  error: string | null;
};

export type ButtonConfigViewModel = {
  setup: SetupReview | null;
  message: string;
  messageType: MessageType;
  buttons: ButtonView[];
  selectedButtonId: number;
  selectedButton: ButtonView | null;
  selectedContent: SelectedContent | null;
  selectedTab: "record" | "upload" | "generate";
  contentListTab: "active" | "draft";
  draftForm: DraftForm;
  recordedWav: unknown | null;
  uploadFile: File | null;
  contentDurations: Record<string, number>;
  events: RecentButtonEvent[];
  generatedSpeechDisabled: boolean;
  busy: boolean;
};

export function contentKey(button: ButtonConfig) {
  return `${button.id}:${button.contentType ?? "none"}:${button.language}`;
}

export function buttonActiveCount(button: ButtonConfig, state: Record<string, ContentState>) {
  if (!button.contentType) return 0;
  const content = state[contentKey(button)];
  return content?.active.length ?? 0;
}

export function buttonDraftCount(button: ButtonConfig, state: Record<string, ContentState>) {
  if (!button.contentType) return 0;
  return state[contentKey(button)]?.inactive.length ?? 0;
}

export function buttonViewModels(buttons: ButtonConfig[], state: Record<string, ContentState>): ButtonView[] {
  return buttons.map((button) => ({
    ...button,
    activeCount: buttonActiveCount(button, state),
    draftCount: buttonDraftCount(button, state)
  }));
}

export function updateDraftFormValue(form: DraftForm, patch: Partial<DraftForm>): DraftForm {
  return { ...form, ...patch };
}

export function buttonConfigFooterLabel(state: ButtonConfigViewModel) {
  if (state.selectedButton?.contentType === "language" && state.selectedTab === "record") return "Save recording";
  if (state.selectedButton?.contentType === "language" && state.selectedTab === "upload") return "Save upload";
  if (state.selectedButton?.contentType && state.selectedTab === "record" && state.recordedWav) return "Save recording";
  if (state.selectedButton?.contentType && state.selectedTab === "upload" && state.uploadFile) return "Save upload";
  if (state.selectedButton?.contentType === "language" && state.selectedTab === "generate" && state.draftForm.text.trim()) {
    return "Save draft";
  }
  return "Save mode";
}

export function buttonConfigFooterDisabled(state: ButtonConfigViewModel) {
  if (state.busy || !state.selectedButton) return true;
  if (state.selectedButton.contentType === "language" && state.selectedTab === "record" && !state.recordedWav) {
    return true;
  }
  if (state.selectedButton.contentType === "language" && state.selectedTab === "upload" && !state.uploadFile) {
    return true;
  }
  if (state.selectedButton.contentType === "language" && state.selectedTab === "record" && state.recordedWav && !state.draftForm.text.trim()) {
    return true;
  }
  if (state.selectedButton.contentType === "language" && state.selectedTab === "upload" && state.uploadFile && !state.draftForm.text.trim()) {
    return true;
  }
  if (state.selectedButton.contentType === "language" && state.selectedTab === "generate" && state.draftForm.text.trim()) {
    return state.generatedSpeechDisabled;
  }
  return false;
}
