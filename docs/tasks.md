# Tasks

## Active

- [x] Keep Pi admin service behind Caddy on loopback HTTP.
- [ ] Validate `tcube-pi` on Raspberry Pi Zero 2 W target hardware.
- [ ] Measure Pi resource usage and admin-load impact with `just measure-pi-admin`.
- [ ] Implement Raspberry Pi GPIO input backend and validate one physical button.
- [ ] Validate MAX98357A I2S audio output and local cached-content playback on Pi Zero 2 W.
- [ ] Validate mini USB microphone capture through the Pi Zero 2 W OTG port.
- [ ] Implement LED output backend and mandatory microphone-active indication.
- [ ] Define microphone capture, retention, upload, and physical indicator privacy rules.
- [ ] Define local SQLite schema versioning and migrations.
- [ ] Add acknowledged upload and retention handling for cube event and microphone data.
- [x] Prepare GitHub Releases CI/CD for installable `tcube-pi` Pi Zero 2 W artifacts.

## Admin UI Follow-Up

- [x] Add a credentialed browser smoke test for the mobile-first dashboard as user `campingas`.
- [x] Verify every button card lists active and draft audio when button language, content language, or button mode changes.
- [x] Add API coverage for language-button fallback content so existing audio remains visible after changing the button language.
- [x] Add an admin UI empty/error state that distinguishes "no content exists" from "content exists for another language or button".
- [x] Add visible API error text near the Active/Drafts tabs when content listing fails.
- [x] Add a content inventory view for all active and draft audio across every button.
- [ ] Wire Learning stats and Run curation only after their local API contracts exist; keep disabled actions visually distinct until then.
- [x] Add Playwright mobile viewport checks for dashboard stacking, button selector sizing, and button-config icon updates.
- [x] Confirm generated, recorded, and uploaded drafts all appear in the Drafts tab before activation.
- [ ] Confirm active content preview URLs work through the same-origin `/api/pi/v1/` admin path behind Caddy.
- [x] the Tcube logo should be clickable to got back to home.
- [x] just under the logo there is Signed in as campingas · owner
    - the username should be in a different color and owner or admin have also their own color.
- [x] Change the text and icon of Admin in the menu, next to wifi to LLMs offline with warning color.
- [x] In button configuration page in the active list, display the audio like this :
    - name of the audio audio.wav only trim is exeed a number of characters
    - rigth under show Default/Recorded/Uploaded/Generated · duration · x plays
    - do not show the playing module, the whole cell should be clickable to play the audio
    - at the end the trash icon do what's expected with a beautifull altert not the default one.
- [x] When record an audio its not very clear when it start/stop to record on the device and if it working. we do need more feedbacks like a simple oscilating line when it capture sounds. and add some hint that guide the process until its at a draft state.
- [x] for add content in generated, if the app did detect that the LLM for TTS is offline, the user should be noticed and all btn should be unclickable untile it detect it back online. but it should be an efficient way of doing it not a spamming api call.

## Later

- [ ] Prepare full flashable SD-card image artifacts after hardware and systemd validation.
- [ ] Clarify future sync use case: who publishes content packages, where packages are hosted, whether this is GitHub Releases, Mac-local curation, parent-to-device transfer, or cloud sync, and what auth/rollback/privacy guarantees are required.
- [ ] Define future content package signing, admin publication controls, retention, and rollback requirements after the sync use case is clarified.
- [ ] Add systemd deployment for the native Pi runtime.
- [ ] Revisit Docker runtime deployment after native GPIO and audio are validated.
