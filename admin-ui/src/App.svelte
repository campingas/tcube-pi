<script lang="ts">
  import { onDestroy, onMount, tick } from "svelte";
  import {
    Activity,
    AlertTriangle,
    ArrowLeft,
    ArrowRight,
    BarChart3,
    Bell,
    Bolt,
    Check,
    CircleCheck,
    Cuboid,
    Database,
    FileAudio,
    Folder,
    Hand,
    HardDrive,
    KeyRound,
    Languages,
    LogIn,
    LogOut,
    Mic,
    Minus,
    Music,
    PawPrint,
    Play,
    Plus,
    RefreshCw,
    Save,
    Settings,
    ShieldCheck,
    SlidersHorizontal,
    Trash2,
    Upload,
    User,
    WandSparkles,
    Wifi,
    Wrench
  } from "@lucide/svelte";
  import {
    acceptInvitation,
    activateContentItem,
    bootstrapOwner,
    clearUnusedGeneratedSpeech,
    completeSetup,
    createInvitation,
    createRecoveryCode,
    generateSpeech,
    getContentInventory,
    getGeneratedSpeechStatus,
    getSession,
    getSetupReview,
    getStatus,
    listActiveContent,
    listInactiveContent,
    listRecentEvents,
    loginPassword,
    logout,
    recoverPassword,
    saveButtonMode,
    saveCubeName,
    saveMultipart,
    trashContentItem,
    verifyWifi
  } from "./api";
  import type {
    ActiveContentItem,
    AuthSession,
    ButtonMode,
    ContentEmptyState,
    GeneratedSpeechStatus,
    ContentInventory,
    ContentInventoryItem,
    ContentType,
    InactiveContentItem,
    RecentButtonEvent,
    ServiceStatus,
    SetupReview
  } from "./api";
  import { blobToWav, canRecordAudio, isSecureRecorderContext } from "./audio";
  import type { RecordedWav } from "./audio";
  import { contentTypeForMode, defaultMode, modeLabel, splitMode } from "./button-mode";
  import type { ButtonConfig, ContentState, MessageType } from "./types";

  type View = "dashboard" | "button-config" | "inventory" | "settings";
  type ContentTab = "record" | "upload" | "generate";
  type ContentListTab = "active" | "draft";
  type ContentListItem = ActiveContentItem | InactiveContentItem;
  type ContentAction = (id: string) => Promise<void>;
  type RecordingStatus = "idle" | "recording" | "processing" | "ready" | "saving";

  const modes: ButtonMode[] = ["language", "animals", "music", "setup_help", "disabled"];
  const languages = [
    "English",
    "French",
    "Vietnamese",
    "Spanish",
    "German",
    "Italian",
    "Portuguese",
    "Dutch",
    "Arabic",
    "Hindi"
  ];
  const providers = ["auto", "voxtral", "vietnamese-vits"];
  const faceNames = ["Top", "Front left", "Front", "Front right", "Back"];
  const generatedSpeechMinBackoffSeconds = 30;
  const generatedSpeechMaxBackoffSeconds = 120;

  let status: ServiceStatus | null = null;
  let session: AuthSession | null = null;
  let setup: SetupReview | null = null;
  let events: RecentButtonEvent[] = [];
  let inventory: ContentInventory | null = null;
  let inventoryError: string | null = null;
  let loading = true;
  let busy = false;
  let view: View = "dashboard";
  let message = "Loading local cube state.";
  let messageType: MessageType = "info";

  let bootstrapForm = { username: "parent", display_name: "Parent Admin", password: "" };
  let loginForm = { username: "", password: "" };
  let recoveryForm = { code: "", password: "" };
  let inviteForm = { code: "", username: "", display_name: "", password: "" };
  let cubeName = "T-Cube";
  let wifiForm = { ssid: "", dashboard_ip: "" };
  let recoveryCode: { code: string; expires_at: string } | null = null;
  let invitation: { code: string; expires_at: string } | null = null;

  let selectedButtonId = 1;
  let selectedTab: ContentTab = "record";
  let contentListTab: ContentListTab = "active";
  let contentState: Record<string, ContentState> = {};
  let draftForm = { title: "", text: "", language: "English", provider: "auto", voice: "" };
  let uploadFile: File | null = null;
  let uploadPreviewUrl: string | null = null;
  let recordedWav: RecordedWav | null = null;
  let recorder: MediaRecorder | null = null;
  let recordingStatus: RecordingStatus = "idle";
  let recordStartedAt = 0;
  let recordSeconds = 0;
  let recordTimer: number | null = null;
  let recordAudioContext: AudioContext | null = null;
  let recordAnalyser: AnalyserNode | null = null;
  let recordWaveFrame: number | null = null;
  let recordWaveform = Array.from({ length: 24 }, () => 0.12);
  let draggingUpload = false;
  let previewAudio: HTMLAudioElement | null = null;
  let previewAudioId: string | null = null;
  let trashPrompt: { id: string; title: string } | null = null;
  let contentDurations: Record<string, number> = {};
  let generatedSpeechStatus: GeneratedSpeechStatus | null = null;
  let generatedSpeechStatusLoading = false;
  let generatedSpeechStatusError: string | null = null;
  let generatedSpeechStatusKey = "";
  let lastGeneratedSpeechStatusKey = "";
  let generatedSpeechCheckTimer: number | null = null;
  let generatedSpeechBackoffSeconds = generatedSpeechMinBackoffSeconds;
  let generatedSpeechOffline = false;
  let generatedSpeechDisabled = false;
  let menuLlmStatus: GeneratedSpeechStatus | null = null;
  let menuLlmStatusLoading = false;
  let menuLlmStatusKey = "";
  let lastMenuLlmStatusKey = "";
  let menuLlmCheckTimer: number | null = null;
  let menuLlmBackoffSeconds = generatedSpeechMinBackoffSeconds;

  $: buttons = buildButtonConfigs(setup);
  $: selectedButton = buttons.find((button) => button.id === selectedButtonId) ?? buttons[0] ?? null;
  $: selectedContent = selectedButton?.contentType ? contentState[contentKey(selectedButton)] : null;
  $: currentRole = session?.cubes?.[0]?.role ?? "";
  $: isOwner = currentRole === "owner";
  $: roleLabel = currentRole === "owner" ? "owner" : currentRole === "manager" ? "admin" : currentRole || "member";
  $: roleClass = currentRole === "owner" ? "owner" : currentRole === "manager" ? "admin" : "member";
  $: invitationCodeFromUrl = new URLSearchParams(window.location.search).get("invite") ?? "";
  $: loadedActive = inventory?.active_count ?? Object.values(contentState).reduce((sum, state) => sum + state.active.length, 0);
  $: setupActive = Object.values(setup?.active_counts ?? {}).reduce((sum, value) => sum + value, 0);
  $: generatedSpeechStatusKey = selectedButton?.contentType === "language" && selectedTab === "generate"
    ? `${draftForm.provider}:${selectedButton.language}`
    : "";
  $: menuLlmStatusKey = session?.authenticated ? `auto:${primaryLanguageForTts(buttons)}` : "";
  $: if (generatedSpeechStatusKey && generatedSpeechStatusKey !== lastGeneratedSpeechStatusKey) {
    void checkGeneratedSpeechStatus(generatedSpeechStatusKey, true);
  }
  $: if (!generatedSpeechStatusKey && lastGeneratedSpeechStatusKey) {
    clearGeneratedSpeechStatusTimer();
    lastGeneratedSpeechStatusKey = "";
  }
  $: generatedSpeechOffline = Boolean(generatedSpeechStatusKey && generatedSpeechStatus && !generatedSpeechStatus.online);
  $: generatedSpeechDisabled = generatedSpeechOffline || (generatedSpeechStatusLoading && !generatedSpeechStatus);
  $: if (menuLlmStatusKey && menuLlmStatusKey !== lastMenuLlmStatusKey) {
    void checkMenuLlmStatus(menuLlmStatusKey, true);
  }
  $: if (!menuLlmStatusKey && lastMenuLlmStatusKey) {
    clearMenuLlmStatusTimer();
    lastMenuLlmStatusKey = "";
    menuLlmStatus = null;
  }
  $: menuLlmOnline = Boolean(menuLlmStatus?.online);
  $: menuLlmLabel = menuLlmStatusLoading && !menuLlmStatus ? "LLMs checking" : menuLlmOnline ? "LLMs online" : "LLMs offline";
  $: totalActive = loadedActive || setupActive;
  $: totalDrafts = inventory?.draft_count ?? Object.values(contentState).reduce((sum, state) => sum + state.inactive.length, 0);
  $: totalUnused = inventory?.unused_count ?? 0;
  $: prerequisites = [
    { id: "account", label: "Owner account created", detail: session?.account?.username ?? "Local owner required", complete: Boolean(setup?.admin_created), action: "Create" },
    { id: "name", label: "Cube name saved", detail: cubeName, complete: Boolean(setup?.cube_name?.trim()), action: "Save" },
    { id: "wifi", label: "Wi-Fi verified", detail: wifiForm.dashboard_ip || "Home network address", complete: Boolean(setup?.wifi_verified), action: "Verify" },
    { id: "language", label: "Active language content", detail: "At least 1 sound on a language button", complete: activeCount("language") > 0, action: "Add" },
    { id: "animals", label: "Active animal content", detail: "At least 1 sound on an animal button", complete: activeCount("animals") > 0, action: "Add" },
    { id: "music", label: "Active music content", detail: "At least 1 sound on a music button", complete: activeCount("music") > 0, action: "Add" }
  ];
  $: setupReady = prerequisites.every((item) => item.complete);
  $: blockedSetupText = prerequisites
    .filter((item) => !item.complete)
    .map((item) => item.label)
    .join(", ");
  $: if (selectedButton?.contentType !== "language" && selectedTab === "generate") {
    setContentTab("record");
  }

  onMount(async () => {
    document.documentElement.dataset.theme = "dark";
    inviteForm.code = invitationCodeFromUrl;
    await refreshAll();
  });

  async function refreshAll() {
    loading = true;
    try {
      const [nextStatus, nextSession, nextSetup] = await Promise.all([
        getStatus(),
        getSession(),
        getSetupReview()
      ]);
      status = nextStatus;
      session = nextSession;
      setup = nextSetup;
      cubeName = setup.cube_name || "T-Cube";
      wifiForm.dashboard_ip = setup.dashboard_ip ?? "";
      await tick();
      await Promise.all([refreshVisibleContent(), refreshEvents(), refreshInventory()]);
      setMessage("Cube state refreshed.", "success");
    } catch (error) {
      setError(error, "Could not reach the Pi. Check that you are on the home network.");
    } finally {
      loading = false;
    }
  }

  async function refreshEvents() {
    if (!session?.authenticated) {
      events = [];
      return;
    }
    try {
      events = await listRecentEvents();
    } catch {
      events = [];
    }
  }

  async function refreshInventory() {
    if (!session?.authenticated) {
      inventory = null;
      inventoryError = null;
      return;
    }
    try {
      inventory = await getContentInventory();
      inventoryError = null;
    } catch (error) {
      inventory = null;
      inventoryError = errorText(error);
    }
  }

  async function refreshVisibleContent() {
    if (!setup || !session?.authenticated) return;
    await Promise.all(buildButtonConfigs(setup).filter((button) => button.contentType).map(refreshContent));
  }

  async function refreshContent(button: ButtonConfig) {
    if (!button.contentType) return;
    const key = contentKey(button);
    contentState = {
      ...contentState,
      [key]: {
        active: [],
        inactive: [],
        activeEmptyState: null,
        inactiveEmptyState: null,
        loading: true,
        error: null
      }
    };
    try {
      const [activeResponse, inactiveResponse] = await Promise.all([
        listActiveContent(button.id, button.contentType, button.language),
        listInactiveContent(button.id, button.contentType, button.language)
      ]);
      contentState = {
        ...contentState,
        [key]: {
          active: activeResponse.items,
          inactive: inactiveResponse.items,
          activeEmptyState: activeResponse.empty_state,
          inactiveEmptyState: inactiveResponse.empty_state,
          loading: false,
          error: null
        }
      };
      void loadPreviewDurations(activeResponse.items);
    } catch (error) {
      contentState = {
        ...contentState,
        [key]: {
          active: [],
          inactive: [],
          activeEmptyState: null,
          inactiveEmptyState: null,
          loading: false,
          error: errorText(error)
        }
      };
    }
  }

  async function run(action: () => Promise<unknown>, success: string) {
    busy = true;
    try {
      await action();
      setMessage(success, "success");
      await refreshAll();
    } catch (error) {
      setError(error);
    } finally {
      busy = false;
    }
  }

  function setMessage(text: string, type: MessageType) {
    message = text;
    messageType = type;
  }

  function setError(error: unknown, fallback = "Request failed. Check the details and try again.") {
    message = errorText(error) || fallback;
    messageType = "error";
  }

  function errorText(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function buildButtonConfigs(review: SetupReview | null): ButtonConfig[] {
    return [1, 2, 3, 4, 5].map((id) => {
      const raw = review?.button_modes?.[String(id)] ?? defaultMode(id);
      const { mode, language } = splitMode(raw);
      return { id, mode, language, contentType: contentTypeForMode(mode) };
    });
  }

  function contentKey(button: ButtonConfig) {
    return `${button.id}:${button.contentType ?? "none"}:${button.language}`;
  }

  function activeCount(type: ContentType) {
    return setup?.active_counts?.[type] ?? 0;
  }

  function buttonActiveCount(button: ButtonConfig, state: Record<string, ContentState>) {
    if (!button.contentType) return 0;
    const content = state[contentKey(button)];
    return content?.active.length ?? 0;
  }

  function buttonDraftCount(button: ButtonConfig, state: Record<string, ContentState>) {
    if (!button.contentType) return 0;
    return state[contentKey(button)]?.inactive.length ?? 0;
  }

  function playsToday() {
    const today = new Date().toISOString().slice(0, 10);
    return events.filter((event) => event.occurred_at.startsWith(today)).length;
  }

  function fmt(value: unknown) {
    if (value === null || value === undefined || value === "") return "Not set";
    if (typeof value === "boolean") return value ? "Yes" : "No";
    return String(value);
  }

  function minutes(seconds: number) {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60).toString().padStart(2, "0");
    return `${mins}:${secs}`;
  }

  function formatDuration(seconds: number) {
    if (!Number.isFinite(seconds) || seconds <= 0) return "0:00";
    return minutes(seconds);
  }

  function sourceLabel(source: string) {
    if (source === "generated") return "Generated";
    if (source === "uploaded") return "Uploaded";
    if (source === "recorded") return "Recorded";
    return "Default";
  }

  function trimAudioTitle(title: string, maxLength = 32) {
    const clean = title.trim();
    if (clean.length <= maxLength) return clean;
    return `${clean.slice(0, maxLength - 1).trimEnd()}…`;
  }

  function contentPlaySummary(item: { id: string; source: string }) {
    return `${sourceLabel(item.source)} · ${formatDuration(contentDurations[item.id] ?? 0)} · x plays`;
  }

  function openButtonConfig(id: number) {
    selectedButtonId = id;
    setContentTab("record");
    contentListTab = "active";
    view = "button-config";
  }

  function goHome() {
    if (!session?.authenticated) return;
    view = "dashboard";
  }

  function openFirstContentButton(tab: ContentTab = "record") {
    const button = buttons.find((item) => item.contentType);
    selectedButtonId = button?.id ?? 1;
    setContentTab(tab);
    view = "button-config";
  }

  function setContentTab(tab: ContentTab) {
    if (selectedTab === "record" && tab !== "record" && recorder) stopRecording();
    selectedTab = tab;
  }

  function primaryLanguageForTts(currentButtons: ButtonConfig[]) {
    return currentButtons.find((button) => button.contentType === "language")?.language || "English";
  }

  async function checkGeneratedSpeechStatus(key: string, immediate = false) {
    clearGeneratedSpeechStatusTimer();
    const previousKey = lastGeneratedSpeechStatusKey;
    lastGeneratedSpeechStatusKey = key;
    if (immediate && previousKey !== key) {
      generatedSpeechStatus = null;
      generatedSpeechStatusError = null;
      generatedSpeechBackoffSeconds = generatedSpeechMinBackoffSeconds;
    }
    const [provider, language] = key.split(":");
    generatedSpeechStatusLoading = true;
    try {
      const nextStatus = await getGeneratedSpeechStatus(provider || "auto", language || "English");
      if (key !== generatedSpeechStatusKey) return;
      generatedSpeechStatus = nextStatus;
      generatedSpeechStatusError = null;
      generatedSpeechBackoffSeconds = nextStatus.online
        ? generatedSpeechMinBackoffSeconds
        : Math.min(
            generatedSpeechMaxBackoffSeconds,
            immediate ? generatedSpeechMinBackoffSeconds : generatedSpeechBackoffSeconds * 2
          );
      if (!nextStatus.online && key === generatedSpeechStatusKey) {
        scheduleGeneratedSpeechStatusCheck(key, generatedSpeechBackoffSeconds);
      }
    } catch (error) {
      if (key !== generatedSpeechStatusKey) return;
      generatedSpeechStatusError = errorText(error);
      generatedSpeechBackoffSeconds = Math.min(generatedSpeechMaxBackoffSeconds, generatedSpeechBackoffSeconds * 2);
      scheduleGeneratedSpeechStatusCheck(key, generatedSpeechBackoffSeconds);
    } finally {
      if (key === generatedSpeechStatusKey) {
        generatedSpeechStatusLoading = false;
      }
    }
  }

  function markGeneratedSpeechOffline(detail: string) {
    if (!generatedSpeechStatusKey) return;
    const [provider] = generatedSpeechStatusKey.split(":");
    const message = detail.includes("TTS provider")
      ? detail
      : `TTS provider is offline or unreachable: ${detail}`;
    generatedSpeechStatus = {
      online: false,
      provider: provider || "auto",
      checked_at: new Date().toISOString(),
      cached: false,
      cache_ttl_seconds: 20,
      next_check_after_seconds: generatedSpeechMinBackoffSeconds,
      message
    };
    generatedSpeechStatusError = null;
    generatedSpeechBackoffSeconds = generatedSpeechMinBackoffSeconds;
    scheduleGeneratedSpeechStatusCheck(generatedSpeechStatusKey, generatedSpeechBackoffSeconds);
  }

  function isSpeechProviderOfflineError(error: unknown) {
    const text = errorText(error).toLowerCase();
    return text.includes("failed to connect to speech provider") || text.includes("tts provider is offline");
  }

  function scheduleGeneratedSpeechStatusCheck(key: string, seconds: number) {
    clearGeneratedSpeechStatusTimer();
    generatedSpeechCheckTimer = window.setTimeout(() => {
      void checkGeneratedSpeechStatus(key);
    }, seconds * 1000);
  }

  function clearGeneratedSpeechStatusTimer() {
    if (generatedSpeechCheckTimer !== null) {
      window.clearTimeout(generatedSpeechCheckTimer);
      generatedSpeechCheckTimer = null;
    }
  }

  async function checkMenuLlmStatus(key: string, immediate = false) {
    clearMenuLlmStatusTimer();
    const previousKey = lastMenuLlmStatusKey;
    lastMenuLlmStatusKey = key;
    if (immediate && previousKey !== key) {
      menuLlmStatus = null;
      menuLlmBackoffSeconds = generatedSpeechMinBackoffSeconds;
    }
    const [provider, language] = key.split(":");
    menuLlmStatusLoading = true;
    try {
      const nextStatus = await getGeneratedSpeechStatus(provider || "auto", language || "English");
      if (key !== menuLlmStatusKey) return;
      menuLlmStatus = nextStatus;
      menuLlmBackoffSeconds = nextStatus.online
        ? generatedSpeechMinBackoffSeconds
        : Math.min(
            generatedSpeechMaxBackoffSeconds,
            immediate ? generatedSpeechMinBackoffSeconds : menuLlmBackoffSeconds * 2
          );
      if (!nextStatus.online && key === menuLlmStatusKey) {
        scheduleMenuLlmStatusCheck(key, menuLlmBackoffSeconds);
      }
    } catch {
      if (key !== menuLlmStatusKey) return;
      menuLlmStatus = {
        online: false,
        provider: provider || "auto",
        checked_at: new Date().toISOString(),
        cached: false,
        cache_ttl_seconds: 20,
        next_check_after_seconds: menuLlmBackoffSeconds,
        message: "TTS provider is offline or unreachable."
      };
      menuLlmBackoffSeconds = Math.min(generatedSpeechMaxBackoffSeconds, menuLlmBackoffSeconds * 2);
      scheduleMenuLlmStatusCheck(key, menuLlmBackoffSeconds);
    } finally {
      if (key === menuLlmStatusKey) {
        menuLlmStatusLoading = false;
      }
    }
  }

  function scheduleMenuLlmStatusCheck(key: string, seconds: number) {
    clearMenuLlmStatusTimer();
    menuLlmCheckTimer = window.setTimeout(() => {
      void checkMenuLlmStatus(key);
    }, seconds * 1000);
  }

  function clearMenuLlmStatusTimer() {
    if (menuLlmCheckTimer !== null) {
      window.clearTimeout(menuLlmCheckTimer);
      menuLlmCheckTimer = null;
    }
  }

  function inventoryItems(status: string) {
    return inventory?.items.filter((item) => item.status === status) ?? [];
  }

  function openInventoryButton(item: ContentInventoryItem) {
    selectedButtonId = item.button_id;
    contentListTab = item.status === "draft" ? "draft" : "active";
    view = "button-config";
  }

  function selectSetupAction(id: string) {
    if (id === "language") selectedButtonId = buttons.find((button) => button.contentType === "language")?.id ?? 1;
    if (id === "animals") selectedButtonId = buttons.find((button) => button.contentType === "animals")?.id ?? 2;
    if (id === "music") selectedButtonId = buttons.find((button) => button.contentType === "music")?.id ?? 3;
    if (id === "language" || id === "animals" || id === "music") view = "button-config";
  }

  function setSelectedMode(mode: ButtonMode) {
    if (!selectedButton) return;
    const language = mode === "language" ? selectedButton.language || "English" : selectedButton.language;
    patchButtonMode({
      ...selectedButton,
      mode,
      language,
      contentType: contentTypeForMode(mode)
    });
  }

  function setSelectedLanguage(language: string) {
    if (!selectedButton) return;
    patchButtonMode({ ...selectedButton, mode: "language", language, contentType: "language" });
  }

  async function saveSelectedButtonMode(button: ButtonConfig) {
    busy = true;
    try {
      await saveButtonMode(button.id, button.mode, button.language);
      patchButtonMode(button);
      setMessage(`Button ${button.id} mode saved.`, "success");
      if (button.contentType) {
        await refreshContent(button);
      }
      await refreshEvents();
    } catch (error) {
      setError(error);
    } finally {
      busy = false;
    }
  }

  function patchButtonMode(button: ButtonConfig) {
    if (!setup) return;
    const modeValue = button.mode === "language" ? `${button.mode}:${button.language}` : button.mode;
    setup = {
      ...setup,
      button_modes: {
        ...setup.button_modes,
        [String(button.id)]: modeValue
      }
    };
  }

  async function activateSelectedContent(id: string) {
    await run(() => activateContentItem(id), "Draft activated. The child can hear it on the next button press.");
  }

  async function trashSelectedContent(id: string) {
    await run(() => trashContentItem(id), "Content moved to trash.");
  }

  async function clearSelectedGenerated() {
    if (!selectedButton) return;
    if (generatedSpeechOffline) {
      setError("TTS is offline. Start the local TTS service before clearing generated drafts.");
      return;
    }
    if (!window.confirm("Move unused generated speech drafts for this button to trash?")) return;
    await run(
      () => clearUnusedGeneratedSpeech(selectedButton.id, selectedButton.language),
      "Unused generated speech moved to trash."
    );
  }

  function promptTrashContent(item: { id: string; title: string }) {
    trashPrompt = item;
  }

  async function confirmTrashContent() {
    if (!trashPrompt) return;
    const { id } = trashPrompt;
    trashPrompt = null;
    await run(() => trashContentItem(id), "Content moved to trash.");
  }

  function cancelTrashContent() {
    trashPrompt = null;
  }

  async function createManagerInvitation() {
    const deviceId = setup?.device_id;
    if (!deviceId) throw new Error("Save the cube name before creating a manager invitation.");
    invitation = await createInvitation(deviceId);
  }

  async function startRecording() {
    if (recordingStatus === "processing" || recordingStatus === "saving") return;
    if (!isSecureRecorderContext()) {
      setError("Recording failed. Open the dashboard over HTTPS or localhost.");
      return;
    }
    if (!canRecordAudio()) {
      setError("Recording failed. This browser does not expose microphone recording APIs.");
      return;
    }
    revokeRecording();
    let stream: MediaStream | null = null;
    try {
      stream = await navigator.mediaDevices.getUserMedia({ audio: true });
      const chunks: BlobPart[] = [];
      recorder = new MediaRecorder(stream);
      recorder.ondataavailable = (event) => {
        if (event.data.size > 0) chunks.push(event.data);
      };
      recorder.onstop = async () => {
        recordingStatus = "processing";
        stream?.getTracks().forEach((track) => track.stop());
        void cleanupRecordingAnalyser();
        try {
          recordedWav = await blobToWav(new Blob(chunks, { type: recorder?.mimeType || "audio/webm" }));
          recordingStatus = "ready";
          setMessage("Recording ready for review.", "success");
        } catch (error) {
          recordingStatus = "idle";
          setError(error);
        } finally {
          stopTimer();
          recorder = null;
        }
      };
      recorder.start();
      recordingStatus = "recording";
      startRecordingAnalyser(stream);
      recordStartedAt = Date.now();
      recordSeconds = 0;
      recordTimer = window.setInterval(() => {
        recordSeconds = Math.floor((Date.now() - recordStartedAt) / 1000);
      }, 250);
    } catch (error) {
      stream?.getTracks().forEach((track) => track.stop());
      void cleanupRecordingAnalyser();
      recordingStatus = "idle";
      setError(error);
    }
  }

  function stopRecording() {
    if (recorder && recorder.state !== "inactive") {
      recordingStatus = "processing";
    }
    recorder?.stop();
  }

  function startRecordingAnalyser(stream: MediaStream) {
    try {
      void cleanupRecordingAnalyser();
      recordAudioContext = new AudioContext();
      const source = recordAudioContext.createMediaStreamSource(stream);
      recordAnalyser = recordAudioContext.createAnalyser();
      recordAnalyser.fftSize = 256;
      source.connect(recordAnalyser);
      updateRecordWaveform();
    } catch {
      void cleanupRecordingAnalyser();
    }
  }

  function updateRecordWaveform() {
    if (!recordAnalyser) return;
    const data = new Uint8Array(recordAnalyser.fftSize);
    recordAnalyser.getByteTimeDomainData(data);
    const segmentSize = Math.floor(data.length / recordWaveform.length);
    recordWaveform = recordWaveform.map((_, index) => {
      const start = index * segmentSize;
      const end = start + segmentSize;
      let peak = 0;
      for (let sampleIndex = start; sampleIndex < end; sampleIndex += 1) {
        peak = Math.max(peak, Math.abs(data[sampleIndex] - 128) / 128);
      }
      return Math.max(0.08, Math.min(1, peak * 2.4));
    });
    recordWaveFrame = window.requestAnimationFrame(updateRecordWaveform);
  }

  async function cleanupRecordingAnalyser() {
    if (recordWaveFrame !== null) {
      window.cancelAnimationFrame(recordWaveFrame);
      recordWaveFrame = null;
    }
    recordAnalyser = null;
    if (recordAudioContext) {
      const context = recordAudioContext;
      recordAudioContext = null;
      if (context.state !== "closed") await context.close();
    }
    recordWaveform = Array.from({ length: 24 }, () => 0.12);
  }

  function stopTimer() {
    if (recordTimer !== null) {
      window.clearInterval(recordTimer);
      recordTimer = null;
    }
  }

  function revokeRecording() {
    if (recordedWav) URL.revokeObjectURL(recordedWav.url);
    recordedWav = null;
    recordSeconds = 0;
    if (recorder && recorder.state !== "inactive") recorder.stop();
    if (!recorder) recordingStatus = "idle";
  }

  function recordingHint(status: RecordingStatus, seconds: number, wav: RecordedWav | null) {
    if (status === "recording") return `Recording ${minutes(seconds)}. Tap again to stop.`;
    if (status === "processing") return "Preparing preview...";
    if (status === "saving") return "Saving recording as draft...";
    if (wav) return `Preview ${minutes(wav.durationSeconds)}, then save it as a draft.`;
    return "Tap record, then speak clearly near your phone.";
  }

  function recordingSaveHint(button: ButtonConfig | null, text: string, wav: RecordedWav | null) {
    if (!wav) return "After recording, preview the audio here before saving.";
    if (button?.contentType === "language" && !text.trim()) {
      return "Enter the text spoken before saving this recording.";
    }
    return "Saving creates an inactive draft for review.";
  }

  function chooseUpload(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    setUploadFile(input.files?.[0] ?? null);
  }

  function dropUpload(event: DragEvent) {
    event.preventDefault();
    draggingUpload = false;
    setUploadFile(event.dataTransfer?.files?.[0] ?? null);
  }

  function setUploadFile(file: File | null) {
    if (uploadPreviewUrl) URL.revokeObjectURL(uploadPreviewUrl);
    uploadFile = null;
    uploadPreviewUrl = null;
    if (!file) return;
    const allowed = file.name.toLowerCase().endsWith(".wav") || file.name.toLowerCase().endsWith(".mp3");
    if (!allowed) {
      setError("Upload failed. File must be MP3 or WAV.");
      return;
    }
    if (file.size > 25 * 1024 * 1024) {
      setError("Upload failed. File must be 25 MB or smaller.");
      return;
    }
    uploadFile = file;
    uploadPreviewUrl = URL.createObjectURL(file);
  }

  function mediaForm(file: Blob, filename: string) {
    if (!selectedButton?.contentType) throw new Error("Choose an active content button first.");
    const languageContent = selectedButton.contentType === "language";
    const title = languageContent ? "" : draftForm.title.trim() || defaultDraftTitle(filename);
    const text = languageContent ? draftForm.text.trim() : "";
    const form = new FormData();
    form.set("content_type", selectedButton.contentType);
    form.set("button_id", String(selectedButton.id));
    form.set("title", title);
    form.set("text", text);
    form.set("language", selectedButton.contentType === "language" ? selectedButton.language : draftForm.language);
    form.set("audio_file", file, filename);
    return form;
  }

  function defaultDraftTitle(filename: string) {
    if (filename === "recording.wav") return "Recorded audio";
    return filename.replace(/\.[^.]+$/, "").replace(/[-_]+/g, " ").trim() || "Uploaded audio";
  }

  async function submitRecording() {
    if (!recordedWav) {
      setError("Save recording failed. Record and preview audio first.");
      return;
    }
    if (!canSaveMediaDraft()) return;
    const wav = recordedWav;
    recordingStatus = "saving";
    await run(async () => {
      await saveMultipart("/content/recordings", mediaForm(wav.blob, "recording.wav"));
      revokeRecording();
      contentListTab = "draft";
    }, "Recording saved as draft.");
    recordingStatus = recordedWav ? "ready" : "idle";
  }

  async function submitUpload() {
    if (!uploadFile) {
      setError("Upload failed. Choose an MP3 or WAV file first.");
      return;
    }
    if (!canSaveMediaDraft()) return;
    const file = uploadFile;
    await run(async () => {
      await saveMultipart("/content/uploads", mediaForm(file, file.name));
      setUploadFile(null);
      contentListTab = "draft";
    }, "Upload saved as draft.");
  }

  async function submitGeneration() {
    if (!selectedButton) return;
    if (selectedButton.contentType !== "language") {
      setError("Generated speech is only available for language buttons.");
      return;
    }
    if (generatedSpeechDisabled) {
      setError("TTS is offline. Start the local TTS service before generating speech.");
      return;
    }
    busy = true;
    try {
      await generateSpeech({
        button_id: selectedButton.id,
        language: selectedButton.language,
        text: draftForm.text,
        provider: draftForm.provider,
        voice: draftForm.voice.trim() || undefined
      });
      contentListTab = "draft";
      setMessage("Generated speech saved as draft.", "success");
      await refreshAll();
    } catch (error) {
      if (isSpeechProviderOfflineError(error)) {
        markGeneratedSpeechOffline(errorText(error));
      }
      setError(error);
    } finally {
      busy = false;
    }
  }

  function canSaveMediaDraft() {
    if (!selectedButton?.contentType) {
      setError("Choose a content button first.");
      return false;
    }
    if (selectedButton.contentType === "language" && !draftForm.text.trim()) {
      setError("Save draft failed. Enter the text spoken in the recording or upload.");
      return false;
    }
    if (selectedButton.contentType !== "language" && !draftForm.title.trim()) {
      setError("Save draft failed. Enter a title for this audio.");
      return false;
    }
    return true;
  }

  function footerActionLabel() {
    if (selectedButton?.contentType && selectedTab === "record" && recordedWav) return "Save recording";
    if (selectedButton?.contentType && selectedTab === "upload" && uploadFile) return "Save upload";
    if (selectedButton?.contentType === "language" && selectedTab === "generate" && draftForm.text.trim()) {
      return "Generate speech";
    }
    return "Save mode";
  }

  function footerActionDisabled() {
    if (busy || !selectedButton) return true;
    if (selectedButton.contentType === "language" && selectedTab === "generate" && draftForm.text.trim()) {
      return generatedSpeechDisabled;
    }
    return false;
  }

  async function runFooterAction() {
    if (!selectedButton) return;
    if (selectedButton.contentType && selectedTab === "record" && recordedWav) {
      await submitRecording();
      return;
    }
    if (selectedButton.contentType && selectedTab === "upload" && uploadFile) {
      await submitUpload();
      return;
    }
    if (selectedButton.contentType === "language" && selectedTab === "generate" && draftForm.text.trim()) {
      await submitGeneration();
      return;
    }
    await saveSelectedButtonMode(selectedButton);
  }

  function modeClass(mode: ButtonMode) {
    if (mode === "language") return "lang";
    if (mode === "animals") return "animal";
    if (mode === "music") return "music";
    if (mode === "setup_help") return "setup";
    return "off";
  }

  function faceName(button: ButtonConfig) {
    return faceNames[button.id - 1] ?? `Button ${button.id}`;
  }

  function contentLabel(button: ButtonConfig) {
    if (button.mode === "language") return shortLanguage(button.language);
    return modeLabel(button.mode);
  }

  async function playContentPreview(item: { id: string; preview_url: string | null; title: string }) {
    if (!item.preview_url) return;
    if (previewAudioId !== item.id) {
      stopContentPreview();
      previewAudio = new Audio(item.preview_url);
      previewAudio.preload = "auto";
      previewAudioId = item.id;
      previewAudio.onended = () => {
        if (previewAudioId === item.id) stopContentPreview();
      };
    }
    try {
      if (previewAudio) {
        previewAudio.currentTime = 0;
        await previewAudio.play();
      }
    } catch (error) {
      setError(error, "Could not play audio preview.");
    }
  }

  function stopContentPreview() {
    if (previewAudio) {
      previewAudio.pause();
      previewAudio.src = "";
      previewAudio.load();
    }
    previewAudio = null;
    previewAudioId = null;
  }

  function onContentRowKeydown(event: KeyboardEvent, item: { preview_url: string | null; id: string; title: string }) {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    void playContentPreview(item);
  }

  async function loadPreviewDurations(items: ActiveContentItem[]) {
    await Promise.all(
      items.map(async (item) => {
        if (!item.preview_url) return;
        const duration = await readAudioDuration(item.preview_url);
        if (duration !== null) {
          contentDurations = {
            ...contentDurations,
            [item.id]: duration
          };
        }
      })
    );
  }

  function readAudioDuration(src: string) {
    return new Promise<number | null>((resolve) => {
      const audio = document.createElement("audio");
      audio.preload = "metadata";
      audio.src = src;
      audio.onloadedmetadata = () => resolve(Number.isFinite(audio.duration) ? audio.duration : null);
      audio.onerror = () => resolve(null);
    });
  }

  function shortLanguage(language: string) {
    if (language === "English") return "EN";
    if (language === "French") return "FR";
    if (language === "Vietnamese") return "VI";
    if (language === "Spanish") return "ES";
    if (language === "German") return "DE";
    return language.slice(0, 2).toUpperCase();
  }

  function relativeTime(value: string) {
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

  onDestroy(() => {
    stopContentPreview();
    stopTimer();
    clearGeneratedSpeechStatusTimer();
    clearMenuLlmStatusTimer();
    if (recorder && recorder.state !== "inactive") recorder.stop();
    void cleanupRecordingAnalyser();
    if (recordedWav) URL.revokeObjectURL(recordedWav.url);
    if (uploadPreviewUrl) URL.revokeObjectURL(uploadPreviewUrl);
  });
</script>

<svelte:head>
  <title>T-Cube Pi Admin</title>
</svelte:head>

<main class="shell">
  {#if !session?.authenticated}
    <nav class="topbar">
    <div class="topbar-left">
      <button type="button" class="topbar-logo topbar-logo-btn" aria-label="Go to dashboard" on:click={goHome}>
        T<span>·</span>Cube
      </button>
      <div class="topbar-session">Local parent dashboard</div>
    </div>
      <button type="button" class="icon-btn" aria-label="Refresh" on:click={refreshAll}>
        <RefreshCw size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
    </nav>

    <div class="body auth-body">
      <section class:error={messageType === "error"} class:success={messageType === "success"} class="notice" aria-live="polite">
        {message}
      </section>

      {#if invitationCodeFromUrl}
        <section class="card auth-card">
          <div class="sec-hdr">
            <div class="sec-title"><User size={16} strokeWidth={1.5} aria-hidden="true" />Accept manager access</div>
          </div>
          <form class="form-stack" on:submit|preventDefault={() => run(async () => (session = await acceptInvitation(inviteForm)), "Manager account created.")}>
            <label>Invitation code <input bind:value={inviteForm.code} autocomplete="off" /></label>
            <label>Username <input bind:value={inviteForm.username} autocomplete="username" /></label>
            <label>Display name <input bind:value={inviteForm.display_name} /></label>
            <label>Password <input bind:value={inviteForm.password} type="password" autocomplete="new-password" /></label>
            <button type="submit" class="btn-primary" disabled={busy}>Create manager account</button>
          </form>
        </section>
      {/if}

      {#if session?.bootstrap_required}
        <section class="card auth-card">
          <div class="sec-hdr">
            <div class="sec-title"><ShieldCheck size={16} strokeWidth={1.5} aria-hidden="true" />Create local owner</div>
          </div>
          <form class="form-stack" on:submit|preventDefault={() => run(async () => (session = await bootstrapOwner(bootstrapForm)), "Owner account created.")}>
            <label>Username <input bind:value={bootstrapForm.username} autocomplete="username" /></label>
            <label>Display name <input bind:value={bootstrapForm.display_name} /></label>
            <label>Password <input bind:value={bootstrapForm.password} type="password" autocomplete="new-password" /></label>
            <button type="submit" class="btn-primary" disabled={busy}>Create owner</button>
          </form>
        </section>
      {:else}
        <section class="card auth-card">
          <div class="sec-hdr">
            <div class="sec-title"><LogIn size={16} strokeWidth={1.5} aria-hidden="true" />Log in</div>
          </div>
          <form class="form-stack" on:submit|preventDefault={() => run(async () => (session = await loginPassword(loginForm)), "Logged in.")}>
            <label>Username <input bind:value={loginForm.username} autocomplete="username" /></label>
            <label>Password <input bind:value={loginForm.password} type="password" autocomplete="current-password" /></label>
            <button type="submit" class="btn-primary" disabled={busy}>Log in</button>
          </form>
        </section>

        <section class="card auth-card">
          <div class="sec-hdr">
            <div class="sec-title"><KeyRound size={16} strokeWidth={1.5} aria-hidden="true" />Reset password</div>
          </div>
          <form class="form-stack" on:submit|preventDefault={() => run(() => recoverPassword(recoveryForm), "Password updated. Previous sessions were revoked.")}>
            <label>Recovery code <input bind:value={recoveryForm.code} autocomplete="off" /></label>
            <label>New password <input bind:value={recoveryForm.password} type="password" autocomplete="new-password" /></label>
            <button type="submit" class="btn-secondary" disabled={busy}>Reset password</button>
          </form>
        </section>
      {/if}
    </div>
  {:else if view === "button-config"}
    {@render ButtonConfigView()}
  {:else if view === "inventory"}
    {@render InventoryView()}
  {:else if view === "settings"}
    {@render SettingsView()}
  {:else}
    {@render DashboardView()}
  {/if}
</main>

{#snippet TopBar(showBack = false)}
  <nav class="topbar">
    <div class="topbar-left">
      <button type="button" class="topbar-logo topbar-logo-btn" aria-label="Go to dashboard" on:click={goHome}>
        T<span>·</span>Cube
      </button>
      <div class="topbar-session">
        <span class="topbar-session-prefix">Signed in as</span>
        <span class="topbar-session-user">{session?.account?.display_name ?? session?.account?.username}</span>
        <span class="topbar-session-role role-{roleClass}">{roleLabel}</span>
      </div>
    </div>
    <div class="topbar-right">
      {#if showBack}
        <button type="button" class="icon-btn" aria-label="Back to dashboard" on:click={() => (view = "dashboard")}>
          <ArrowLeft size={18} strokeWidth={1.5} aria-hidden="true" />
        </button>
      {/if}
      <button type="button" class="icon-btn" aria-label="Notifications">
        <Bell size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
      <button type="button" class="icon-btn" aria-label="Settings" on:click={() => (view = "settings")}>
        <Settings size={18} strokeWidth={1.5} aria-hidden="true" />
      </button>
    </div>
  </nav>
{/snippet}

{#snippet StatusBar()}
  <div class="status-bar" role="status" aria-label="System health">
    {#if !Boolean(status?.database_present)}
      <div class="status-item">
        <span class:sdot-ok={Boolean(status?.database_present)} class:sdot-warn={!Boolean(status?.database_present)} class="sdot"></span>
        <Database size={14} strokeWidth={1.5} aria-hidden="true" />
        <span class:status-ok={Boolean(status?.database_present)} class:status-warn={!Boolean(status?.database_present)}>Database</span>
      </div>
    {/if}
    {#if !Boolean(status?.media_root)}
      <div class="status-item">
        <span class:sdot-ok={Boolean(status?.media_root)} class:sdot-warn={!Boolean(status?.media_root)} class="sdot"></span>
        <HardDrive size={14} strokeWidth={1.5} aria-hidden="true" />
        <span class:status-ok={Boolean(status?.media_root)} class:status-warn={!Boolean(status?.media_root)}>Audio</span>
      </div>
    {/if}
    {#if !Boolean(status?.content_root)}
      <div class="status-item">
        <span class:sdot-ok={Boolean(status?.content_root)} class:sdot-warn={!Boolean(status?.content_root)} class="sdot"></span>
        <Folder size={14} strokeWidth={1.5} aria-hidden="true" />
        <span class:status-ok={Boolean(status?.content_root)} class:status-warn={!Boolean(status?.content_root)}>Content</span>
      </div>
    {/if}
    {#if !Boolean(setup?.wifi_verified)}
      <div class="status-item">
        <span class:sdot-ok={Boolean(setup?.wifi_verified)} class:sdot-warn={!Boolean(setup?.wifi_verified)} class="sdot"></span>
        <Wifi size={14} strokeWidth={1.5} aria-hidden="true" />
        <span class:status-ok={Boolean(setup?.wifi_verified)} class:status-warn={!Boolean(setup?.wifi_verified)}>Wi-Fi</span>
      </div>
    {/if}
    <div class="status-item">
      <span class:sdot-ok={menuLlmOnline} class:sdot-warn={!menuLlmOnline} class="sdot"></span>
      {#if menuLlmOnline}
        <CircleCheck class="status-ok" size={14} strokeWidth={1.5} aria-hidden="true" />
      {:else if menuLlmStatusLoading && !menuLlmStatus}
        <RefreshCw class="status-warn" size={14} strokeWidth={1.5} aria-hidden="true" />
      {:else}
        <AlertTriangle class="status-warn" size={14} strokeWidth={1.5} aria-hidden="true" />
      {/if}
      <span class:status-ok={menuLlmOnline} class:status-warn={!menuLlmOnline}>{menuLlmLabel}</span>
    </div>
  </div>
{/snippet}

{#snippet DashboardView()}
  {@render TopBar()}
  {@render StatusBar()}

  <div class="body">
    <section class:error={messageType === "error"} class:success={messageType === "success"} class="notice" aria-live="polite">
      {message}
    </section>

    <section class="card" data-testid="dashboard-hero-card">
      <div class="cube-hero">
        <div class="cube-avatar" aria-hidden="true">
          <Cuboid size={28} strokeWidth={1.5} />
          <div class="cube-online-dot"></div>
        </div>
        <div class="cube-info">
          <div class="cube-name">{fmt(setup?.cube_name)}</div>
          <div class="cube-sub"><Wifi size={13} strokeWidth={1.5} aria-hidden="true" />{setup?.wifi_verified ? "Home" : "Wi-Fi pending"} · {fmt(setup?.dashboard_ip ?? setup?.dashboard_address)}</div>
        </div>
        <div class="cube-badge" aria-label="Cube is reachable">
          <CircleCheck size={14} strokeWidth={1.5} aria-hidden="true" /> Online
        </div>
      </div>
      <div class="cube-stats" role="list" aria-label="Cube statistics" data-testid="dashboard-stats">
        <div class="cstat" role="listitem">
          <div class="cstat-num">{playsToday()}</div>
          <div class="cstat-lbl">Presses today</div>
        </div>
        <div class="cstat" role="listitem">
          <div class="cstat-num stat-active">{totalActive}</div>
          <div class="cstat-lbl">Active sounds</div>
        </div>
        <div class="cstat" role="listitem">
          <div class="cstat-num stat-draft">{totalDrafts}</div>
          <div class="cstat-lbl">Drafts</div>
        </div>
        <div class="cstat" role="listitem">
          <div class="cstat-num stat-unused">{totalUnused}</div>
          <div class="cstat-lbl">Unused</div>
        </div>
      </div>
    </section>

    <section class="card" data-testid="dashboard-inventory-card">
      <div class="sec-hdr">
        <div class="sec-title"><FileAudio size={15} strokeWidth={1.5} aria-hidden="true" />Content inventory</div>
        <button type="button" class="sec-link" on:click={() => (view = "inventory")}>
          View all <ArrowRight size={13} strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
      <div class="inventory-summary">
        <div><strong>{totalActive}</strong><span>Active in setup</span></div>
        <div><strong>{totalDrafts}</strong><span>Drafts</span></div>
        <div><strong>{totalUnused}</strong><span>Unused audio</span></div>
      </div>
      {#if inventoryError}
        <div class="content-api-error" role="alert">
          <AlertTriangle size={15} strokeWidth={1.5} aria-hidden="true" />
          <span>{inventoryError}</span>
        </div>
      {/if}
    </section>

    <section class="card" data-testid="dashboard-buttons-card">
      <div class="sec-hdr">
        <div class="sec-title">{@render LayoutGridIcon()}Buttons</div>
        <button type="button" class="sec-link" on:click={() => openButtonConfig(selectedButtonId)}>
          Manage all <ArrowRight size={13} strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
      <div class="btn-strip-outer">
        <div class="btn-strip" aria-label="Cube button faces" data-testid="dashboard-button-strip">
          {#each buttons as button}
            <button type="button" class="btn-face-card" data-testid={`dashboard-button-${button.id}`} on:click={() => openButtonConfig(button.id)}>
              <div class="bfc-icon bfc-{modeClass(button.mode)}" data-testid={`dashboard-button-${button.id}-icon`}>
                {#if button.mode === "language"}
                  <Languages size={18} strokeWidth={1.5} aria-hidden="true" />
                {:else if button.mode === "animals"}
                  <PawPrint size={18} strokeWidth={1.5} aria-hidden="true" />
                {:else if button.mode === "music"}
                  <Music size={18} strokeWidth={1.5} aria-hidden="true" />
                {:else if button.mode === "setup_help"}
                  <Wrench size={18} strokeWidth={1.5} aria-hidden="true" />
                {:else}
                  <Minus size={18} strokeWidth={1.5} aria-hidden="true" />
                {/if}
              </div>
              <div class="bfc-name">{faceName(button)}</div>
              <div class="bfc-count">{button.contentType ? `${buttonActiveCount(button, contentState)} sounds` : "—"}</div>
              <div class="bfc-mode bfm-{modeClass(button.mode)}">{contentLabel(button)}</div>
            </button>
          {/each}
        </div>
      </div>
    </section>

    <section class="card">
      <div class="sec-hdr">
        <div class="sec-title"><Bolt size={15} strokeWidth={1.5} aria-hidden="true" />Quick actions</div>
      </div>
      <div class="actions-grid">
        <button type="button" class="action-card primary-action" disabled>
          <div class="ac-icon ac-icon-white"><RefreshCw size={20} strokeWidth={1.5} aria-hidden="true" /></div>
          <div class="ac-body">
            <div class="ac-title">Run curation</div>
            <div class="ac-desc">Update schedule with LLM</div>
          </div>
          <ArrowRight class="ac-arrow" size={18} strokeWidth={1.5} aria-hidden="true" />
        </button>
        <button type="button" class="action-card" disabled>
          <div class="ac-icon ac-icon-violet"><BarChart3 size={20} strokeWidth={1.5} aria-hidden="true" /></div>
          <div class="ac-body">
            <div class="ac-title">Learning stats</div>
            <div class="ac-desc">Words heard, repetitions</div>
          </div>
          <ArrowRight class="ac-arrow" size={18} strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
    </section>

    <section class="card">
      <div class="sec-hdr">
        <div class="sec-title"><Activity size={15} strokeWidth={1.5} aria-hidden="true" />Recent activity</div>
      </div>
      {@render EventFeed(events)}
    </section>

    {#if !setupReady}
      <section class="setup-banner" aria-label="Setup checklist">
        <div class="setup-banner-hdr">
          <AlertTriangle size={18} strokeWidth={1.5} aria-hidden="true" />
          <div class="setup-banner-title">Setup incomplete</div>
          <div class="setup-pct">{prerequisites.filter((item) => item.complete).length} of {prerequisites.length} done</div>
        </div>
        <div class="prereq-list" role="list">
          {#each prerequisites as item}
            <button type="button" class:prereq-done={item.complete} class="prereq-item" on:click={() => selectSetupAction(item.id)}>
              <div class:pc-done={item.complete} class:pc-todo={!item.complete} class="prereq-check">
                {#if item.complete}<Check size={12} strokeWidth={1.5} aria-hidden="true" />{:else}<Minus size={12} strokeWidth={1.5} aria-hidden="true" />{/if}
              </div>
              <div class="prereq-body">
                <div class="prereq-name">{item.label}</div>
                <div class="prereq-detail">{item.detail}</div>
              </div>
              {#if !item.complete}
                <div class="prereq-action">{item.action}<ArrowRight size={13} strokeWidth={1.5} aria-hidden="true" /></div>
              {/if}
            </button>
          {/each}
        </div>
        <button
          type="button"
          class:ready={setupReady}
          class="setup-complete-btn"
          title={!setupReady ? `Missing: ${blockedSetupText}` : "Completing setup switches the cube to child mode."}
          disabled={busy || !isOwner || !setupReady}
          on:click={() => window.confirm("Completing setup switches the cube to child mode. You can still manage content from this dashboard.") && run(completeSetup, "Setup complete. The cube is ready for child mode.")}
        >
          <Play size={16} strokeWidth={1.5} aria-hidden="true" />
          Complete setup — {prerequisites.filter((item) => !item.complete).length} items remaining
        </button>
      </section>
    {/if}
  </div>
{/snippet}

{#snippet ButtonConfigView()}
  {@render TopBar(true)}

  <div class="face-strip-wrap">
    <p class="face-strip-label">Select a button</p>
    <div class="face-strip" role="tablist" aria-label="Cube faces" data-testid="button-selector">
      {#each buttons as button}
        <button
          type="button"
          class:selected={selectedButtonId === button.id}
          class="face-pill"
          data-testid={`button-selector-${button.id}`}
          role="tab"
          aria-selected={selectedButtonId === button.id}
          on:click={() => (selectedButtonId = button.id)}
        >
          <div class="face-pill-icon fpi-{modeClass(button.mode)}" data-testid={`button-selector-${button.id}-icon`}>
            {#if button.mode === "language"}
              <Languages size={16} strokeWidth={1.5} aria-hidden="true" />
            {:else if button.mode === "animals"}
              <PawPrint size={16} strokeWidth={1.5} aria-hidden="true" />
            {:else if button.mode === "music"}
              <Music size={16} strokeWidth={1.5} aria-hidden="true" />
            {:else if button.mode === "setup_help"}
              <Wrench size={16} strokeWidth={1.5} aria-hidden="true" />
            {:else}
              <Minus size={16} strokeWidth={1.5} aria-hidden="true" />
            {/if}
          </div>
          <div class="face-pill-name">{faceName(button)}</div>
          <div class="face-pill-count">{button.contentType ? `${buttonActiveCount(button, contentState)} active` : contentLabel(button)}</div>
        </button>
      {/each}
    </div>
  </div>

  <div class="body config-body">
    {#if selectedButton}
      <section class="section-card">
        <div class="face-hero">
          <div class="face-hero-icon fpi-{modeClass(selectedButton.mode)}" data-testid="selected-button-hero-icon">
            {#if selectedButton.mode === "language"}
              <Languages size={22} strokeWidth={1.5} aria-hidden="true" />
            {:else if selectedButton.mode === "animals"}
              <PawPrint size={22} strokeWidth={1.5} aria-hidden="true" />
            {:else if selectedButton.mode === "music"}
              <Music size={22} strokeWidth={1.5} aria-hidden="true" />
            {:else if selectedButton.mode === "setup_help"}
              <Wrench size={22} strokeWidth={1.5} aria-hidden="true" />
            {:else}
              <Minus size={22} strokeWidth={1.5} aria-hidden="true" />
            {/if}
          </div>
          <div class="face-hero-info">
            <div class="face-hero-name">{faceName(selectedButton)} · {modeLabel(selectedButton.mode)}</div>
            <div class="face-hero-sub">{selectedButton.mode === "language" ? `${selectedButton.language} · ` : ""}Button {selectedButton.id}</div>
          </div>
          <div class:active-badge={Boolean(selectedButton.contentType)} class:disabled-badge={!selectedButton.contentType}>
            <span class="active-dot"></span>
            {selectedButton.contentType ? "Active" : "No content"}
          </div>
        </div>
        <div class="stats-row">
          <div class="stat-cell">
            <div class="stat-num stat-active">{buttonActiveCount(selectedButton, contentState)}</div>
            <div class="stat-lbl">Active</div>
          </div>
          <div class="stat-cell">
            <div class="stat-num stat-draft">{buttonDraftCount(selectedButton, contentState)}</div>
            <div class="stat-lbl">Draft</div>
          </div>
          <div class="stat-cell">
            <div class="stat-num">{events.filter((event) => event.button_id === selectedButton.id).length}</div>
            <div class="stat-lbl">Recent plays</div>
          </div>
        </div>
      </section>

      <section class="section-card">
        <div class="sc-header">
          <div class="sc-title"><SlidersHorizontal size={16} strokeWidth={1.5} aria-hidden="true" />Mode</div>
        </div>
        <div class="mode-grid" role="radiogroup" aria-label="Button mode">
          {#each modes as mode, index}
            <button
              type="button"
              class:selected-mode={selectedButton.mode === mode}
              class:mode-cell-5th={index === 4}
              class:mode-cell={index !== 4}
              data-testid={`button-mode-${mode}`}
              role="radio"
              aria-checked={selectedButton.mode === mode}
              on:click={() => setSelectedMode(mode)}
            >
              {#if mode === "language"}
                <Languages size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if mode === "animals"}
                <PawPrint size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if mode === "music"}
                <Music size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else if mode === "setup_help"}
                <Wrench size={18} strokeWidth={1.5} aria-hidden="true" />
              {:else}
                <Minus size={18} strokeWidth={1.5} aria-hidden="true" />
              {/if}
              {modeLabel(mode)}
            </button>
          {/each}
        </div>
        {#if selectedButton.mode === "language"}
          <div class="lang-pad">
            <label class="field-label">Language
              <select class="lang-select" value={selectedButton.language} aria-label="Select language for this button" on:change={(event) => setSelectedLanguage((event.currentTarget as HTMLSelectElement).value)}>
                {#each languages as language}
                  <option value={language}>{language}</option>
                {/each}
              </select>
            </label>
          </div>
        {/if}
      </section>

      {#if selectedButton.contentType}
        <section class="section-card">
          <div class="content-tabs" role="tablist">
            <button type="button" class:active-tab={contentListTab === "active"} class="ctab" role="tab" aria-selected={contentListTab === "active"} on:click={() => (contentListTab = "active")}>
              Active <span class="ctab-count cc-active">{selectedContent?.active.length ?? 0}</span>
            </button>
            <button type="button" class:active-tab={contentListTab === "draft"} class="ctab" role="tab" aria-selected={contentListTab === "draft"} on:click={() => (contentListTab = "draft")}>
              Drafts <span class="ctab-count cc-draft">{selectedContent?.inactive.length ?? 0}</span>
            </button>
          </div>
          {#if selectedContent?.error}
            <div class="content-api-error" role="alert">
              <AlertTriangle size={15} strokeWidth={1.5} aria-hidden="true" />
              <span>{selectedContent.error}</span>
            </div>
          {/if}
          {#if contentListTab === "active"}
            {@render ContentRows(selectedContent?.active ?? [], Boolean(selectedContent?.loading), selectedContent?.activeEmptyState ?? null, "No active content for this button.", "Move to trash", trashSelectedContent, undefined)}
          {:else}
            {@render ContentRows(selectedContent?.inactive ?? [], Boolean(selectedContent?.loading), selectedContent?.inactiveEmptyState ?? null, "No drafts for this button.", "Activate", activateSelectedContent, trashSelectedContent)}
          {/if}
        </section>

        <section class="section-card">
          <div class="sc-header">
            <div class="sc-title"><Plus size={16} strokeWidth={1.5} aria-hidden="true" />Add content</div>
            <div class="sc-meta">saves as draft</div>
          </div>
          <div class="add-tabs" role="tablist" aria-label="Add content method">
            <button type="button" class:active-atab={selectedTab === "record"} class="atab" role="tab" aria-selected={selectedTab === "record"} on:click={() => setContentTab("record")}>
              <Mic size={15} strokeWidth={1.5} aria-hidden="true" />Record
            </button>
            <button type="button" class:active-atab={selectedTab === "upload"} class="atab" role="tab" aria-selected={selectedTab === "upload"} on:click={() => setContentTab("upload")}>
              <Upload size={15} strokeWidth={1.5} aria-hidden="true" />Upload
            </button>
            {#if selectedButton.contentType === "language"}
              <button type="button" class:active-atab={selectedTab === "generate"} class="atab" role="tab" aria-selected={selectedTab === "generate"} on:click={() => setContentTab("generate")}>
                <WandSparkles size={15} strokeWidth={1.5} aria-hidden="true" />Generate
              </button>
            {/if}
          </div>

          <div class="add-body">
            {#if selectedButton.contentType === "language"}
              <div class="gen-field">
                <label class="field-label">{selectedTab === "generate" ? "Text to speech" : "Text spoken"}
                  <textarea class="gen-input" rows="3" bind:value={draftForm.text} placeholder="Bonjour tout le monde." disabled={selectedTab === "generate" && generatedSpeechDisabled}></textarea>
                </label>
              </div>
            {:else}
              <div class="gen-field">
                <label class="field-label">Title
                  <input class="gen-input" bind:value={draftForm.title} placeholder={selectedButton.contentType === "music" ? "Song title" : "Animal sound title"} />
                </label>
              </div>
            {/if}

            {#if selectedTab === "record"}
              <div class:recording-active={recordingStatus === "recording"} class:recording-ready={Boolean(recordedWav)} class="record-zone" data-testid="record-zone">
                <button
                  type="button"
                  class:recording={recordingStatus === "recording"}
                  class="record-btn-big"
                  on:click={recorder ? stopRecording : startRecording}
                  aria-label={recorder ? "Stop recording" : "Start recording"}
                  disabled={busy || recordingStatus === "processing" || recordingStatus === "saving"}
                  data-testid="record-toggle"
                >
                  {#if recordingStatus === "recording"}
                    <span class="record-stop-dot" aria-hidden="true"></span>
                  {:else}
                    <Mic size={28} strokeWidth={1.5} aria-hidden="true" />
                  {/if}
                </button>
                <div class="record-step" data-testid="record-status">{recordingHint(recordingStatus, recordSeconds, recordedWav)}</div>
                {#if recordingStatus === "recording"}
                  <div class="record-wave" aria-label="Live microphone level" data-testid="record-waveform">
                    {#each recordWaveform as level}
                      <span style={`height: ${Math.round(8 + level * 28)}px`}></span>
                    {/each}
                  </div>
                {/if}
                <div class="record-hint">{recordingSaveHint(selectedButton, draftForm.text, recordedWav)}</div>
                {#if recordedWav}<audio src={recordedWav.url} controls></audio>{/if}
                <button type="button" class="btn-primary" on:click={submitRecording} disabled={busy || recordingStatus === "saving" || !recordedWav || (selectedButton.contentType === "language" && !draftForm.text.trim())}>
                  <Save size={16} strokeWidth={1.5} aria-hidden="true" />Save recording
                </button>
              </div>
            {:else if selectedTab === "upload"}
              <div
                class:dragging={draggingUpload}
                class="upload-zone"
                role="button"
                tabindex="0"
                on:dragover|preventDefault={() => (draggingUpload = true)}
                on:dragleave={() => (draggingUpload = false)}
                on:drop={dropUpload}
              >
                <div class="upload-icon-big"><Upload size={26} strokeWidth={1.5} aria-hidden="true" /></div>
                <label class="upload-hint">Tap to pick a file<input type="file" accept="audio/mpeg,audio/mp3,audio/wav,.mp3,.wav" on:change={chooseUpload} /></label>
                <div class="upload-formats">{uploadFile ? uploadFile.name : "MP3 or WAV · max 25 MB"}</div>
                {#if uploadPreviewUrl}<audio src={uploadPreviewUrl} controls></audio>{/if}
                <button type="button" class="btn-primary" on:click={submitUpload} disabled={busy || !uploadFile || (selectedButton.contentType === "language" && !draftForm.text.trim())}>
                  <Upload size={16} strokeWidth={1.5} aria-hidden="true" />Save upload
                </button>
              </div>
            {:else if selectedButton.contentType === "language"}
              {#if generatedSpeechOffline}
                <div class="content-api-error" role="alert" data-testid="tts-offline-notice">
                  <AlertTriangle size={15} strokeWidth={1.5} aria-hidden="true" />
                  <span>{generatedSpeechStatus?.message ?? "TTS is offline. Start the local TTS service to generate speech."}</span>
                </div>
              {:else if generatedSpeechStatusLoading}
                <div class="content-api-error" role="status" data-testid="tts-status-loading">
                  <RefreshCw size={15} strokeWidth={1.5} aria-hidden="true" />
                  <span>Checking local TTS availability...</span>
                </div>
              {:else if generatedSpeechStatusError}
                <div class="content-api-error" role="status" data-testid="tts-status-error">
                  <AlertTriangle size={15} strokeWidth={1.5} aria-hidden="true" />
                  <span>Could not check TTS status: {generatedSpeechStatusError}</span>
                </div>
              {/if}
              <div class="gen-row">
                <label class="field-label">Provider
                  <select class="lang-select" bind:value={draftForm.provider} disabled={generatedSpeechDisabled}>
                    {#each providers as provider}
                      <option value={provider}>{provider}</option>
                    {/each}
                  </select>
                </label>
                <label class="field-label">Voice
                  <input class="gen-input" bind:value={draftForm.voice} placeholder="Optional" disabled={generatedSpeechDisabled} />
                </label>
              </div>
              <button type="button" class="btn-primary" on:click={submitGeneration} disabled={busy || !draftForm.text.trim() || generatedSpeechDisabled}>
                <WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />Generate speech
              </button>
            {/if}

            {#if selectedButton.contentType === "language"}
              <button type="button" class="btn-secondary" on:click={clearSelectedGenerated} disabled={busy || (selectedTab === "generate" && generatedSpeechDisabled)}>Clear generated drafts</button>
            {/if}
          </div>
        </section>
      {:else}
        <section class="section-card empty-state">
          <Minus size={24} strokeWidth={1.5} aria-hidden="true" />
          <strong>No content lane</strong>
          <p>Set this button to Language, Animals, or Music before adding active content or drafts.</p>
        </section>
      {/if}
    {/if}
  </div>

  {#if selectedButton}
    <div class="save-bar">
      <div class="save-note">{footerActionLabel() === "Save mode" ? "Changes apply on the next button press" : "Drafts stay inactive until activated"}</div>
      <button type="button" class="save-btn" on:click={runFooterAction} disabled={footerActionDisabled()}>
        <Check size={16} strokeWidth={1.5} aria-hidden="true" />{footerActionLabel()}
      </button>
    </div>
  {/if}
{/snippet}

{#snippet SettingsView()}
  {@render TopBar(true)}
  <div class="body">
    <section class:error={messageType === "error"} class:success={messageType === "success"} class="notice" aria-live="polite">
      {message}
    </section>
    <section class="card">
      <div class="sec-hdr">
        <div class="sec-title"><Cuboid size={16} strokeWidth={1.5} aria-hidden="true" />Cube</div>
      </div>
      <form class="form-stack" on:submit|preventDefault={() => run(() => saveCubeName(cubeName), "Cube name saved.")}>
        <label>Cube name <input bind:value={cubeName} disabled={!isOwner} /></label>
        <button type="submit" class="btn-secondary" disabled={busy || !isOwner}>Save name</button>
      </form>
      <form class="form-stack" on:submit|preventDefault={() => run(() => verifyWifi(wifiForm.ssid, wifiForm.dashboard_ip), "Wi-Fi marked verified.")}>
        <label>Wi-Fi SSID <input bind:value={wifiForm.ssid} placeholder="Home Wi-Fi" disabled={!isOwner} /></label>
        <label>Dashboard IP <input bind:value={wifiForm.dashboard_ip} placeholder="192.168.1.10" disabled={!isOwner} /></label>
        <button type="submit" class="btn-secondary" disabled={busy || !isOwner}>Mark Wi-Fi verified</button>
      </form>
    </section>
    <section class="card">
      <div class="sec-hdr">
        <div class="sec-title"><User size={16} strokeWidth={1.5} aria-hidden="true" />Account</div>
      </div>
      <div class="settings-actions">
        <button type="button" class="btn-secondary" on:click={() => run(async () => (recoveryCode = await createRecoveryCode()), "Recovery code created.")} disabled={busy}>Create recovery code</button>
        <button type="button" class="btn-secondary" on:click={() => run(createManagerInvitation, "Manager invitation created.")} disabled={busy || !isOwner || !setup?.device_id}>Create manager invitation</button>
        <button type="button" class="btn-secondary" on:click={() => run(logout, "Logged out.")} disabled={busy}><LogOut size={16} strokeWidth={1.5} aria-hidden="true" />Log out</button>
      </div>
      {#if recoveryCode}
        <div class="secret"><code>{recoveryCode.code}</code><span>Expires {recoveryCode.expires_at}</span></div>
      {/if}
      {#if invitation}
        <div class="secret"><code>{invitation.code}</code><span>Expires {invitation.expires_at}</span></div>
      {/if}
    </section>
  </div>
{/snippet}

{#snippet InventoryView()}
  {@render TopBar(true)}
  <div class="body">
    <section class:error={messageType === "error"} class:success={messageType === "success"} class="notice" aria-live="polite">
      {message}
    </section>
    <section class="card">
      <div class="sec-hdr">
        <div class="sec-title"><FileAudio size={16} strokeWidth={1.5} aria-hidden="true" />Content inventory</div>
        <button type="button" class="sec-link" on:click={refreshInventory}>
          Refresh <RefreshCw size={13} strokeWidth={1.5} aria-hidden="true" />
        </button>
      </div>
      <div class="inventory-summary">
        <div><strong>{inventory?.active_count ?? 0}</strong><span>Active in setup</span></div>
        <div><strong>{inventory?.draft_count ?? 0}</strong><span>Drafts</span></div>
        <div><strong>{inventory?.unused_count ?? 0}</strong><span>Unused audio</span></div>
      </div>
      {#if inventoryError}
        <div class="content-api-error" role="alert">
          <AlertTriangle size={15} strokeWidth={1.5} aria-hidden="true" />
          <span>{inventoryError}</span>
        </div>
      {/if}
    </section>

    {@render InventoryGroup("Unused audio", "Active files hidden by the current button mode or language.", inventoryItems("unused"))}
    {@render InventoryGroup("Draft audio", "Inactive files waiting for review.", inventoryItems("draft"))}
    {@render InventoryGroup("Active audio", "Files playable in the current button setup.", inventoryItems("active"))}
  </div>
{/snippet}

{#snippet InventoryGroup(title: string, detail: string, items: ContentInventoryItem[])}
  <section class="card inventory-card">
    <div class="sec-hdr">
      <div>
        <div class="sec-title"><FileAudio size={15} strokeWidth={1.5} aria-hidden="true" />{title}</div>
        <div class="inventory-detail">{detail}</div>
      </div>
      <span class="inventory-count">{items.length}</span>
    </div>
    {#if items.length === 0}
      <div class="empty-state">
        <FileAudio size={24} strokeWidth={1.5} aria-hidden="true" />
        <strong>No {title.toLowerCase()}</strong>
        <p>This inventory group is empty.</p>
      </div>
    {:else}
      <div class="content-list" role="list">
        {#each items as item}
          <div class="ci inventory-row" role="listitem">
            <div class="ci-icon {item.status === 'unused' ? 'ci-uploaded' : item.status === 'draft' ? 'ci-generated' : 'ci-recorded'}">
              <FileAudio size={16} strokeWidth={1.5} aria-hidden="true" />
            </div>
            <div class="ci-meta">
              <div class="ci-name">{item.title}</div>
              <div class="ci-detail">{faceNames[item.button_id - 1] ?? `Button ${item.button_id}`} · {item.content_type}{item.language ? ` · ${item.language}` : ""} · {item.source}</div>
              <div class="inventory-reason">{item.reason}</div>
              {#if item.preview_url}<audio src={item.preview_url} controls></audio>{/if}
            </div>
            <div class="ci-actions">
              <button type="button" class="cia" on:click={() => openInventoryButton(item)} aria-label="Open button">
                <ArrowRight size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  </section>
{/snippet}

{#snippet EventFeed(items: RecentButtonEvent[])}
  {#if items.length === 0}
    <div class="empty-state">
      <Activity size={24} strokeWidth={1.5} aria-hidden="true" />
      <strong>No button events yet</strong>
      <p>The feed appears after the child presses a button and the runtime logs the event.</p>
    </div>
  {:else}
    <div class="feed" role="list" aria-label="Recent activity feed">
      {#each items as event}
        <div class="feed-item" role="listitem">
          <div class="feed-icon fi-press"><Hand size={14} strokeWidth={1.5} aria-hidden="true" /></div>
          <div class="feed-body">
            <div class="feed-text"><strong>{faceNames[event.button_id - 1] ?? `Button ${event.button_id}`}</strong> pressed — played <strong>{event.response_text || event.response_id}</strong></div>
            <div class="feed-time">{relativeTime(event.occurred_at)}</div>
          </div>
          <span class="feed-badge fb-teal">Play</span>
        </div>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet ContentRows(items: ContentListItem[], loadingContent: boolean, emptyState: ContentEmptyState | null, empty: string, actionLabel: string, action: ContentAction, secondaryAction: ContentAction | undefined)}
  {#if loadingContent}
    <div class="content-list">
      <div class="skeleton"></div>
      <div class="skeleton short"></div>
    </div>
  {:else if items.length === 0}
    <div class="empty-state">
      <FileAudio size={24} strokeWidth={1.5} aria-hidden="true" />
      <strong>{emptyState?.title ?? empty}</strong>
      <p>{emptyState?.detail ?? (contentListTab === "active" ? "Activate a draft before the child can hear it." : "Record, upload, or generate content to create a draft.")}</p>
    </div>
  {:else}
    <div class="content-list" role="list">
      {#each items as item}
        {#if actionLabel === "Move to trash" && !secondaryAction}
          <div
            class="ci ci-playable"
            role="button"
            tabindex="0"
            aria-label={`Play ${item.title}`}
            on:click={() => void playContentPreview(item)}
            on:keydown={(event) => onContentRowKeydown(event, item)}
          >
            <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : 'ci-recorded'}">
              {#if item.source === "generated"}<WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />{:else if item.source === "uploaded"}<Upload size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<Mic size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
            </div>
            <div class="ci-meta">
              <div class="ci-name" title={item.title}>{trimAudioTitle(item.title)}</div>
              <div class="ci-detail">{contentPlaySummary(item)}</div>
            </div>
            <div class="ci-actions">
              <button type="button" class="cia del" on:click|stopPropagation={() => promptTrashContent(item)} aria-label="Move to trash">
                <Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />
              </button>
            </div>
          </div>
        {:else}
          <div class="ci" role="listitem">
            <div class="ci-icon {item.source === 'generated' ? 'ci-generated' : item.source === 'uploaded' ? 'ci-uploaded' : 'ci-recorded'}">
              {#if item.source === "generated"}<WandSparkles size={16} strokeWidth={1.5} aria-hidden="true" />{:else if item.source === "uploaded"}<Upload size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<Mic size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
            </div>
            <div class="ci-meta">
              <div class="ci-name">{item.title}</div>
              <div class="ci-detail">{item.source} · {item.content_type}</div>
              {#if item.preview_url}<audio src={item.preview_url} controls></audio>{/if}
            </div>
            <div class="ci-actions">
              <button type="button" class="cia" on:click={() => action(item.id)} aria-label={actionLabel}>
                {#if actionLabel === "Activate"}<Check size={16} strokeWidth={1.5} aria-hidden="true" />{:else}<Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />{/if}
              </button>
              {#if secondaryAction}
                <button type="button" class="cia del" on:click={() => secondaryAction(item.id)} aria-label="Move to trash"><Trash2 size={16} strokeWidth={1.5} aria-hidden="true" /></button>
              {/if}
            </div>
          </div>
        {/if}
      {/each}
    </div>
  {/if}
{/snippet}

{#if trashPrompt}
  <div class="trash-backdrop" role="presentation" on:click={cancelTrashContent}>
    <div
      class="trash-dialog"
      role="dialog"
      tabindex="-1"
      aria-modal="true"
      aria-labelledby="trash-dialog-title"
      aria-describedby="trash-dialog-desc"
      on:click|stopPropagation
      on:keydown={(event) => {
        if (event.key === "Escape") cancelTrashContent();
      }}
    >
      <div class="trash-dialog-icon"><AlertTriangle size={22} strokeWidth={1.5} aria-hidden="true" /></div>
      <div class="trash-dialog-body">
        <div id="trash-dialog-title" class="trash-dialog-title">Move audio to trash?</div>
        <div id="trash-dialog-desc" class="trash-dialog-desc">{trashPrompt.title} will be removed from the active list and deleted from disk.</div>
      </div>
      <div class="trash-dialog-actions">
        <button type="button" class="btn-secondary" on:click={cancelTrashContent}>Cancel</button>
        <button type="button" class="btn-primary trash-confirm" on:click={confirmTrashContent}>
          <Trash2 size={16} strokeWidth={1.5} aria-hidden="true" />Move to trash
        </button>
      </div>
    </div>
  </div>
{/if}

{#snippet LayoutGridIcon()}
  <SlidersHorizontal size={15} strokeWidth={1.5} aria-hidden="true" />
{/snippet}
