import type { ActiveContentItem, ButtonMode, ContentType, InactiveContentItem } from "./api";

export type ButtonConfig = {
  id: number;
  mode: ButtonMode;
  language: string;
  contentType: ContentType | null;
};

export type ContentState = {
  active: ActiveContentItem[];
  inactive: InactiveContentItem[];
  loading: boolean;
  error: string | null;
};

export type MessageType = "info" | "success" | "error";

