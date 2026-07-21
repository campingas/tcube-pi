import { readFile } from "node:fs/promises";
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await mockAdminApi(page);
});

test("mobile dashboard stacks primary cards and keeps stats within the viewport", async ({ page }) => {
  await page.goto("/");

  const hero = page.getByTestId("dashboard-hero-card");
  const buttons = page.getByTestId("dashboard-buttons-card");
  await expect(hero).toBeVisible();
  await expect(page.getByTestId("hero-wifi-line")).toHaveText(/Home · 192\.168\.50\.159/);
  await expect(page.getByTestId("hero-usb-line")).toHaveText(/USB · 10\.55\.0\.1/);
  await expect(page.getByTestId("hero-wifi-icon")).toHaveClass(/hero-icon-ok/);
  await expect(page.getByTestId("hero-usb-icon")).toHaveClass(/hero-icon-ok/);
  await expect(hero.locator(".cube-online-dot")).toHaveCount(0);
  await expect(hero.locator(".cube-badge")).toHaveCount(0);
  await expect(page.getByTestId("dashboard-inventory-card")).toHaveCount(0);
  await expect(buttons).toBeVisible();
  await expect(page.getByText("Signed in as campingas")).toBeVisible();
  await expect(page.getByText("Top button pressed — played apple_en_recorded.wav")).toBeVisible();
  await expect(page.getByText("bird_en_voxtral.wav activated on Top button")).toBeVisible();

  const boxes = await Promise.all([hero.boundingBox(), buttons.boundingBox()]);
  expect(boxes.every(Boolean)).toBe(true);
  const [heroBox, buttonsBox] = boxes as NonNullable<(typeof boxes)[number]>[];

  expect(buttonsBox.y).toBeGreaterThan(heroBox.y + heroBox.height - 1);
  expect(Math.abs(heroBox.x - buttonsBox.x)).toBeLessThanOrEqual(1);
  expect(heroBox.width).toBeLessThanOrEqual(390);

  const statBoxes = await page.getByTestId("dashboard-stats").locator(".cstat").evaluateAll((items) =>
    items.map((item) => {
      const rect = item.getBoundingClientRect();
      return { left: rect.left, right: rect.right, width: rect.width };
    })
  );
  expect(statBoxes).toHaveLength(4);
  for (const box of statBoxes) {
    expect(box.left).toBeGreaterThanOrEqual(0);
    expect(box.right).toBeLessThanOrEqual(390);
    expect(box.width).toBeGreaterThan(70);
  }

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
});

test("dashboard stat cards open focused detail views", async ({ page }) => {
  await page.goto("/");

  await page.getByTestId("dashboard-stat-active").click();
  await expect(page.getByTestId("stat-detail-view")).toBeVisible();
  await expect(page.getByText("Active sounds")).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh" })).toHaveCount(0);
  await expect(page.getByText("Morning greeting")).toBeVisible();
  await expect(page.getByText("Top · Language · English · Generated")).toBeVisible();

  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await page.getByTestId("dashboard-stat-draft").click();
  await expect(page.getByText("Drafts")).toBeVisible();
  await expect(page.getByText("Ready to review")).toBeVisible();
  await expect(page.getByText("Inactive files waiting for review.")).toBeVisible();

  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await page.getByTestId("dashboard-stat-unused").click();
  await expect(page.getByText("Unused audio")).toBeVisible();
  await expect(page.getByText("Old animal sound")).toBeVisible();
  await expect(page.getByText("Hidden by current setup")).toBeVisible();

  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await page.getByTestId("dashboard-stat-presses").click();
  await expect(page.getByText("Presses today")).toBeVisible();
  await expect(page.getByText("Hello")).toBeVisible();
  await expect(page.getByText(/Top · language ·/)).toBeVisible();
});

test("top bar shows manager role as manager while keeping manager styling", async ({ page }) => {
  await page.route("**/api/pi/v1/auth/session", async (route) => {
    await route.fulfill({
      json: {
        authenticated: true,
        bootstrap_required: false,
        account: { id: "acct-manager", username: "bob43", display_name: "bob43" },
        cubes: [{ device_id: "cube-1", label: "T-Cube", role: "manager" }]
      }
    });
  });
  await page.goto("/");

  await expect(page.locator(".topbar-session-prefix")).toHaveText("Signed in as");
  await expect(page.locator(".topbar-session-user")).toHaveText("bob43");
  await expect(page.locator(".topbar-session-role")).toHaveText("manager");
  await expect(page.locator(".topbar-session-role")).toHaveClass(/role-admin/);
});

test("create owner screen does not show a refresh action", async ({ page }) => {
  await page.route("**/api/pi/v1/auth/session", async (route) => {
    await route.fulfill({
      json: {
        authenticated: false,
        bootstrap_required: true,
        account: null,
        cubes: []
      }
    });
  });
  await page.goto("/");

  await expect(page.getByText("Create local owner")).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh" })).toHaveCount(0);
  await expect(page.getByRole("button", { name: /go to dashboard/i })).toHaveCount(0);
});

test("mobile button selector keeps five fixed-size button pills horizontally usable", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("dashboard-button-1").click();

  const selector = page.getByTestId("button-selector");
  await expect(selector).toBeVisible();
  await expect(page.getByText("Select a button")).toBeVisible();

  const selectorSize = await selector.evaluate((node) => ({
    clientWidth: node.clientWidth,
    scrollWidth: node.scrollWidth
  }));
  expect(selectorSize.scrollWidth).toBeGreaterThan(selectorSize.clientWidth);

  for (let id = 1; id <= 5; id += 1) {
    const pill = page.getByTestId(`button-selector-${id}`);
    await expect(pill).toBeVisible();
    const box = await pill.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeGreaterThanOrEqual(66);
    expect(box!.width).toBeLessThanOrEqual(70);
    expect(box!.height).toBeGreaterThanOrEqual(82);
    expect(box!.height).toBeLessThanOrEqual(110);
  }
});

test("button config mode changes update selector and hero icons on mobile", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("dashboard-button-1").click();

  await expect(page.getByTestId("selected-button-hero-icon")).toHaveClass(/fpi-lang/);
  await expect(page.getByTestId("button-selector-1-icon")).toHaveClass(/fpi-lang/);
  await expect(page.locator(".save-bar")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Save mode" })).toBeVisible();

  await page.getByTestId("button-mode-music").click();
  await expect(page.getByTestId("button-mode-music")).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("selected-button-hero-icon")).toHaveClass(/fpi-music/);
  await expect(page.getByTestId("button-selector-1-icon")).toHaveClass(/fpi-music/);
  await expect(page.getByText("Top · Music")).toBeVisible();

  await page.getByTestId("button-mode-setup_help").click();
  await expect(page.getByTestId("button-mode-setup_help")).toHaveAttribute("aria-checked", "true");
  await expect(page.getByTestId("selected-button-hero-icon")).toHaveClass(/fpi-setup/);
  await expect(page.getByTestId("button-selector-1-icon")).toHaveClass(/fpi-setup/);
  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await expect(page.getByTestId("dashboard-hero-card")).toBeVisible();
});

test("button config active rows trim long titles, show summaries, and open the trash modal", async ({ page }) => {
  await page.goto("/");
  await page.getByTestId("dashboard-button-1").click();

  await expect(page.getByText("Published")).toHaveCount(0);
  await expect(page.getByText("Active content")).toHaveCount(0);
  const activeRow = page.getByRole("button", { name: /Play This file name is intentionally long/ });
  await expect(activeRow).toBeVisible();
  await expect(activeRow.getByRole("button", { name: "Move to trash" })).toBeVisible();
  await expect(activeRow.locator("audio")).toHaveCount(0);

  await expect(page.getByTitle("This file name is intentionally long enough to trim nicely.wav")).toBeVisible();
  await expect(page.getByText("This file name is intentionally…")).toBeVisible();
  await expect(page.getByText(/Generated · 0:0\d · 3 plays/)).toBeVisible();
  await expect(page.getByText(/Generated · 0:01 · 3 plays/)).toBeVisible({ timeout: 5000 });

  await activeRow.click();

  await activeRow.getByRole("button", { name: "Move to trash" }).click();
  const dialog = page.getByRole("dialog", { name: /Move audio to trash\?/ });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(/will be removed from the active list and deleted from disk/)).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Move to trash" })).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);

  await page.getByRole("tab", { name: /Drafts/i }).click();
  await expect(page.getByRole("button", { name: "Clear generated" })).toHaveCount(0);
  const draftRow = page.getByRole("button", { name: /Play draft Bonjour/ });
  await expect(draftRow).toBeVisible();
  await expect(draftRow).toContainText("tap to preview");
  await expect(draftRow.getByRole("button", { name: "Activate draft" })).toBeVisible();
});

test("recording flow shows live microphone feedback and draft guidance", async ({ page }) => {
  await installMockRecordingApis(page);
  await page.goto("/");
  expect(await page.evaluate(() => ({
    audioContext: String(AudioContext).includes("MockAudioContext"),
    mediaRecorder: String(MediaRecorder).includes("MockMediaRecorder"),
    getUserMedia: typeof navigator.mediaDevices?.getUserMedia,
    hostname: window.location.hostname,
    secure: window.isSecureContext
  }))).toEqual({
    audioContext: true,
    mediaRecorder: true,
    getUserMedia: "function",
    hostname: "127.0.0.1",
    secure: true
  });
  await page.getByTestId("dashboard-button-1").click();

  await expect(page.getByRole("tab", { name: /Record/i })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByRole("tab", { name: /Record/i })).toHaveClass(/active-atab/);
  await expect(page.getByTestId("record-zone")).toBeVisible();
  await expect(page.getByTestId("record-zone").getByPlaceholder("Write the text spoken here")).toBeVisible();
  await expect(page.getByTestId("record-zone").getByText("Text spoken")).toHaveCount(0);
  await expect(page.locator(".record-btn-big")).toBeVisible();
  await expect(page.getByTestId("record-status")).toHaveText("Tap record, then speak clearly near your phone.");
  await expect(page.getByText("After recording, preview the audio here before saving.")).toBeVisible();
  await expect(page.locator(".save-bar")).toHaveCount(0);

  await page.getByTestId("record-toggle").click();
  expect(await page.evaluate(() => (window as Window & { __getUserMediaCalls?: number }).__getUserMediaCalls ?? 0)).toBe(1);
  await expect(page.getByTestId("record-status")).toContainText("Recording");
  await expect(page.getByTestId("record-waveform")).toBeVisible();
  await expect(page.getByRole("button", { name: "Stop recording" })).toBeVisible();

  await page.getByTestId("record-toggle").click();
  await expect(page.getByTestId("record-status")).toContainText("Preview");
  await expect(page.locator(".record-zone audio")).toBeVisible();
  await expect(page.getByText("Enter the text spoken before saving this recording.")).toBeVisible();
  await expect(page.getByTestId("record-zone").getByRole("button", { name: "Save recording" })).toBeDisabled();

  await page.getByLabel("Text spoken").fill("Bonjour tout le monde.");
  await expect(page.getByTestId("record-zone").getByRole("button", { name: "Save recording" })).toBeEnabled();
  await page.getByTestId("record-zone").getByRole("button", { name: "Save recording" }).click();
  await expect(page.getByRole("tab", { name: /Drafts/i })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText("Inactive drafts")).toHaveCount(0);
  await expect(page.getByText("Review queue")).toHaveCount(0);
  await expect(page.getByText("test · language · inactive")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Activate draft" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Move draft to trash" })).toBeVisible();

  await page.getByRole("tab", { name: /Upload/i }).click();
  await expect(page.getByRole("tab", { name: /Upload/i })).toHaveClass(/active-atab/);
  await expect(page.getByTestId("upload-zone")).toBeVisible();
  await expect(page.getByLabel("Upload steps")).toBeVisible();
  await expect(page.getByText("Choose file")).toBeVisible();
  await expect(page.getByText("Choose an MP3 or WAV under 25 MB.")).toBeVisible();
  await expect(page.getByTestId("upload-zone").getByLabel("Text spoken")).toHaveCount(0);
  await expect(page.getByTestId("upload-zone").getByRole("button", { name: "Save Draft" })).toBeDisabled();
  await page.getByTestId("upload-zone").locator("input[type='file']").setInputFiles({
    name: "bonjour.wav",
    mimeType: "audio/wav",
    buffer: Buffer.from("mock upload")
  });
  await expect(page.getByText("bonjour.wav")).toBeVisible();
  await expect(page.getByText("11 B · MP3 or WAV")).toBeVisible();
  await expect(page.getByTestId("upload-zone").locator("audio")).toBeVisible();
  await expect(page.getByTestId("upload-zone").getByLabel("Text spoken")).toBeVisible();
  await expect(page.getByText("Drafts are not heard by the child until you activate them.")).toBeVisible();
  await expect(page.getByTestId("upload-zone").getByRole("button", { name: "Choose another file" })).toBeVisible();
  await expect(page.getByTestId("upload-zone").getByRole("button", { name: "Save Draft" })).toBeEnabled();
  await page.getByTestId("upload-zone").getByRole("button", { name: "Save Draft" }).click();
  await expect(page.getByRole("tab", { name: /Drafts/i })).toHaveAttribute("aria-selected", "true");
});

test("generated speech disables only generate controls while TTS is offline", async ({ page }) => {
  await page.clock.install();
  await page.goto("/");
  await expect(page.getByText("LLMs offline")).toBeVisible();
  await page.getByTestId("dashboard-button-1").click();

  await page.getByRole("tab", { name: /Generate/i }).click();
  await expect(page.getByTestId("tts-offline-notice")).toContainText("TTS provider is offline");
  await expect(page.getByLabel("Text to speech")).toBeDisabled();
  await expect(page.getByLabel("Provider")).toBeEnabled();
  await expect(page.getByRole("button", { name: "Generate speech" })).toBeDisabled();
  await page.getByLabel("Provider").selectOption("voxtral");

  await page.getByRole("tab", { name: /Record/i }).click();
  await expect(page.getByTestId("record-toggle")).toBeEnabled();
  await expect(page.getByLabel("Text spoken")).toBeEnabled();

  await page.getByRole("tab", { name: /Generate/i }).click();
  await page.clock.fastForward(31_000);
  await expect(page.getByTestId("tts-offline-notice")).toHaveCount(0);
  await page.clock.fastForward(61_000);
  await expect(page.getByLabel("Text to speech")).toBeEnabled();
  await expect(page.getByLabel("Voice")).toBeEnabled();
  await expect(page.getByLabel("Voice")).toHaveValue("neutral_male");
  await expect(page.getByLabel("Voice")).toContainText("cheerful_female");
  await page.getByLabel("Text to speech").fill("Bonjour tout le monde.");
  await expect(page.getByRole("button", { name: "Generate speech" })).toBeEnabled();
  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await expect(page.getByText("LLMs online")).toBeVisible();
});

test("setup checklist sits below the notice and Wi-Fi action opens settings verification", async ({ page }) => {
  await page.route("**/api/pi/v1/status", async (route) => {
    await route.fulfill({
      json: {
        status: "ok",
        service: "tcube-pi-admin",
        mode: "admin",
        database_present: true,
        ui_dist_present: true,
        media_root: "data/audio",
        content_root: "content",
        hostname: "tcube.local",
        usb_address: "",
        usb_connected: false,
        contract_note: "test"
      }
    });
  });
  await page.route("**/api/pi/v1/setup/review", async (route) => {
    await route.fulfill({
      json: {
        cube_name: "T-Cube",
        device_id: "cube-1",
        admin_created: true,
        wifi_verified: false,
        wifi_ssid: null,
        dashboard_ip: "",
        dashboard_address: "https://tcube.local/",
        button_modes: {
          "1": "language:English",
          "2": "animals",
          "3": "music",
          "4": "setup_help",
          "5": "disabled"
        },
        active_counts: {
          language: 2,
          animals: 1,
          music: 1
        }
      }
    });
  });
  await page.goto("/");

  const notice = page.locator(".notice").first();
  const setupChecklist = page.getByLabel("Setup checklist");
  const hero = page.getByTestId("dashboard-hero-card");
  await expect(setupChecklist).toBeVisible();
  await expect(page.getByTestId("hero-wifi-line")).toHaveText(/wi-fi · 192\.168\.0\.1/);
  await expect(page.getByTestId("hero-usb-line")).toHaveText(/USB · Not connected/);
  await expect(page.getByTestId("hero-wifi-icon")).toHaveClass(/hero-icon-warn/);
  await expect(page.getByTestId("hero-usb-icon")).toHaveClass(/hero-icon-muted/);
  const [noticeBox, setupBox, heroBox] = await Promise.all([
    notice.boundingBox(),
    setupChecklist.boundingBox(),
    hero.boundingBox()
  ]);
  expect(noticeBox).not.toBeNull();
  expect(setupBox).not.toBeNull();
  expect(heroBox).not.toBeNull();
  expect(setupBox!.y).toBeGreaterThan(noticeBox!.y + noticeBox!.height - 1);
  expect(heroBox!.y).toBeGreaterThan(setupBox!.y + setupBox!.height - 1);

  await page.getByRole("button", { name: /Wi-Fi verified/i }).click();
  await expect(page.getByRole("navigation").getByText("Settings")).toBeVisible();
  await expect(page.locator(".settings-row-title").filter({ hasText: "Focus routine" })).toHaveText("Focus routine");
  await expect(page.getByText("Hold any two buttons together for 3 seconds. This setting is stored only on this cube.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Mark verified" })).toBeVisible();
});

test("settings page matches grouped setup controls and calls settings APIs", async ({ page }) => {
  await page.addInitScript(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (value: string) => {
          (window as Window & { __copiedText?: string }).__copiedText = value;
        }
      }
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();

  await expect(page.getByRole("navigation").getByText("Settings")).toBeVisible();
  await expect(page.locator(".settings-group-label")).toHaveText([
    "Cube",
    "Sound · Owner only",
    "Focus routine · Owner only",
    "Account",
    "Manager invitations · Owner only",
    "Danger zone"
  ]);

  const volumeSlider = page.getByTestId("audio-volume-slider");
  const volumeRequests: string[] = [];
  page.on("request", (request) => {
    if (request.method() === "PUT" && new URL(request.url()).pathname === "/api/pi/v1/setup/audio") {
      volumeRequests.push(request.postData() ?? "");
    }
  });
  await expect(volumeSlider).toHaveValue("50");
  await volumeSlider.fill("0");
  await expect(page.getByTestId("audio-volume-label")).toHaveText("Muted");
  await expect(page.getByText("Volume set to 0%.")).toBeVisible();
  expect(volumeRequests).toHaveLength(1);

  await page.getByLabel("Child age").fill("10");
  await expect(page.getByLabel("Focus minutes")).toHaveValue("20");
  await expect(page.getByLabel("Break minutes")).toHaveValue("5");
  await expect(page.getByLabel("Cycles")).toHaveValue("3");
  await expect(page.getByRole("radio", { name: "Focus" })).toHaveAttribute("aria-checked", "true");
  await page.getByLabel("Enable after save").check();
  await page.getByLabel("Focus minutes").fill("21");
  await expect(page.getByRole("radio", { name: "Custom" })).toHaveAttribute("aria-checked", "true");
  await page.getByRole("button", { name: "Save focus routine" }).click();
  await expect(page.getByText("Focus routine saved.")).toBeVisible();
  await expect(page.getByText("Enabled")).toBeVisible();

  await expect(page.getByLabel("Display name")).toHaveValue("T-Cube");
  await page.getByLabel("Display name").fill("Mia's Cube");
  await page.getByRole("button", { name: "Save name" }).click();
  await expect(page.getByText("Cube name saved.")).toBeVisible();

  await page.getByRole("button", { name: "Create recovery code" }).click();
  await expect(page.getByText("RCV-123-456")).toBeVisible();
  await expect(page.getByRole("button", { name: "Copy recovery code" })).toBeVisible();

  await page.getByRole("button", { name: "Create new invitation link" }).click();
  await expect(page.getByText("INV-789-ABC")).toBeVisible();
  await page.getByRole("button", { name: "Copy invite link" }).click();
  await expect(page.getByText("Invitation link copied.")).toBeVisible();
  const expectedInviteUrl = await page.evaluate(() => `${window.location.origin}/?invite=INV-789-ABC`);
  await expect.poll(() => page.evaluate(() => (window as Window & { __copiedText?: string }).__copiedText)).toBe(expectedInviteUrl);

  await page.getByRole("button", { name: "Clear unused content" }).click();
  await expect(page.getByText("Unused content cleared.")).toBeVisible();

  await page.getByRole("button", { name: "Factory reset" }).click();
  const resetDialog = page.getByRole("dialog", { name: "Factory reset this cube?" });
  await expect(resetDialog).toBeVisible();
  await expect(resetDialog.getByRole("button", { name: "Factory reset" })).toBeDisabled();
  await resetDialog.getByLabel("Factory reset confirmation").fill("FACTORY RESET");
  await expect(resetDialog.getByRole("button", { name: "Factory reset" })).toBeEnabled();
  await resetDialog.getByRole("button", { name: "Factory reset" }).click();
  await expect(page.getByText("Factory reset complete. Create a new owner account to set up this cube.")).toBeVisible();
  await expect(page.getByText("Create local owner")).toBeVisible();

  const overflow = await page.evaluate(() => document.documentElement.scrollWidth - document.documentElement.clientWidth);
  expect(overflow).toBeLessThanOrEqual(1);
});

test("manager can view sound and focus settings but cannot edit them", async ({ page }) => {
  await page.route("**/api/pi/v1/auth/session", async (route) => {
    await route.fulfill({
      json: {
        authenticated: true,
        bootstrap_required: false,
        account: { id: "acct-manager", username: "manager", display_name: "Manager" },
        cubes: [{ device_id: "cube-1", label: "T-Cube", role: "manager" }]
      }
    });
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();

  await expect(page.getByTestId("audio-volume-label")).toHaveText("50%");
  await expect(page.getByTestId("audio-volume-slider")).toBeDisabled();
  await expect(page.getByText("Only an owner can change device volume.")).toBeVisible();
  await expect(page.getByText("Focus routine · Owner only")).toBeVisible();
  await expect(page.getByLabel("Child age")).toBeDisabled();
  await expect(page.getByLabel("Enable after save")).toBeDisabled();
  await expect(page.getByRole("button", { name: "Save focus routine" })).toBeDisabled();
});

test("failed volume save rolls the slider back and shows its dedicated error", async ({ page }) => {
  await page.route("**/api/pi/v1/setup/audio", async (route) => {
    if (route.request().method() === "PUT") {
      await route.fulfill({ status: 400, json: { detail: "volume save failed" } });
      return;
    }
    await route.fallback();
  });
  await page.goto("/");
  await page.getByRole("button", { name: "Settings" }).click();

  const volumeSlider = page.getByTestId("audio-volume-slider");
  await volumeSlider.fill("75");
  await expect(volumeSlider).toHaveValue("50");
  await expect(page.getByText("volume save failed")).toBeVisible();
  await expect(page.getByTestId("audio-volume-label")).toHaveText("50%");
});

async function mockAdminApi(page: Page) {
  let generatedSpeechStatusCalls = 0;
  let audioSettings = {
    volume_percent: 50,
    updated_at: "2026-07-01T00:00:00.000Z"
  };
  let pomodoroSettings = {
    enabled: false,
    child_age_years: null,
    focus_minutes: 10,
    break_minutes: 3,
    cycles: 2,
    preset: "mini",
    validated_at: null,
    updated_at: "2026-07-01T00:00:00.000Z",
    recommendation: {
      preset: "mini",
      focus_minutes: 10,
      break_minutes: 3,
      cycles: 2,
      reason: "Starter plan until an owner saves the child age."
    },
    trigger: {
      mode: "any",
      required_button_count: 2,
      assembly_window_ms: 500,
      hold_seconds: 3
    }
  };
  page.on("dialog", (dialog) => dialog.accept());
  await page.route("**/*", async (route) => {
    const url = new URL(route.request().url());
    const path = url.pathname;
    if (!path.startsWith("/api/pi/v1/")) {
      if (path.startsWith("/api/media/active/")) {
        const audioFixture = await readFile(new URL("../../../content/audio/english/good-morning.wav", import.meta.url));
        await route.fulfill({
          body: audioFixture,
          headers: {
            "content-type": "audio/wav"
          }
        });
        return;
      }
      await route.continue();
      return;
    }

    if (path === "/api/pi/v1/status") {
      await route.fulfill({
        json: {
          status: "ok",
          service: "tcube-pi-admin",
          mode: "admin",
          database_present: true,
          ui_dist_present: true,
          media_root: "data/audio",
          content_root: "content",
          hostname: "tcube.local",
          usb_address: "10.55.0.1",
          usb_connected: true,
          contract_note: "test"
        }
      });
      return;
    }

    if (path === "/api/pi/v1/auth/session") {
      await route.fulfill({
        json: {
          authenticated: true,
          bootstrap_required: false,
          account: { id: "acct-1", username: "campingas", display_name: "Parent" },
          cubes: [{ device_id: "cube-1", label: "T-Cube", role: "owner" }]
        }
      });
      return;
    }

    if (path === "/api/pi/v1/setup/review") {
      await route.fulfill({
        json: {
          cube_name: "T-Cube",
          device_id: "cube-1",
          admin_created: true,
          wifi_verified: true,
          wifi_ssid: "Home",
          dashboard_ip: "192.168.50.159",
          dashboard_address: "https://tcube.local/",
          button_modes: {
            "1": "language:English",
            "2": "animals",
            "3": "music",
            "4": "setup_help",
            "5": "disabled"
          },
          active_counts: {
            language: 2,
            animals: 1,
            music: 1
          }
        }
      });
      return;
    }

    if (path === "/api/pi/v1/setup/pomodoro" && route.request().method() === "GET") {
      await route.fulfill({ json: pomodoroSettings });
      return;
    }

    if (path === "/api/pi/v1/setup/audio" && route.request().method() === "GET") {
      await route.fulfill({ json: audioSettings });
      return;
    }

    if (path === "/api/pi/v1/setup/audio" && route.request().method() === "PUT") {
      const body = route.request().postDataJSON();
      audioSettings = {
        volume_percent: body.volume_percent,
        updated_at: "2026-07-01T12:00:00.000Z"
      };
      await route.fulfill({ json: audioSettings });
      return;
    }

    if (path === "/api/pi/v1/setup/pomodoro" && route.request().method() === "PUT") {
      const body = route.request().postDataJSON();
      pomodoroSettings = {
        ...pomodoroSettings,
        ...body,
        validated_at: "2026-07-01T12:00:00.000Z",
        updated_at: "2026-07-01T12:00:00.000Z",
        recommendation: {
          preset: "focus",
          focus_minutes: 20,
          break_minutes: 5,
          cycles: 3,
          reason: "Focus plan for ages 9-12."
        }
      };
      await route.fulfill({ json: pomodoroSettings });
      return;
    }

    if (path === "/api/pi/v1/events/recent") {
      await route.fulfill({
        json: [
          {
            id: "activity-1",
            kind: "signed_in",
            occurred_at: new Date().toISOString(),
            button_id: null,
            button_label: null,
            mode: null,
            response_id: null,
            response_text: null,
            content_id: null,
            content_type: null,
            content_title: null,
            audio_filename: null,
            source: null,
            text: "campingas"
          },
          {
            id: "button-1",
            kind: "button_pressed",
            occurred_at: new Date().toISOString(),
            button_id: 1,
            button_label: "Top",
            mode: "language",
            response_id: "hello",
            response_text: "Hello",
            content_id: null,
            content_type: null,
            content_title: null,
            audio_filename: "apple_en_recorded.wav",
            source: null,
            text: "Hello"
          },
          {
            id: "activity-2",
            kind: "content_activated",
            occurred_at: new Date().toISOString(),
            button_id: 1,
            button_label: "Top",
            mode: null,
            response_id: null,
            response_text: null,
            content_id: "bird",
            content_type: "language",
            content_title: "Bird",
            audio_filename: "bird_en_voxtral.wav",
            source: "generated",
            text: "bird"
          }
        ]
      });
      return;
    }

    if (path === "/api/pi/v1/content/inventory") {
      await route.fulfill({
        json: {
          active_count: 4,
          draft_count: 1,
          unused_count: 1,
          items: [
            {
              id: "active-1",
              status: "active",
              button_id: 1,
              content_type: "language",
              language: "English",
              title: "Morning greeting",
              text: "Good morning",
              source: "generated",
              state: "active",
              audio_path: "active/language/morning-greeting.wav",
              preview_url: "/api/media/active/language/morning-greeting.wav",
              reason: "Playable by the current setup"
            },
            {
              id: "draft-1",
              status: "draft",
              button_id: 1,
              content_type: "language",
              language: "English",
              title: "Ready to review",
              text: "Review me",
              source: "recorded",
              state: "archived",
              audio_path: "draft/language/ready-to-review.wav",
              preview_url: "/api/media/draft/language/ready-to-review.wav",
              reason: "Inactive draft waiting for activation"
            },
            {
              id: "unused-1",
              status: "unused",
              button_id: 2,
              content_type: "animals",
              language: null,
              title: "Old animal sound",
              text: null,
              source: "uploaded",
              state: "active",
              audio_path: "active/animals/old-animal-sound.wav",
              preview_url: "/api/media/active/animals/old-animal-sound.wav",
              reason: "Hidden by current setup"
            }
          ]
        }
      });
      return;
    }

    if (path === "/api/pi/v1/setup/name" && route.request().method() === "POST") {
      await route.fulfill({
        json: {
          status: "ok",
          device_id: "cube-1",
          name: "Mia's Cube",
          provisioned: true,
          token: null
        }
      });
      return;
    }

    if (path === "/api/pi/v1/setup/wifi/verified" && route.request().method() === "POST") {
      await route.fulfill({ json: { status: "ok" } });
      return;
    }

    if (path === "/api/pi/v1/auth/recovery-code" && route.request().method() === "POST") {
      await route.fulfill({
        json: {
          code: "RCV-123-456",
          expires_at: "2026-07-06T00:00:00Z"
        }
      });
      return;
    }

    if (path === "/api/pi/v1/auth/invitations" && route.request().method() === "POST") {
      await route.fulfill({
        json: {
          id: "invite-1",
          code: "INV-789-ABC",
          device_id: "cube-1",
          role: "manager",
          expires_at: "2026-07-06T00:00:00Z"
        }
      });
      return;
    }

    if (path === "/api/pi/v1/content/unused" && route.request().method() === "DELETE") {
      await route.fulfill({ json: { status: "ok", deleted_count: 1 } });
      return;
    }

    if (path === "/api/pi/v1/setup/factory-reset" && route.request().method() === "POST") {
      await route.fulfill({ json: { status: "ok", bootstrap_required: true } });
      return;
    }

    if (path === "/api/pi/v1/auth/logout" && route.request().method() === "POST") {
      await route.fulfill({ json: { status: "ok" } });
      return;
    }

    if ((path === "/api/pi/v1/content/recordings" || path === "/api/pi/v1/content/uploads") && route.request().method() === "POST") {
      const source = path.endsWith("/recordings") ? "recorded" : "uploaded";
      await route.fulfill({
        json: {
          id: `${source}-draft`,
          content_type: "language",
          title: `${source}-english-bonjour-tout-le-monde.wav`,
          text: "Bonjour tout le monde.",
          language: "English",
          state: "archived",
          source,
          audio_path: `draft/language/${source}-english-bonjour-tout-le-monde.wav`,
          preview_url: `/api/media/draft/language/${source}-english-bonjour-tout-le-monde.wav`
        }
      });
      return;
    }

    if (path === "/api/pi/v1/content/generated-speech/status") {
      generatedSpeechStatusCalls += 1;
      await route.fulfill({
        json: generatedSpeechStatusCalls <= 2
          ? {
              online: false,
              provider: "voxtral",
              checked_at: new Date().toISOString(),
              cached: false,
              cache_ttl_seconds: 20,
              next_check_after_seconds: 20,
              message: "TTS provider is offline or unreachable: failed to connect to speech provider",
              voices: []
            }
          : {
              online: true,
              provider: "voxtral",
              checked_at: new Date().toISOString(),
              cached: false,
              cache_ttl_seconds: 20,
              next_check_after_seconds: 20,
              message: "TTS provider is online and ready for generated speech.",
              voices: ["neutral_male", "cheerful_female", "fr_female"]
            }
      });
      return;
    }

    if (path.startsWith("/api/pi/v1/content/items/") && route.request().method() === "DELETE") {
      await route.fulfill({ json: { status: "ok" } });
      return;
    }

    if (path.match(/^\/api\/pi\/v1\/setup\/buttons\/\d+\/mode$/)) {
      await route.fulfill({ json: { status: "ok" } });
      return;
    }

    const activeMatch = path.match(/^\/api\/pi\/v1\/content\/buttons\/(\d+)\/([^/]+)\/active$/);
    if (activeMatch) {
      await route.fulfill({
        json: {
          items: activeItems(activeMatch[1], activeMatch[2]),
          empty_state: null
        }
      });
      return;
    }

    const inactiveMatch = path.match(/^\/api\/pi\/v1\/content\/buttons\/(\d+)\/([^/]+)\/inactive$/);
    if (inactiveMatch) {
      await route.fulfill({
        json: {
          items: inactiveItems(inactiveMatch[1], inactiveMatch[2]),
          empty_state: null
        }
      });
      return;
    }

    await route.fulfill({
      status: 404,
      json: { detail: `Unhandled test route: ${path}` }
    });
  });
}

async function installMockRecordingApis(page: Page) {
  await page.addInitScript(() => {
    const stream = {
      getTracks: () => [{ stop: () => undefined }]
    };

    Object.defineProperty(Navigator.prototype, "mediaDevices", {
      configurable: true,
      value: {
        getUserMedia: async () => {
          (window as Window & { __getUserMediaCalls?: number }).__getUserMediaCalls =
            ((window as Window & { __getUserMediaCalls?: number }).__getUserMediaCalls ?? 0) + 1;
          return stream;
        }
      }
    });

    class MockMediaRecorder extends EventTarget {
      mimeType = "audio/webm";
      ondataavailable: ((event: BlobEvent) => void) | null = null;
      onstop: (() => void | Promise<void>) | null = null;
      state = "inactive";

      constructor() {
        super();
      }

      start() {
        this.state = "recording";
      }

      stop() {
        this.state = "inactive";
        const dataEvent = new BlobEvent("dataavailable", { data: new Blob(["mock audio"], { type: "audio/webm" }) });
        this.ondataavailable?.(dataEvent);
        void this.onstop?.();
      }
    }

    class MockAudioContext {
      sampleRate = 8000;
      state = "running";

      createMediaStreamSource() {
        return { connect: () => undefined };
      }

      createAnalyser() {
        return {
          fftSize: 256,
          getByteTimeDomainData: (data: Uint8Array) => {
            for (let index = 0; index < data.length; index += 1) {
              data[index] = 128 + Math.round(Math.sin(index / 4) * 42);
            }
          }
        };
      }

      async decodeAudioData() {
        return {
          duration: 1,
          length: 8000,
          numberOfChannels: 1,
          sampleRate: 8000,
          getChannelData: () => new Float32Array(8000)
        };
      }

      async close() {
        this.state = "closed";
      }
    }

    Object.defineProperty(window, "MediaRecorder", { configurable: true, value: MockMediaRecorder });
    Object.defineProperty(window, "AudioContext", { configurable: true, value: MockAudioContext });
  });
}

function activeItems(buttonId: string, contentType: string) {
  if (buttonId === "1" && contentType === "language") {
    return [
      activeItem(
        "lang-1",
        "language",
        "This file name is intentionally long enough to trim nicely.wav",
        "Hello",
        3
      ),
      activeItem("lang-2", "language", "Goodbye", "Goodbye", 1)
    ];
  }
  if (buttonId === "2" && contentType === "animals") {
    return [activeItem("animal-1", "animals", "Cat", "Meow")];
  }
  if (buttonId === "3" && contentType === "music") {
    return [activeItem("music-1", "music", "Clean up", "Clean up")];
  }
  return [];
}

function inactiveItems(buttonId: string, contentType: string) {
  if (buttonId === "1" && contentType === "language") {
    return [
      {
        ...activeItem("draft-1", "language", "Bonjour", "Bonjour"),
        state: "archived",
        audio_path: "draft/language/bonjour.wav",
        preview_url: "/api/media/draft/language/bonjour.wav"
      }
    ];
  }
  return [];
}

function activeItem(id: string, contentType: string, title: string, text: string, playCount = 0) {
  return {
    id,
    content_type: contentType,
    title,
    text,
    source: id === "lang-1" ? "generated" : "test",
    state: "active",
    audio_path: `active/${contentType}/${id}.wav`,
    preview_url: `/api/media/active/${contentType}/${id}.wav`,
    play_count: playCount
  };
}
