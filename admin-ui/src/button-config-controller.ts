import type { ActiveContentItem, ContentEmptyState, InactiveContentItem, RecentActivityEvent, SetupReview } from "./api";
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
  events: RecentActivityEvent[];
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
