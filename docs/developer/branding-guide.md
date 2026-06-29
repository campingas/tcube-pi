# Branding Guide — tcube-pi

> Part of the `docs/developer/` suite. This file governs the visual design of the **admin UI dashboard** served by `tcube-pi-admin`.
> These guide share the same Wada palette foundation and type pairing. This guide wins for the admin UI.
>
> For stack, lint rules, and code patterns → `architecture-guide.md`
> For what the admin UI does → `FEATURES.md`
> For why it exists → `VISION.md`

---

## How to use this guide

Four layers, each independently iterable:

1. **Colour** — Wada palette tokens, dark-mode mappings, proportion rules, contrast audit
2. **Scale** — Type scale, spacing rhythm, radius tokens
3. **Voice** — Admin UI copy tone, label patterns, error and status language
4. **Checklist** — Pre-ship gate for admin UI components

When iterating, mark changed sections with a version note at the bottom. Keep old values commented out for one release cycle before deleting.

---

## Core constraint: dark mode only

The admin UI is **dark-mode only**. There is no light mode. There is no OS-preference detection. The `data-theme` attribute is always `dark` and is set on the `<html>` element at render time, not toggled.

**Rationale:** The admin UI is a parent tool used during setup and content management — often at a desk, often after a child's bedtime, often with the room dimmed. A dark interface reduces eye strain in that context, removes the complexity of a two-theme system, and makes the status indicators (LED colours, content state badges, button glow previews) more readable against a dark ground. The landing page handles light/dark switching; the admin UI does not need to.

The dark theme maps Wada tokens to dark surface roles as defined in §1.

---

## 1. Colour — Wada Foundation, Dark Mapping

### Palette source

The T-Cube admin UI shares the same Sanzo Wada palette source as the landing page. All hex values reference the reconciled W.S. Colors catalogue entries from `tcube-landing/docs/developer/branding-guide.md`. No new colours are introduced in this guide without a Wada reference.

Reference: [wscolors.com](https://wscolors.com) · [sanzo-wada.dmbk.io](https://sanzo-wada.dmbk.io)

---

### Shared palette tokens

These tokens are identical to the landing page. Do not redefine them — import from the shared token source when the monorepo structure supports it.

| Token       | Wada # | Wada name             | HEX      | Role in shared system                        |
|-------------|--------|-----------------------|----------|----------------------------------------------|
| `paper`     | 154    | White                 | `#ffffff` | Light surface (landing only in light mode)  |
| `paperSoft` | 39     | Sulpher Yellow        | `#f5ecc2` | Warm alternate surface (landing)            |
| `ink`       | 159    | Black                 | `#111314` | Deep dark base — primary dark surface here  |
| `inkMid`    | 158    | Slate Color           | `#34454c` | Mid-dark surface, cards, panels             |
| `inkSoft`   | 157    | Warm Gray             | `#a1a39a` | Muted text, labels, secondary metadata      |
| `coral`     | 19     | Etruscan Red          | `#c55347` | Primary accent — active states, primary CTA |
| `teal`      | 102    | Light Porcelain Green | `#00908a` | Success, active content, learning cues      |
| `amber`     | 70     | Khaki                 | `#bc892b` | Warning, draft/pending states               |
| `violet`    | 142    | Dark Soft Violet      | `#66629c` | Teacher mode, older-child content accent    |
| `sage`      | 99     | Pistachio Green       | `#648f7b` | Open/maker signals, secondary success       |

---

### Dark surface mapping

In the admin UI, the Wada tokens map to a layered dark surface system. Surfaces are built from the ink family; text is built from the paper family; accents carry the same signal meaning as the landing.

| CSS variable         | Value / Token         | Role                                                              |
|----------------------|-----------------------|-------------------------------------------------------------------|
| `--surface-base`     | `#0d0f10`             | Page background — slightly deeper than `ink` for depth           |
| `--surface-0`        | `ink` `#111314`       | Primary card surface, sidebars, nav background                   |
| `--surface-1`        | `#1a1e20`             | Raised card surface, table rows (odd), input backgrounds         |
| `--surface-2`        | `#22282b`             | Elevated surface — modals, dropdowns, tooltips                   |
| `--surface-3`        | `inkMid` `#34454c`    | Highest elevation — active selection, focused input border       |
| `--text-primary`     | `#f0ece4`             | Primary text — warm near-white derived from `paperSoft`          |
| `--text-secondary`   | `#b8b4ac`             | Secondary text, descriptions, panel subtitles                    |
| `--text-muted`       | `inkSoft` `#a1a39a`   | Labels, captions, metadata, placeholder text                     |
| `--text-disabled`    | `#545850`             | Disabled state text                                              |
| `--border`           | `rgba(240,236,228,0.07)` | Default border — barely visible divider                       |
| `--border-strong`    | `rgba(240,236,228,0.14)` | Card borders, section dividers                                |
| `--border-focus`     | `coral` `#c55347`     | Focused input ring                                               |
| `--accent-coral`     | `coral` `#c55347`     | Primary CTA, active button, destructive confirm                  |
| `--accent-teal`      | `teal` `#00908a`      | Active content badge, success state, confirmed actions           |
| `--accent-amber`     | `amber` `#bc892b`     | Draft badge, pending state, caution indicator                    |
| `--accent-violet`    | `violet` `#66629c`    | Teacher-mode label, older-child content tag                      |
| `--accent-sage`      | `sage` `#648f7b`      | Open hardware, secondary success, safe/trusted indicator         |
| `--glow-coral`       | `rgba(197,83,71,0.18)` | Coral button glow, active CTA background wash                   |
| `--glow-teal`        | `rgba(0,144,138,0.15)` | Teal badge glow, active content wash                            |
| `--glow-amber`       | `rgba(188,137,43,0.15)` | Amber draft glow, warning wash                                 |

`--surface-base` is the only value that departs from the named Wada palette. It is one step darker than `ink` to give the layered surface system enough range. All other values map directly to Wada tokens.

---

### Proportion rule

Same Wada proportion logic as the landing, adapted for a dashboard context:

| Weight | Role                                      | Tokens in dark UI                               |
|--------|-------------------------------------------|-------------------------------------------------|
| 60–70% | Dominant — page and card surfaces         | `--surface-base`, `--surface-0`, `--surface-1`  |
| 20–30% | Supporting — text, borders, structure     | `--text-primary`, `--text-secondary`, `--border`|
| 5–10%  | Accent — status, CTAs, badge signals      | `coral`, `teal`, `amber`, `violet`, `sage`      |

In a dashboard, accent colours carry semantic meaning (active, draft, warning, error). They should never be used purely decoratively. If an accent colour appears without a status reason, remove it.

---

### Status semantics — accent colour contracts

These mappings are contracts, not suggestions. Every status badge, indicator dot, and state label in the admin UI must follow them.

| State              | Accent      | Glow var          | Never use for                           |
|--------------------|-------------|-------------------|-----------------------------------------|
| Active / live      | `teal`      | `--glow-teal`     | Draft, pending, error, warning          |
| Draft / inactive   | `amber`     | `--glow-amber`    | Active, error, success                  |
| Error / destructive| `coral`     | `--glow-coral`    | Active content (could be misread as live)|
| Primary CTA        | `coral`     | `--glow-coral`    | Status indicators (reserve for actions) |
| Teacher / older    | `violet`    | none              | General status                          |
| Trusted / open     | `sage`      | none              | Warnings or errors                      |
| Disabled           | none        | none              | Any colour — disabled is colourless     |

**The coral rule:** `coral` carries two meanings in this system — primary CTA and error/destructive. This is intentional: both are high-attention states that require a deliberate action. They are distinguished by context (a button vs. a badge) and by copy. Never use coral for passive status indicators.

---

### Contrast audit — dark surfaces (WCAG AA)

| Pair                                    | Ratio | Level    | Notes                                    |
|-----------------------------------------|-------|----------|------------------------------------------|
| `--text-primary` on `--surface-base`    | 13.4  | AAA      | All body text and headings               |
| `--text-secondary` on `--surface-0`     | 6.8   | AA       | Descriptions, panel subtitles            |
| `--text-muted` on `--surface-0`         | 4.6   | AA       | Labels at `sm` size and above only       |
| `--text-muted` on `--surface-1`         | 3.9   | Large AA | Captions and metadata at `md`+ only      |
| `--text-disabled` on `--surface-1`      | 1.8   | Fail     | Intentional — disabled must not be read  |
| `coral` on `--surface-0`                | 4.2   | Large AA | CTA labels, error badges at `md`+        |
| `teal` on `--surface-0`                 | 3.8   | Large AA | Active badges at `md`+, not body copy    |
| `amber` on `--surface-0`                | 3.3   | Large AA | Draft badges at `lg`+ only               |
| `--text-primary` on `coral`             | 3.1   | Large AA | CTA button labels — use `font-weight:500`+|
| `--text-primary` on `--surface-3`       | 8.7   | AAA      | Focused input text                       |

Never use `amber` or `teal` for normal-size body text. They fail AA at `base` size on dark surfaces.

---

### Usage rules

- **Page background**: always `--surface-base`. Never `surface-0` for the outermost layer.
- **Cards and panels**: `--surface-0` on `--surface-base`. Raised panels use `--surface-1`.
- **Modals and dropdowns**: `--surface-2`. Add a `--border-strong` ring.
- **Text**: `--text-primary` for headings and body. `--text-secondary` for descriptions. `--text-muted` for labels and captions only — never for full sentences.
- **Inputs**: `--surface-1` background, `--border-strong` default border, `--border-focus` (coral) on focus.
- **Buttons**: Primary CTA uses `coral` background with `--text-primary`. Secondary/ghost uses `--surface-1` with `--border-strong` and `--text-secondary`.
- **Destructive actions**: `coral` background. Always require confirmation — never one-click.
- **Status badges**: follow the semantic contract table above without exception.
- **Cube face preview**: in the admin UI, cube button previews use the five accent colours (`coral`, `teal`, `amber`, `violet`, `sage`) on a `--surface-1` card with a subtle glow from the corresponding `--glow-*` variable. The cube body is `--text-primary` (warm near-white) in the admin dark theme.
- **Never**: pure black backgrounds (`#000`), saturated neon accents, blue or purple gradients, white (#ffffff) as a surface colour, or any hex value not in this table.

---

## 2. Scale

### 2a. Type scale

The admin UI uses `DM Sans` for all text including headings. `DM Serif Display` appears only in the logo/wordmark in the nav bar — not in any heading, label, or UI copy.

**Rationale:** Display serifs in a functional dashboard add visual weight without aiding usability. The admin UI is read at close range by a parent managing content, not scanned from a distance. DM Sans at varying weights provides all the hierarchy needed.

Root: `15px` (tighter than the landing's 17px — dashboard density requires it). Modular ratio: 1.2.

| Token  | rem   | Approx px | Use                                               |
|--------|-------|-----------|---------------------------------------------------|
| `xs`   | 0.694 | 10        | Timestamps, secondary metadata, version strings   |
| `sm`   | 0.833 | 12        | Labels, badges, table cells, form helper text     |
| `base` | 1     | 15        | Body baseline, input values, descriptions         |
| `md`   | 1.2   | 18        | Card headings, section subtitles, nav items       |
| `lg`   | 1.44  | 22        | Panel headings, primary section titles            |
| `xl`   | 1.728 | 26        | Page-level headings (used sparingly)              |
| `2xl`  | 2.074 | 31        | Reserved for single-page hero moments (setup complete, empty state headers) |

No `clamp()` fluid sizing in the admin UI — it is accessed from a browser at a known viewport range (phone to desktop). Fixed rem values are sufficient and more predictable in a tool context.

Font weights in use:
- `300` — captions, timestamps, metadata
- `400` — body copy, descriptions, input values
- `500` — labels, nav items, badge text, secondary headings
- `600` — primary headings, CTA labels, status emphasis
- `700` — reserved for the logo wordmark and critical alert headings only

---

### 2b. Spacing rhythm

Same Fibonacci-φ rhythm as the landing, tightened by one step for dashboard density.

| Token       | px  | Use                                                     |
|-------------|-----|---------------------------------------------------------|
| `rhythm-1`  | 4   | Icon-to-label gap, inline tag padding                   |
| `rhythm-2`  | 8   | Component inner padding, button padding (tight)         |
| `rhythm-3`  | 12  | Card inner padding, form field gap, table cell padding  |
| `rhythm-5`  | 20  | Between related elements in a card                      |
| `rhythm-8`  | 32  | Card-to-card gap, section inner padding                 |
| `rhythm-13` | 52  | Between major sections on the page                      |
| `rhythm-21` | 84  | Page-level vertical breathing room (top of main content)|

Dashboard pages should feel dense but not cramped. `rhythm-3` is the default inner padding for cards. `rhythm-5` is the default gap between cards in a grid.

---

### 2c. Radius tokens

| Token      | px  | Use                                                       |
|------------|-----|-----------------------------------------------------------|
| `sm`       | 4   | Badges, status dots, small tag chips                      |
| `base`     | 8   | Input fields, small buttons, table row selections         |
| `md`       | 12  | Cards, panels, dropdowns                                  |
| `lg`       | 16  | Modal containers, large card groups                       |
| `pill`     | 999 | Primary CTA buttons, nav CTA                              |

The admin UI is flatter and more angular than the landing page. This is intentional — a tool should feel precise, not soft. `md` (12px) is the default card radius. The landing page's `radiusLg` (24px) is too soft for dashboard cards.

---

### 2d. Icon system

Use a single icon family throughout. Use **Lucide** via the installed `@lucide/svelte` package (MIT, consistent stroke weight, Svelte-compatible).

Rules:
- Import icons from `@lucide/svelte`; do not install the deprecated `lucide-svelte` package.
- All icons `20px` in standard UI contexts, `16px` in compact table rows and badges, `24px` in empty states and primary nav.
- Stroke weight `1.5px` consistent — never mix stroke weights within a component.
- Icon colour inherits from the surrounding text colour token. Never set icon colour independently of its label.
- Never use an icon without an accessible label (`aria-label` or adjacent visible text).
- Decorative icons that add no semantic meaning should be `aria-hidden="true"`.

---

## 3. Voice — Admin UI Copy

The admin UI speaks differently from the landing page. The landing persuades; the admin UI instructs. The parent using the admin UI already believes in T-Cube — they built one. They need clear, direct, error-free guidance.

---

### The three-word model

**Clear** — Every label, heading, and message tells the parent exactly what is happening or what to do. No mystery.

> ✗ "Content operation completed successfully."
> ✓ "Recording saved as draft."

**Direct** — No padding words. No marketing language. No brand voice in the tool.

> ✗ "Empowering your T-Cube experience with new content."
> ✓ "Add a recording for Button 1."

**Honest** — Errors explain what went wrong and what to do next. Success messages confirm exactly what changed. Empty states explain why something is empty, not just that it is.

> ✗ "Error: operation failed."
> ✓ "Upload failed — file must be MP3 or WAV under 10 MB."

---

### Label patterns

Labels in the admin UI follow consistent patterns. Apply them without variation.

| Context              | Pattern                           | Example                              |
|----------------------|-----------------------------------|--------------------------------------|
| Button mode          | `[Mode]` noun only                | `Language` · `Animals` · `Disabled`  |
| Content status       | Past participle                   | `Active` · `Draft` · `Trashed`       |
| Action buttons       | Verb + object                     | `Save recording` · `Activate` · `Move to trash` |
| Destructive actions  | Verb + consequence                | `Delete recording` · `Revoke session`|
| Empty states         | Explain + action                  | `No active content for this button — upload or generate a recording.` |
| Error messages       | What + why + fix                  | `Login failed — check your password and try again.` |
| Success messages     | Confirm + what changed            | `Password updated. Your previous session has been revoked.` |
| Loading states       | Present progressive               | `Saving…` · `Uploading…` · `Generating speech…` |
| Setup prerequisites  | Missing item as noun phrase       | `No active language content` · `Wi-Fi not verified` |

---

### Things never to write in the admin UI

- Marketing language: "powerful", "seamless", "revolutionary", "next-generation"
- Vague errors: "Something went wrong", "An error occurred", "Please try again"
- Orphaned success: "Done!" (done what?)
- Passive voice in errors: "Content could not be activated" → "Activation failed — content file not found"
- Ellipsis abuse: `Saving content...` → `Saving content…` (use the actual ellipsis character `…` not three dots)
- Exclamation marks: the admin UI never uses them, even for success

---

### The activation contract in copy

Content activation is the most important state transition in the admin UI. Copy around it must be precise:

- **Inactive / Draft** — visible to parent, never heard by child
- **Active** — can be played by the child on next button press
- **Trashed** — removed from both states, not deleted from filesystem until cleanup

Never use "published", "live", "enabled", or "approved" as synonyms for active. The word is `Active`. This is the only word used in UI labels, API responses, and documentation.

---

## 4. Component Direction

### Navigation

Fixed left sidebar on desktop, bottom tab bar on mobile. The nav carries:
- The T-Cube wordmark (DM Serif Display, `lg` size, `--text-primary`, coral dot separator)
- The cube name (editable, `sm` size, `--text-muted`)
- Primary nav items: Setup, Buttons, Content, Settings
- Session status (account name, role badge, logout)

The sidebar background is `--surface-0`. Nav items use `--text-secondary` at rest, `--text-primary` + left coral bar on active.

---

### Dashboard / Status panel

The first view after login. Shows:
- Service status row (database, UI, media, content) — each as a small badge: `teal` for OK, `amber` for warning, `coral` for error
- Cube name and Wi-Fi status
- Setup prerequisites checklist — each item as a row with a status icon and a direct action link
- Recent activity feed from the SQLite event log — timestamps in `xs`, event descriptions in `sm`

Status indicators must always show a text label alongside the colour signal. Never rely on colour alone.

---

### Button configuration cards

Five cards, one per cube face. Each card shows:
- The face position (Top, Front-Left, Front-Right, Back-Left, Back-Right)
- The current mode badge in the appropriate accent colour
- The configured language (if language mode)
- A count of active content items
- A quick-action link to manage content

Cards use `--surface-0` on `--surface-base`, `--border-strong` ring, `md` radius. The mode badge uses the accent colour contract from §1. No glow on the card itself — glow is reserved for the button shape preview inside the card.

---

### Content management

A two-column layout: active content list on the left, draft content list on the right. Each content item is a row with:
- Filename or generated label (truncated, `sm`, `--text-primary`)
- Content type icon (Lucide, `16px`)
- Status badge (`Active` in teal, `Draft` in amber)
- Actions: Activate / Move to trash / Preview
- Duration or file size in `xs` `--text-muted`

Drag-to-reorder is not in v1. Order is managed through activation sequence.

---

### Media upload and recording

A card with three tabs: Record, Upload, Generate. Each tab is a distinct flow:

**Record**: browser microphone capture with a waveform visualiser (simple CSS animation, not canvas), preview playback, and a submit button. The recording is labelled `Draft` immediately on submit.

**Upload**: drag-and-drop zone accepting MP3 and WAV. File validation happens before upload — wrong format or over-size files are rejected with a specific error message before the request is made.

**Generate**: a form with provider selection (of differents TTS models installed), optional voice selector, and a text input for the sentence to generate. Submit creates a draft. The parent reviews before activating.

All three flows end in the same state: a new draft item in the inactive content list.

---

### Setup flows

Setup is a linear flow gated by prerequisites. The UI shows which prerequisites are unmet and links directly to the action that resolves each one. The setup completion button is disabled until all prerequisites are met. The disabled state includes a tooltip listing unmet items.

Setup completion is a one-time action that cannot be undone from the UI. The confirmation step uses a `coral` confirm button and a plain-language warning: "Completing setup switches the cube to child mode. You can still manage content from this dashboard."

---

### Empty states

Every list, every content area, every status panel has an explicit empty state. Empty states follow the pattern:

```
[Lucide icon, 24px, --text-muted]
[Heading, md, --text-secondary]  "No active content for Button 1"
[Description, sm, --text-muted]  "Upload or generate a recording to get started."
[Optional CTA, coral pill button] "Add recording"
```

Never show a blank area. Never show a spinner that never resolves. If data is loading, show a skeleton. If data is empty, explain why and offer the next action.

---

### Error and loading states

**Loading**: skeleton screens (not spinners) for list views. Spinners only for point actions (submit, upload, generate) where the interaction is momentary.

**API errors**: shown inline near the action that triggered them, not in a global toast. The error message follows the "what + why + fix" pattern from §3.

**Session expiry**: redirect to login with a message: "Your session expired — log in again to continue."

**Network error**: "Could not reach the Pi. Check that you're on the home network." — with a retry button.

---

## Quick Reference Card

For use when writing Svelte components, reviewing PRs, or checking designs.

- **Always dark mode.** `data-theme="dark"` on `<html>`. No toggle, no OS preference.
- **Surfaces**: `--surface-base` → `--surface-0` → `--surface-1` → `--surface-2`. Never skip a layer.
- **Text**: `--text-primary` for body and headings. `--text-secondary` for descriptions. `--text-muted` for labels and captions only.
- **Accent contracts**: `teal` = active/success · `amber` = draft/warning · `coral` = CTA/error · `violet` = teacher mode · `sage` = open/trusted.
- **Typography**: `DM Sans` for all UI text. `DM Serif Display` only for the logo wordmark. Root `15px`, ratio 1.2.
- **Spacing**: `rhythm-3` (12px) default card padding. `rhythm-5` (20px) between cards. `rhythm-8` (32px) between sections.
- **Radii**: `base` (8px) for inputs. `md` (12px) for cards. `pill` for primary CTA buttons.
- **Icons**: Lucide, `20px` standard / `16px` compact / `24px` empty state. Stroke `1.5px`. Always labelled.
- **Copy**: Clear, Direct, Honest. No marketing language. No vague errors. No exclamation marks.
- **Status**: `Active` (teal) / `Draft` (amber) / `Trashed` (no colour) — these three words only, no synonyms.
- **Never**: pure black backgrounds, neon accents, colour-only status signals, `DM Serif Display` in UI headings, `amber` or `teal` for body text.

---

## Relationship to Landing Page Branding Guide

| Dimension        | Landing page                        | Admin UI (this guide)               |
|------------------|-------------------------------------|-------------------------------------|
| Theme            | Light default, dark on toggle       | Dark only, always                   |
| Type display     | DM Serif Display for headings       | DM Sans for all headings            |
| Type body        | DM Sans                             | DM Sans                             |
| Root size        | 17px                                | 15px                                |
| Card radius      | 24px (radiusLg)                     | 12px (md)                           |
| Palette source   | Wada, shared tokens                 | Wada, same shared tokens            |
| Accent semantics | Expressive / emotional              | Semantic / status-driven            |
| Copy voice       | Confident, Warm, Cheerful, Legible  | Clear, Direct, Honest               |
| Audience         | Parent discovering T-Cube           | Parent managing their T-Cube        |

The palette is the thread that makes both surfaces feel like the same product. Everything else is adapted to the job.

---

## Version Notes

- 2026-06-28: Initial guide created for `tcube-pi` admin UI. Dark-mode-only. Harmonized with `tcube-landing` Wada palette. DM Serif Display restricted to logo wordmark. Dashboard-specific component direction added for all major UI surfaces.
