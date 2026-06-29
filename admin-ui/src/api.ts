export type ServiceStatus = {
  status: string;
  service: string;
  mode: string;
  database_present: boolean;
  ui_dist_present: boolean;
  media_root: string;
  content_root: string;
  hostname: string;
  usb_address: string;
  contract_note: string;
};

export type Account = {
  id: string;
  username: string;
  display_name: string;
};

export type Cube = {
  device_id: string;
  label: string;
  role: "owner" | "manager" | string;
};

export type AuthSession = {
  authenticated: boolean;
  bootstrap_required: boolean;
  account: Account | null;
  cubes: Cube[];
};

export type SetupReview = {
  cube_name: string;
  device_id: string | null;
  admin_created: boolean;
  wifi_verified: boolean;
  dashboard_ip: string | null;
  dashboard_address: string;
  button_modes: Record<string, string>;
  active_counts: Record<string, number>;
};

export type ContentType = "language" | "animals" | "music";
export type ButtonMode = ContentType | "setup_help" | "disabled";

export type ActiveContentItem = {
  id: string;
  content_type: ContentType;
  title: string;
  text: string;
  source: string;
  state: "active";
  audio_path: string | null;
  preview_url: string | null;
};

export type InactiveContentItem = {
  id: string;
  content_type: ContentType;
  title: string;
  text: string | null;
  language: string | null;
  state: "archived" | "active";
  source: string;
  audio_path: string;
  preview_url: string;
};

export type ContentEmptyState = {
  title: string;
  detail: string;
};

export type ContentListResponse<T> = {
  items: T[];
  empty_state: ContentEmptyState | null;
};

export type ContentInventoryItem = {
  id: string;
  status: "active" | "draft" | "unused" | string;
  button_id: number;
  content_type: ContentType;
  language: string | null;
  title: string;
  text: string | null;
  source: string;
  state: "active" | "archived" | string;
  audio_path: string | null;
  preview_url: string | null;
  reason: string;
};

export type ContentInventory = {
  items: ContentInventoryItem[];
  active_count: number;
  draft_count: number;
  unused_count: number;
};

export type RecoveryCode = {
  code: string;
  expires_at: string;
};

export type Invitation = {
  id: string;
  code: string;
  device_id: string;
  role: "manager";
  expires_at: string;
};

export type CleanupResponse = {
  status: "ok";
  deleted_count: number;
};

export type GeneratedSpeechStatus = {
  online: boolean;
  provider: string;
  checked_at: string;
  cached: boolean;
  cache_ttl_seconds: number;
  next_check_after_seconds: number;
  message: string;
};

export type RecentButtonEvent = {
  occurred_at: string;
  button_id: number;
  mode: string;
  response_id: string;
  response_text: string;
};

type RequestOptions = RequestInit & {
  json?: unknown;
};

const API_ROOT = "/api/pi/v1";

export class ApiError extends Error {
  status: number;
  body: unknown;

  constructor(message: string, status: number, body: unknown) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
  }
}

export async function api<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const headers = new Headers(options.headers);
  let body = options.body;
  if (options.json !== undefined) {
    headers.set("content-type", "application/json");
    body = JSON.stringify(options.json);
  }
  const response = await fetch(path, {
    ...options,
    body,
    headers,
    credentials: "same-origin"
  });
  const contentType = response.headers.get("content-type") ?? "";
  const payload = contentType.includes("application/json")
    ? await response.json()
    : await response.text();
  if (!response.ok) {
    throw new ApiError(errorMessage(payload, response.statusText), response.status, payload);
  }
  return payload as T;
}

export function getStatus() {
  return api<ServiceStatus>(`${API_ROOT}/status`);
}

export function getSession() {
  return api<AuthSession>(`${API_ROOT}/auth/session`);
}

export function bootstrapOwner(body: { username: string; display_name: string; password: string }) {
  return api<AuthSession>(`${API_ROOT}/auth/bootstrap`, { method: "POST", json: body });
}

export function loginPassword(body: { username: string; password: string }) {
  return api<AuthSession>(`${API_ROOT}/auth/login/password`, { method: "POST", json: body });
}

export function logout() {
  return api<{ status: "ok" }>(`${API_ROOT}/auth/logout`, { method: "POST" });
}

export function createRecoveryCode() {
  return api<RecoveryCode>(`${API_ROOT}/auth/recovery-code`, { method: "POST" });
}

export function createInvitation(deviceId: string) {
  return api<Invitation>(`${API_ROOT}/auth/invitations`, {
    method: "POST",
    json: { device_id: deviceId }
  });
}

export function acceptInvitation(body: {
  code: string;
  username: string;
  display_name: string;
  password: string;
}) {
  return api<AuthSession>(`${API_ROOT}/auth/invitations/accept`, { method: "POST", json: body });
}

export function recoverPassword(body: { code: string; password: string }) {
  return api<{ status: "ok" }>(`${API_ROOT}/auth/recover`, { method: "POST", json: body });
}

export function getSetupReview() {
  return api<SetupReview>(`${API_ROOT}/setup/review`);
}

export function listRecentEvents() {
  return api<RecentButtonEvent[]>(`${API_ROOT}/events/recent`);
}

export function getContentInventory() {
  return api<ContentInventory>(`${API_ROOT}/content/inventory`);
}

export function saveCubeName(cubeName: string) {
  return api<{ status: "ok"; device_id: string; name: string; provisioned: boolean; token: string | null }>(
    `${API_ROOT}/setup/name`,
    { method: "POST", json: { cube_name: cubeName } }
  );
}

export function verifyWifi(ssid: string, dashboardIp: string) {
  return api<{ status: "ok" }>(`${API_ROOT}/setup/wifi/verified`, {
    method: "POST",
    json: { ssid, dashboard_ip: dashboardIp }
  });
}

export function completeSetup() {
  return api<{
    status: "complete";
    led_pattern: string;
    spoken_confirmation: boolean;
    dashboard_address: string;
  }>(`${API_ROOT}/setup/complete`, { method: "POST" });
}

export function saveButtonMode(buttonId: number, mode: ButtonMode, language: string) {
  const body = mode === "language" ? { mode, language } : { mode };
  return api<{ status: "ok" }>(`${API_ROOT}/setup/buttons/${buttonId}/mode`, {
    method: "POST",
    json: body
  });
}

export function listActiveContent(buttonId: number, contentType: ContentType, language?: string) {
  return api<ContentListResponse<ActiveContentItem>>(
    `${API_ROOT}/content/buttons/${buttonId}/${contentType}/active${languageQuery(contentType, language)}`
  );
}

export function listInactiveContent(buttonId: number, contentType: ContentType, language?: string) {
  return api<ContentListResponse<InactiveContentItem>>(
    `${API_ROOT}/content/buttons/${buttonId}/${contentType}/inactive${languageQuery(contentType, language)}`
  );
}

export function activateContentItem(id: string) {
  return api<InactiveContentItem>(`${API_ROOT}/content/items/${encodeURIComponent(id)}/activate`, {
    method: "POST"
  });
}

export function trashContentItem(id: string) {
  return api<{ status: "ok" }>(`${API_ROOT}/content/items/${encodeURIComponent(id)}`, {
    method: "DELETE"
  });
}

export function clearUnusedGeneratedSpeech(buttonId: number, language: string) {
  return api<CleanupResponse>(`${API_ROOT}/content/generated-speech/unused`, {
    method: "DELETE",
    json: { button_id: buttonId, language }
  });
}

export function clearUnusedContent() {
  return api<CleanupResponse>(`${API_ROOT}/content/unused`, { method: "DELETE" });
}

export function saveMultipart(path: "/content/recordings" | "/content/uploads", form: FormData) {
  return api<InactiveContentItem>(`${API_ROOT}${path}`, {
    method: "POST",
    body: form
  });
}

export function generateSpeech(body: {
  button_id: number;
  language: string;
  text: string;
  provider?: string;
  voice?: string;
}) {
  return api<InactiveContentItem>(`${API_ROOT}/content/generated-speech`, {
    method: "POST",
    json: body
  });
}

export function getGeneratedSpeechStatus(provider: string, language: string) {
  const params = new URLSearchParams({
    provider: provider.trim() || "auto",
    language: language.trim() || "English"
  });
  return api<GeneratedSpeechStatus>(`${API_ROOT}/content/generated-speech/status?${params.toString()}`);
}

function languageQuery(contentType: ContentType, language?: string) {
  if (contentType !== "language" || !language?.trim()) {
    return "";
  }
  return `?language=${encodeURIComponent(language.trim())}`;
}

function errorMessage(payload: unknown, fallback: string) {
  if (typeof payload === "string" && payload.trim()) {
    return payload;
  }
  if (payload && typeof payload === "object") {
    const value = payload as Record<string, unknown>;
    for (const key of ["detail", "error", "message"]) {
      if (typeof value[key] === "string") {
        return value[key] as string;
      }
    }
  }
  return fallback || "Request failed";
}
