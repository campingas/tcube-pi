# Branding Guide

This guide governs the admin UI served by `tcube-pi-admin`. Load it for admin UI visual design, copy, layout, status colors, and component work.

## Quick Reference

- Admin UI is dark-mode-only; no light mode and no OS preference toggle.
- Primary audience is a parent managing one local cube.
- UI tone is Clear, Direct, Honest.
- Use `DM Sans` for UI text. Use `DM Serif Display` only for the logo/wordmark.
- Use Lucide icons through `@lucide/svelte`.
- Status colors must include text labels; never rely on color alone.
- Use compact, tool-like layouts; avoid marketing-page composition.
- Content state words are `Active`, `Draft`, and `Trashed`; do not use synonyms.
- Activation copy must make clear that drafts are never heard by the child.
- Destructive actions require confirmation.

## Color Tokens

The admin UI uses the T-Cube Wada palette in a dark dashboard mapping.

| Role | Token | Value | Use |
| --- | --- | --- | --- |
| Base surface | `--surface-base` | `#0d0f10` | Page background |
| Primary surface | `--surface-0` | `#111314` | Cards, nav, panels |
| Raised surface | `--surface-1` | `#1a1e20` | Inputs, rows, raised panels |
| Elevated surface | `--surface-2` | `#22282b` | Modals, dropdowns, tooltips |
| Strong surface | `--surface-3` | `#34454c` | Focused or selected surfaces |
| Primary text | `--text-primary` | `#f0ece4` | Headings and body |
| Secondary text | `--text-secondary` | `#b8b4ac` | Descriptions |
| Muted text | `--text-muted` | `#a1a39a` | Labels and metadata |
| Primary/action/error | `--accent-coral` | `#c55347` | CTAs, destructive, errors |
| Active/success | `--accent-teal` | `#00908a` | Active content, success |
| Draft/warning | `--accent-amber` | `#bc892b` | Drafts, pending, warnings |
| Teacher/older | `--accent-violet` | `#66629c` | Teacher or older-child cues |
| Trusted/open | `--accent-sage` | `#648f7b` | Trusted/open secondary success |

Never introduce arbitrary hex values unless the design system is intentionally updated.

## Status Semantics

| State | Accent | Notes |
| --- | --- | --- |
| Active/live | Teal | Child can hear this content |
| Draft/inactive | Amber | Parent can review; child cannot hear it |
| Error/destructive | Coral | Requires attention or confirmation |
| Primary action | Coral | Deliberate parent command |
| Disabled | No accent | Disabled is colorless |

Do not use teal or amber for normal body text; reserve them for labeled statuses and controls.

## Type, Spacing, And Shape

- Root size: `15px`.
- Type scale: `xs` 10px, `sm` 12px, `base` 15px, `md` 18px, `lg` 22px, `xl` 26px.
- Default body/field text is `base`; compact labels and metadata use `sm` or `xs`.
- Spacing rhythm: 4, 8, 12, 20, 32, 52, 84px.
- Default card padding is 12px or 20px depending on density.
- Radii: 4px badges, 8px inputs/buttons, 12px cards, 16px modals, pill only for primary CTA buttons.
- Do not use viewport-scaled font sizes in app UI.

## Icons

- Use Lucide icons from `@lucide/svelte`.
- Standard icon size is 20px; compact rows/badges use 16px; empty states may use 24px.
- Stroke width is 1.5px.
- Icon color inherits surrounding text or status color.
- Icons need either visible adjacent text or an `aria-label`; decorative icons use `aria-hidden="true"`.

## Copy Rules

Clear:

- Say exactly what changed or what the parent must do.
- Prefer `Recording saved as draft.` over `Operation completed successfully.`

Direct:

- Avoid marketing language in the tool.
- Prefer `Add recording` over `Empower your cube with new content.`

Honest:

- Explain errors with what happened, why, and how to recover.
- Prefer `Upload failed - file must be MP3 or WAV under 25 MB.`

Never write vague errors, exclamation marks, orphaned success messages, or passive failures.

## Component Direction

- Dashboard: compact status, cube state, setup checklist, button access, and recent activity.
- Button configuration: full workflow for mode, active content, drafts, record, upload, and generate.
- Content rows: filename/label, source or metadata, status, preview, and icon-only actions with accessible labels.
- Recording/upload/generate flows all create drafts first.
- Settings: grouped local cube/account/admin controls, with owner-only actions visibly disabled for non-owners.
- Empty states always explain why the area is empty and point to the next action.
- Loading list views should use skeletons; point actions may use inline busy states.

## Landing Page Difference

The landing page can be expressive and promotional. The admin UI is a dense local tool. Keep it quieter, more direct, and more predictable than the marketing surface.
