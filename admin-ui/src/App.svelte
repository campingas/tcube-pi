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
    ChevronRight,
    ChevronUp,
    CircleCheck,
    Copy,
    Cuboid,
    Database,
    FileAudio,
    Folder,
    Hand,
    HardDrive,
    KeyRound,
    Languages,
    Link,
    LogIn,
    LogOut,
    Lock,
    Mic,
    Minus,
    Music,
    PawPrint,
    Play,
    Plus,
    Save,
    Settings,
    ShieldCheck,
    SlidersHorizontal,
    Trash2,
    Upload,
    Usb,
    User,
    Users,
    WandSparkles,
    Wifi,
    Wrench,
    X
  } from "@lucide/svelte";
  import {
    acceptInvitation,
    activateContentItem,
    bootstrapOwner,
    clearUnusedContent,
    clearUnusedGeneratedSpeech,
    completeSetup,
    createInvitation,
    createRecoveryCode,
    factoryReset,
    generateSpeech,
    getContentInventory,
    getGeneratedSpeechStatus,
    getAudioSettings,
    getPomodoroSettings,
    getSession,
    getSetupReview,
    getStatus,
    listActiveContent,
    listInactiveContent,
    listRecentEvents,
    listSoundboxCatalog,
    setSoundboxSelection,
    loginPassword,
    logout,
    recoverPassword,
    saveButtonMode,
    saveAudioSettings,
    saveCubeName,
    saveMultipart,
    savePomodoroSettings,
    trashContentItem,
    verifyWifi
  } from "./api";
  import type {
    ActiveContentItem,
    AudioSettings,
    AuthSession,
    ButtonMode,
    ContentEmptyState,
    GeneratedSpeechStatus,
    ContentInventory,
    ContentInventoryItem,
    ContentType,
    InactiveContentItem,
    PomodoroSettings,
    RecentActivityEvent,
    ServiceStatus,
    SetupReview,
    SoundboxItem
  } from "./api";
  import { blobToWav, canRecordAudio, isSecureRecorderContext } from "./audio";
  import type { RecordedWav } from "./audio";
  import { buttonViewModels, contentKey, updateDraftFormValue } from "./button-config-controller";
  import type { ButtonView } from "./button-config-controller";
  import { contentTypeForMode, defaultMode, modeLabel, splitMode } from "./button-mode";
  import {
    applyAgeRecommendation,
    applyPreset,
    markPomodoroCustom,
    pomodoroPayload,
    settingsToPomodoroForm
  } from "./focus-routine-controller";
  import type { PomodoroForm } from "./focus-routine-controller";
  import {
    generatedSpeechDisabled as generatedSpeechHealthDisabled,
    generatedSpeechMaxBackoffSeconds,
    generatedSpeechMinBackoffSeconds,
    generatedSpeechOfflineStatus,
    generatedSpeechStatusKey as buildGeneratedSpeechStatusKey,
    generatedSpeechVoices,
    isSpeechProviderOfflineMessage,
    menuGeneratedSpeechStatusKey,
    nextGeneratedSpeechBackoff,
    parseGeneratedSpeechStatusKey,
    preferredGeneratedSpeechVoice
  } from "./generated-speech-health";
  import {
    defaultDraftTitle,
    initialRecordWaveform,
    mediaDraftValidationError,
    recordingStatusAfterRevoke,
    recordingStatusAfterSave,
    recordingStatusAfterStop,
    shouldBlockRecordingStart,
    validateUploadFile,
    waveformLevels
  } from "./recording-controller";
  import type { RecordingStatus } from "./recording-controller";
  import type { ButtonConfig, ContentState, DraftForm, InventoryFilter, MessageType } from "./types";
  import AuthView from "./views/AuthView.svelte";
  import DashboardView from "./views/DashboardView.svelte";
  // Secondary views are code-split into their own chunks and loaded on first
  // navigation. The loaders are memoized so re-navigation resolves instantly.
  let buttonConfigViewPromise: Promise<typeof import("./views/ButtonConfigView.svelte").default> | undefined;
  let inventoryViewPromise: Promise<typeof import("./views/InventoryView.svelte").default> | undefined;
  let settingsViewPromise: Promise<typeof import("./views/SettingsView.svelte").default> | undefined;
  const loadButtonConfigView = () =>
    (buttonConfigViewPromise ??= import("./views/ButtonConfigView.svelte").then((m) => m.default));
  const loadInventoryView = () =>
    (inventoryViewPromise ??= import("./views/InventoryView.svelte").then((m) => m.default));
  const loadSettingsView = () =>
    (settingsViewPromise ??= import("./views/SettingsView.svelte").then((m) => m.default));
  type View = "dashboard" | "button-config" | "inventory" | "settings";
  type ContentTab = "record" | "upload" | "generate";
  type ContentListTab = "active" | "draft";
  type ContentListItem = ActiveContentItem | InactiveContentItem;
  type ContentAction = (id: string) => Promise<void>;

  const modes: ButtonMode[] = ["language", "animals", "music", "soundbox", "setup_help", "disabled"];
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
  let status: ServiceStatus | null = null;
  let session: AuthSession | null = null;
  let setup: SetupReview | null = null;
  let pomodoro: PomodoroSettings | null = null;
  let audioSettings: AudioSettings | null = null;
  let audioVolume = 50;
  let audioSaving = false;
  let audioMessage: string | null = null;
  let audioError: string | null = null;
  let events: RecentActivityEvent[] = [];
  let inventory: ContentInventory | null = null;
  let inventoryError: string | null = null;
  let loading = true;
  let busy = false;
  let view: View = "dashboard";
  let inventoryFilter: InventoryFilter = "active";
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
  let soundboxState: Record<number, { items: SoundboxItem[]; loading: boolean; error: string | null }> = {};
  let draftForm: DraftForm = { title: "", text: "", language: "English", provider: "auto", voice: "" };
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
  let recordWaveform = initialRecordWaveform();
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
  let generatedSpeechVoiceOptions: string[] = [];
  let menuLlmStatus: GeneratedSpeechStatus | null = null;
  let menuLlmStatusLoading = false;
  let menuLlmStatusKey = "";
  let lastMenuLlmStatusKey = "";
  let menuLlmCheckTimer: number | null = null;
  let menuLlmBackoffSeconds = generatedSpeechMinBackoffSeconds;
  let settingsCubeNameOpen = true;
  let settingsWifiOpen = false;
  let settingsRecoveryOpen = true;
  let factoryResetPromptOpen = false;
  let factoryResetConfirmation = "";
  let pomodoroForm: PomodoroForm = settingsToPomodoroForm(null);

  $: buttons = buttonViewModels(buildButtonConfigs(setup), contentState);
  $: selectedButton = buttons.find((button) => button.id === selectedButtonId) ?? buttons[0] ?? null;
  $: selectedContent = selectedButton?.contentType ? contentState[contentKey(selectedButton)] : null;
  $: if (session?.authenticated && selectedButton?.mode === "soundbox" && !soundboxState[selectedButton.id]) {
    void refreshSoundbox(selectedButton.id);
  }
  $: currentRole = session?.cubes?.[0]?.role ?? "";
  $: isOwner = currentRole === "owner";
  $: roleLabel = currentRole === "owner" ? "owner" : currentRole === "manager" ? "manager" : currentRole || "member";
  $: roleClass = currentRole === "owner" ? "owner" : currentRole === "manager" ? "admin" : "member";
  $: invitationCodeFromUrl = new URLSearchParams(window.location.search).get("invite") ?? "";
  $: loadedActive = inventory?.active_count ?? Object.values(contentState).reduce((sum, state) => sum + state.active.length, 0);
  $: setupActive = Object.values(setup?.active_counts ?? {}).reduce((sum, value) => sum + value, 0);
  $: generatedSpeechStatusKey = buildGeneratedSpeechStatusKey(selectedButton, selectedTab, draftForm);
  $: menuLlmStatusKey = menuGeneratedSpeechStatusKey(Boolean(session?.authenticated), buttons);
  $: if (generatedSpeechStatusKey && generatedSpeechStatusKey !== lastGeneratedSpeechStatusKey) {
    void checkGeneratedSpeechStatus(generatedSpeechStatusKey, true);
  }
  $: if (!generatedSpeechStatusKey && lastGeneratedSpeechStatusKey) {
    clearGeneratedSpeechStatusTimer();
    lastGeneratedSpeechStatusKey = "";
  }
  $: generatedSpeechOffline = Boolean(generatedSpeechStatusKey && generatedSpeechStatus && !generatedSpeechStatus.online);
  $: generatedSpeechDisabled = generatedSpeechHealthDisabled(generatedSpeechStatusKey, generatedSpeechStatus, generatedSpeechStatusLoading);
  $: generatedSpeechVoiceOptions = generatedSpeechVoices(generatedSpeechStatus);
  $: if (selectedTab === "generate" && generatedSpeechVoiceOptions.length) {
    const nextVoice = preferredGeneratedSpeechVoice(generatedSpeechVoiceOptions, draftForm.voice);
    if (nextVoice !== draftForm.voice) updateDraftForm({ voice: nextVoice });
  }
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
      if (session.authenticated) {
        [pomodoro, audioSettings] = await Promise.all([getPomodoroSettings(), getAudioSettings()]);
        pomodoroForm = settingsToPomodoroForm(pomodoro);
        audioVolume = audioSettings.volume_percent;
        audioMessage = null;
        audioError = null;
      } else {
        pomodoro = null;
        pomodoroForm = settingsToPomodoroForm(null);
        audioSettings = null;
        audioVolume = 50;
        audioMessage = null;
        audioError = null;
      }
      cubeName = setup.cube_name || "T-Cube";
      wifiForm.ssid = setup.wifi_ssid ?? "";
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

  async function refreshSoundbox(buttonId: number) {
    soundboxState = {
      ...soundboxState,
      [buttonId]: { items: soundboxState[buttonId]?.items ?? [], loading: true, error: null }
    };
    try {
      const catalog = await listSoundboxCatalog(buttonId);
      soundboxState = {
        ...soundboxState,
        [buttonId]: { items: catalog.items, loading: false, error: null }
      };
    } catch (error) {
      soundboxState = {
        ...soundboxState,
        [buttonId]: { items: [], loading: false, error: errorText(error) }
      };
    }
  }

  async function toggleSoundboxSound(slug: string, active: boolean) {
    if (!selectedButton) return;
    const buttonId = selectedButton.id;
    busy = true;
    try {
      const catalog = await setSoundboxSelection(buttonId, slug, active);
      soundboxState = {
        ...soundboxState,
        [buttonId]: { items: catalog.items, loading: false, error: null }
      };
      setMessage(active ? "SoundBox sound turned on." : "SoundBox sound turned off.", "success");
    } catch (error) {
      setError(error);
    } finally {
      busy = false;
    }
  }

  async function run(action: () => Promise<unknown>, success: string) {
    busy = true;
    try {
      await action();
      await refreshAll();
      setMessage(success, "success");
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

  async function saveMasterVolume(volumePercent: number) {
    if (!isOwner || audioSaving) return;
    const previousVolume = audioSettings?.volume_percent ?? audioVolume;
    audioVolume = volumePercent;
    audioSaving = true;
    audioMessage = null;
    audioError = null;
    try {
      audioSettings = await saveAudioSettings(volumePercent);
      audioVolume = audioSettings.volume_percent;
      audioMessage = `Volume set to ${audioSettings.volume_percent}%.`;
    } catch (error) {
      audioVolume = previousVolume;
      audioError = errorText(error) || "Could not save device volume.";
    } finally {
      audioSaving = false;
    }
  }

  function buildButtonConfigs(review: SetupReview | null): ButtonConfig[] {
    return [1, 2, 3, 4, 5].map((id) => {
      const raw = review?.button_modes?.[String(id)] ?? defaultMode(id);
      const { mode, language } = splitMode(raw);
      return { id, mode, language, contentType: contentTypeForMode(mode) };
    });
  }

  function activeCount(type: ContentType) {
    return setup?.active_counts?.[type] ?? 0;
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

  function updateDraftForm(patch: Partial<DraftForm>) {
    draftForm = updateDraftFormValue(draftForm, patch);
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
    const { provider, language } = parseGeneratedSpeechStatusKey(key);
    generatedSpeechStatusLoading = true;
    try {
      const nextStatus = await getGeneratedSpeechStatus(provider, language);
      if (key !== generatedSpeechStatusKey) return;
      generatedSpeechStatus = nextStatus;
      generatedSpeechStatusError = null;
      generatedSpeechBackoffSeconds = nextGeneratedSpeechBackoff(nextStatus.online, generatedSpeechBackoffSeconds, immediate);
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
    generatedSpeechStatus = generatedSpeechOfflineStatus(generatedSpeechStatusKey, detail);
    generatedSpeechStatusError = null;
    generatedSpeechBackoffSeconds = generatedSpeechMinBackoffSeconds;
    scheduleGeneratedSpeechStatusCheck(generatedSpeechStatusKey, generatedSpeechBackoffSeconds);
  }

  function isSpeechProviderOfflineError(error: unknown) {
    return isSpeechProviderOfflineMessage(errorText(error));
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
    const { provider, language } = parseGeneratedSpeechStatusKey(key);
    menuLlmStatusLoading = true;
    try {
      const nextStatus = await getGeneratedSpeechStatus(provider, language);
      if (key !== menuLlmStatusKey) return;
      menuLlmStatus = nextStatus;
      menuLlmBackoffSeconds = nextGeneratedSpeechBackoff(nextStatus.online, menuLlmBackoffSeconds, immediate);
      if (!nextStatus.online && key === menuLlmStatusKey) {
        scheduleMenuLlmStatusCheck(key, menuLlmBackoffSeconds);
      }
    } catch {
      if (key !== menuLlmStatusKey) return;
      menuLlmStatus = {
        online: false,
        provider,
        checked_at: new Date().toISOString(),
        cached: false,
        cache_ttl_seconds: 20,
        next_check_after_seconds: menuLlmBackoffSeconds,
        message: "TTS provider is offline or unreachable.",
        voices: []
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

  function openInventoryButton(item: ContentInventoryItem) {
    selectedButtonId = item.button_id;
    contentListTab = item.status === "draft" ? "draft" : "active";
    view = "button-config";
  }

  function openStatDetail(filter: InventoryFilter) {
    inventoryFilter = filter;
    view = "inventory";
  }

  function selectSetupAction(id: string) {
    if (id === "language") selectedButtonId = buttons.find((button) => button.contentType === "language")?.id ?? 1;
    if (id === "animals") selectedButtonId = buttons.find((button) => button.contentType === "animals")?.id ?? 2;
    if (id === "music") selectedButtonId = buttons.find((button) => button.contentType === "music")?.id ?? 3;
    if (id === "language" || id === "animals" || id === "music") view = "button-config";
    if (id === "wifi") {
      settingsWifiOpen = true;
      view = "settings";
    }
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
      if (button.mode === "soundbox") {
        await refreshSoundbox(button.id);
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

  async function clearAllUnusedContent() {
    if (!window.confirm("Clear all unused audio files from this cube? Active content in the current setup and drafts will stay available.")) return;
    await run(async () => {
      const result = await clearUnusedContent();
      setMessage(`${result.deleted_count} unused audio item${result.deleted_count === 1 ? "" : "s"} cleared.`, "success");
    }, "Unused content cleared.");
  }

  function openFactoryResetPrompt() {
    factoryResetConfirmation = "";
    factoryResetPromptOpen = true;
  }

  function cancelFactoryReset() {
    factoryResetPromptOpen = false;
    factoryResetConfirmation = "";
  }

  async function confirmFactoryReset() {
    if (factoryResetConfirmation !== "FACTORY RESET") return;
    busy = true;
    try {
      await factoryReset(factoryResetConfirmation);
      session = { authenticated: false, bootstrap_required: true, account: null, cubes: [] };
      setup = null;
      events = [];
      inventory = null;
      pomodoro = null;
      pomodoroForm = settingsToPomodoroForm(null);
      contentState = {};
      soundboxState = {};
      recoveryCode = null;
      invitation = null;
      factoryResetPromptOpen = false;
      factoryResetConfirmation = "";
      view = "dashboard";
      setMessage("Factory reset complete. Create a new owner account to set up this cube.", "success");
    } catch (error) {
      setError(error);
    } finally {
      busy = false;
    }
  }

  async function copyText(value: string, label: string) {
    try {
      await navigator.clipboard.writeText(value);
      setMessage(`${label} copied.`, "success");
    } catch {
      setError("Copy failed. Select and copy the code manually.");
    }
  }

  function invitationUrl(code: string) {
    return `${window.location.origin}/?invite=${encodeURIComponent(code)}`;
  }

  async function startRecording() {
    if (shouldBlockRecordingStart(recordingStatus)) return;
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
    recordingStatus = recordingStatusAfterStop(recorder?.state) ?? recordingStatus;
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
    recordWaveform = waveformLevels(data, recordWaveform.length);
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
    recordWaveform = initialRecordWaveform();
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
    recordingStatus = recordingStatusAfterRevoke(Boolean(recorder)) ?? recordingStatus;
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
    const validation = validateUploadFile(file);
    if (!validation.ok) {
      setError(validation.error);
      return;
    }
    uploadFile = file;
    uploadPreviewUrl = URL.createObjectURL(file);
    if (selectedButton?.contentType && selectedButton.contentType !== "language" && !draftForm.title.trim()) {
      draftForm = updateDraftFormValue(draftForm, { title: defaultDraftTitle(file.name) });
    }
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
    recordingStatus = recordingStatusAfterSave(Boolean(recordedWav));
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
      const generatedDraft = await generateSpeech({
        button_id: selectedButton.id,
        language: selectedButton.language,
        text: draftForm.text,
        provider: draftForm.provider,
        voice: draftForm.voice.trim() || undefined
      });
      contentListTab = "draft";
      setMessage("Generated speech saved as draft.", "success");
      await playContentPreview(generatedDraft);
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
    const error = mediaDraftValidationError(selectedButton, draftForm);
    if (error) {
      setError(error);
      return false;
    }
    return true;
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
    <AuthView
      state={{ session, invitationCodeFromUrl, message, messageType, bootstrapForm, loginForm, recoveryForm, inviteForm, busy }}
      actions={{
        submitInvitation: async () => run(async () => (session = await acceptInvitation(inviteForm)), "Manager account created."),
        submitBootstrap: async () => run(async () => (session = await bootstrapOwner(bootstrapForm)), "Owner account created."),
        submitLogin: async () => run(async () => (session = await loginPassword(loginForm)), "Logged in."),
        submitRecovery: async () => run(() => recoverPassword(recoveryForm), "Password updated. Previous sessions were revoked.")
      }}
    />
  {:else if view === "button-config"}
    {#await loadButtonConfigView() then ButtonConfigView}
    <ButtonConfigView
      state={{
        session,
        setup,
        message,
        messageType,
        buttons,
        selectedButtonId,
        selectedButton,
        selectedContent,
        selectedTab,
        contentListTab,
        draftForm,
        recorder,
        recordingStatus,
        recordSeconds,
        recordWaveform,
        recordedWav,
        uploadFile,
        uploadPreviewUrl,
        draggingUpload,
        contentDurations,
        events,
        generatedSpeechDisabled,
        generatedSpeechStatusLoading,
        generatedSpeechStatusError,
        generatedSpeechVoiceOptions,
        trashPrompt,
        soundbox: selectedButton ? soundboxState[selectedButton.id] ?? null : null,
        busy
      }}
      actions={{
        goHome,
        openSettings: () => (view = "settings"),
        setSelectedButtonId: (id: number) => (selectedButtonId = id),
        setContentListTab: (tab: "active" | "draft") => (contentListTab = tab),
        setContentTab,
        setSelectedMode,
        setSelectedLanguage,
        updateDraftForm,
        saveSelectedButtonMode: () => selectedButton && saveSelectedButtonMode(selectedButton),
        activateSelectedContent,
        trashSelectedContent,
        startRecording,
        stopRecording,
        revokeRecording,
        submitRecording,
        chooseUpload,
        clearUpload: () => setUploadFile(null),
        dropUpload,
        setUploadDragging: (dragging: boolean) => (draggingUpload = dragging),
        submitUpload,
        submitGeneration,
        playContentPreview,
        toggleSoundboxSound,
        promptTrashContent,
        cancelTrashContent,
        confirmTrashContent
      }}
    />
    {/await}
  {:else if view === "inventory"}
    {#await loadInventoryView() then InventoryView}
    <InventoryView
      state={{ session, message, messageType, inventory, inventoryError, events, filter: inventoryFilter }}
      actions={{
        goHome,
        openSettings: () => (view = "settings"),
        openInventoryButton
      }}
    />
    {/await}
  {:else if view === "settings"}
    {#await loadSettingsView() then SettingsView}
    <SettingsView
      state={{
        session,
        status,
        setup,
        pomodoro,
        pomodoroForm,
        audioSettings,
        audioVolume,
        audioSaving,
        audioMessage,
        audioError,
        message,
        messageType,
        roleLabel,
        isOwner,
        busy,
        cubeName,
        wifiForm,
        settingsCubeNameOpen,
        settingsWifiOpen,
        settingsRecoveryOpen,
        recoveryCode,
        invitation,
        totalUnused,
        factoryResetPromptOpen,
        factoryResetConfirmation
      }}
      actions={{
        goHome,
        openSettings: () => (view = "settings"),
        setSettingsCubeNameOpen: (open: boolean) => (settingsCubeNameOpen = open),
        setSettingsWifiOpen: (open: boolean) => (settingsWifiOpen = open),
        setSettingsRecoveryOpen: (open: boolean) => (settingsRecoveryOpen = open),
        saveCubeName: async (value: string) => run(() => saveCubeName(value), "Cube name saved."),
        verifyWifi: async (ssid: string, dashboardIp: string) => run(() => verifyWifi(ssid, dashboardIp), "Wi-Fi marked verified."),
        setPomodoroForm: (form: PomodoroForm) => (pomodoroForm = form),
        applyPomodoroAge: (age: string) => (pomodoroForm = applyAgeRecommendation(pomodoroForm, age)),
        applyPomodoroPreset: (preset: "mini" | "focus" | "full" | "custom") => (pomodoroForm = applyPreset(pomodoroForm, preset)),
        updatePomodoroCustom: (patch: Partial<Omit<PomodoroForm, "preset">>) => (pomodoroForm = markPomodoroCustom(pomodoroForm, patch)),
        savePomodoro: async () =>
          run(async () => {
            pomodoro = await savePomodoroSettings(pomodoroPayload(pomodoroForm));
            pomodoroForm = settingsToPomodoroForm(pomodoro);
          }, "Focus routine saved."),
        setAudioVolume: (volumePercent: number) => {
          audioVolume = volumePercent;
          audioMessage = null;
          audioError = null;
        },
        saveAudioVolume: saveMasterVolume,
        createRecoveryCode: async () => run(async () => (recoveryCode = await createRecoveryCode()), "Recovery code created."),
        copyText,
        createManagerInvitation: async () => run(createManagerInvitation, "Manager invitation created."),
        clearAllUnusedContent: async () => run(clearAllUnusedContent, "Unused content cleared."),
        openFactoryResetPrompt,
        logout: () => run(logout, "Logged out."),
        dismissInvitation: () => (invitation = null),
        dismissRecoveryCode: () => (recoveryCode = null),
        setFactoryResetConfirmation: (value: string) => (factoryResetConfirmation = value),
        cancelFactoryReset,
        confirmFactoryReset
      }}
    />
    {/await}
  {:else}
    <DashboardView
      state={{
        status,
        setup,
        session,
        message,
        messageType,
        buttons,
        events,
        prerequisites,
        setupReady,
        blockedSetupText,
        totalActive,
        totalDrafts,
        totalUnused,
        menuLlmOnline,
        menuLlmStatusLoading,
        menuLlmLabel
      }}
      actions={{
        goHome,
        openStatDetail,
        openSettings: () => (view = "settings"),
        openButtonConfig,
        selectSetupAction,
        completeSetup: async () => {
          if (!window.confirm("Completing setup switches the cube to child mode. You can still manage content from this dashboard.")) return;
          await run(completeSetup, "Setup complete. The cube is ready for child mode.");
        }
      }}
    />
  {/if}
</main>
