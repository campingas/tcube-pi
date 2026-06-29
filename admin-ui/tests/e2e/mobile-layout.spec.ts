import { readFile } from "node:fs/promises";
import { expect, test, type Page } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await mockAdminApi(page);
});

test("mobile dashboard stacks primary cards and keeps stats within the viewport", async ({ page }) => {
  await page.goto("/");

  const hero = page.getByTestId("dashboard-hero-card");
  const inventory = page.getByTestId("dashboard-inventory-card");
  const buttons = page.getByTestId("dashboard-buttons-card");
  await expect(hero).toBeVisible();
  await expect(inventory).toBeVisible();
  await expect(buttons).toBeVisible();

  const boxes = await Promise.all([hero.boundingBox(), inventory.boundingBox(), buttons.boundingBox()]);
  expect(boxes.every(Boolean)).toBe(true);
  const [heroBox, inventoryBox, buttonsBox] = boxes as NonNullable<(typeof boxes)[number]>[];

  expect(inventoryBox.y).toBeGreaterThan(heroBox.y + heroBox.height - 1);
  expect(buttonsBox.y).toBeGreaterThan(inventoryBox.y + inventoryBox.height - 1);
  expect(Math.abs(heroBox.x - inventoryBox.x)).toBeLessThanOrEqual(1);
  expect(Math.abs(inventoryBox.x - buttonsBox.x)).toBeLessThanOrEqual(1);
  expect(heroBox.width).toBeLessThanOrEqual(390);

  const statBoxes = await page.getByTestId("dashboard-stats").getByRole("listitem").evaluateAll((items) =>
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

test("mobile button selector keeps five fixed-size button pills horizontally usable", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /Manage all/i }).click();

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

  const activeRow = page.getByRole("button", { name: /Play This file name is intentionally long/ });
  await expect(activeRow).toBeVisible();
  await expect(activeRow.getByRole("button", { name: "Move to trash" })).toBeVisible();
  await expect(activeRow.locator("audio")).toHaveCount(0);

  await expect(page.getByTitle("This file name is intentionally long enough to trim nicely.wav")).toBeVisible();
  await expect(page.getByText("This file name is intentionally…")).toBeVisible();
  await expect(page.getByText(/Generated · 0:0\d · x plays/)).toBeVisible();
  await expect(page.getByText(/Generated · 0:01 · x plays/)).toBeVisible({ timeout: 5000 });

  await activeRow.click();

  await activeRow.getByRole("button", { name: "Move to trash" }).click();
  const dialog = page.getByRole("dialog", { name: /Move audio to trash\?/ });
  await expect(dialog).toBeVisible();
  await expect(dialog.getByText(/will be removed from the active list and deleted from disk/)).toBeVisible();
  await expect(dialog.getByRole("button", { name: "Move to trash" })).toBeVisible();
  await dialog.getByRole("button", { name: "Cancel" }).click();
  await expect(dialog).toHaveCount(0);
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

  await expect(page.getByTestId("record-zone")).toBeVisible();
  await expect(page.getByTestId("record-status")).toHaveText("Tap record, then speak clearly near your phone.");
  await expect(page.getByText("After recording, preview the audio here before saving.")).toBeVisible();

  const saveButton = page.getByRole("button", { name: "Save recording" });
  await expect(saveButton).toBeDisabled();

  await page.getByTestId("record-toggle").click();
  expect(await page.evaluate(() => (window as Window & { __getUserMediaCalls?: number }).__getUserMediaCalls ?? 0)).toBe(1);
  await expect(page.getByTestId("record-status")).toContainText("Recording");
  await expect(page.getByTestId("record-waveform")).toBeVisible();
  await expect(page.getByRole("button", { name: "Stop recording" })).toBeVisible();

  await page.getByTestId("record-toggle").click();
  await expect(page.getByTestId("record-status")).toContainText("Preview");
  await expect(page.locator(".record-zone audio")).toBeVisible();
  await expect(page.getByText("Enter the text spoken before saving this recording.")).toBeVisible();
  await expect(saveButton).toBeDisabled();

  await page.getByLabel("Text spoken").fill("Bonjour tout le monde.");
  await expect(saveButton).toBeEnabled();
});

test("generated speech disables only generate controls while TTS is offline", async ({ page }) => {
  await page.clock.install();
  await page.goto("/");
  await expect(page.getByText("LLMs offline")).toBeVisible();
  await page.getByTestId("dashboard-button-1").click();

  await page.getByRole("tab", { name: /Generate/i }).click();
  await expect(page.getByTestId("tts-offline-notice")).toContainText("TTS provider is offline");
  await expect(page.getByLabel("Text to speech")).toBeDisabled();
  await expect(page.getByLabel("Provider")).toBeDisabled();
  await expect(page.getByRole("button", { name: "Generate speech" })).toBeDisabled();

  await page.getByRole("tab", { name: /Record/i }).click();
  await expect(page.getByTestId("record-toggle")).toBeEnabled();
  await expect(page.getByLabel("Text spoken")).toBeEnabled();

  await page.getByRole("tab", { name: /Generate/i }).click();
  await page.clock.fastForward(31_000);
  await expect(page.getByTestId("tts-offline-notice")).toHaveCount(0);
  await page.clock.fastForward(61_000);
  await expect(page.getByLabel("Text to speech")).toBeEnabled();
  await page.getByLabel("Text to speech").fill("Bonjour tout le monde.");
  await expect(page.getByRole("button", { name: "Generate speech" })).toBeEnabled();
  await page.getByRole("button", { name: /go to dashboard/i }).click();
  await expect(page.getByText("LLMs online")).toBeVisible();
});

async function mockAdminApi(page: Page) {
  let generatedSpeechStatusCalls = 0;
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

    if (path === "/api/pi/v1/events/recent") {
      await route.fulfill({
        json: [
          {
            occurred_at: new Date().toISOString(),
            button_id: 1,
            mode: "language",
            response_id: "hello",
            response_text: "Hello"
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
          items: []
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
              message: "TTS provider is offline or unreachable: failed to connect to speech provider"
            }
          : {
              online: true,
              provider: "voxtral",
              checked_at: new Date().toISOString(),
              cached: false,
              cache_ttl_seconds: 20,
              next_check_after_seconds: 20,
              message: "TTS provider is online and ready for generated speech."
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
        "Hello"
      ),
      activeItem("lang-2", "language", "Goodbye", "Goodbye")
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

function activeItem(id: string, contentType: string, title: string, text: string) {
  return {
    id,
    content_type: contentType,
    title,
    text,
    source: id === "lang-1" ? "generated" : "test",
    state: "active",
    audio_path: `active/${contentType}/${id}.wav`,
    preview_url: `/api/media/active/${contentType}/${id}.wav`
  };
}
