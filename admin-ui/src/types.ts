import type { ActiveContentItem, ButtonMode, ContentEmptyState, ContentType, InactiveContentItem } from "./api";

export type ButtonConfig = {
  id: number;
  mode: ButtonMode;
  language: string;
  contentType: ContentType | null;
};

export type ContentState = {
  active: ActiveContentItem[];
  inactive: InactiveContentItem[];
  activeEmptyState: ContentEmptyState | null;
  inactiveEmptyState: ContentEmptyState | null;
  loading: boolean;
  error: string | null;
};

export type DraftForm = {
  title: string;
  text: string;
  language: string;
  provider: string;
  voice: string;
};

export type MessageType = "info" | "success" | "error";

export type InventoryFilter = "presses_today" | "active" | "draft" | "unused";
