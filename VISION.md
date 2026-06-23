# T-Cube Vision

T-Cube is a physical, AI-assisted ludic cube toy for toddlers.

## Product Contract

T-Cube must be:

* Physical first
* Screen-free for the child
* Menu-free
* Offline-first
* Immediate in response
* Deterministic in child-facing behavior
* Simple enough for a 12-month-old to understand

T-Cube could evolve with the toddler with:

* A chatbot
* A smart speaker
* A motorise cube

## Target Users

Primary child user: 1 - 2 years old.

Future child range: 2-7 years old.

Parent/admin user: an adult who reviews usage, manages content, and maintains the device.

## Child Interaction

The child interacts only through physical buttons.

Idle state:

* The active button glows softly.
* The child sees a large glowing button.
* A button press produces immediate feedback.

Primary loop:

1. Child presses a button.
2. Cube plays a sound, phrase, or music response with very low latency.
3. Cube records the interaction event.
4. Optional short background audio capture supports later analysis.

Success means the child understands cause and effect, repeats the interaction without help, and remains engaged.

## Hardware Concept

Initial form: soft-edged cube, roughly 20-30 cm per side.

Interaction: five active faces, each with one large illuminated arcade-style button.

Button modes:

* Languages short sentences
* Animal sounds
* Music

Power and connectivity:

* Battery powered
* USB-C charging
* Local administration over Wi-Fi
* USB-C administration if practical

Preferred prototype platform: Raspberry Pi Zero 2 W with pre-soldered GPIO headers.

Rationale: GPIO support, Linux userland, local storage, low power consumption, small size, and enough capacity for deterministic playback and synchronization when AI and admin services run on a local network hardware (MacOS).

## Content Model

Language responses must be short, usually eight words or fewer.

Content should combine repetition and freshness:

* Keep a stable weekly sentence set.
* Add new sentences gradually.
* Avoid fully unpredictable child-facing responses.

## Parent and Admin Experience

The cube exposes a local dashboard for adults.

The dashboard should show:

* Button usage patterns
* Favorite modes
* Language exposure trends
* Attention and engagement signals
* Weekly development summaries

The dashboard should support:

* Content updates
* Software updates
* Local review of collected data

## AI Boundaries

AI is allowed for background processing only:

* Transcription
* Categorization
* Summarization
* Report generation
* Parent insight generation
* Content suggestions

AI must not block button feedback.

Child-facing playback must come from local deterministic content.

AI and content generation run on a local MacOs service for the current prototype. The cube approved content and always plays it locally, so child-facing behavior remains available.

## Data and Privacy

The cube records button events.

Short audio capture is optional and must be treated as sensitive local data.

Before implementation, define:

* What audio is captured
* How long audio is retained
* What derived summaries are stored
* Who can access the dashboard
* How parents can disable capture

## Engineering Constraints

* Rust for child-facing device runtime
* Rust for the Pi-hosted local admin API
* External browser/admin assets may be served as static files when available
* SQLite for local storage
* Rust tests for the Pi runtime and admin API
* `just` for command orchestration
* Event-driven architecture
* Simple deployment
* Low maintenance

## Open Questions

Hardware:

* Exact button model and size
* LED control approach
* Speaker and amplifier selection
* Microphone placement
* Battery capacity and charging board
* Enclosure material and child-safety requirements

Software:

* SQLite schema
* Content package format
* Local dashboard authentication
* Update mechanism over Wi-Fi or USB-C
* Onboard AI feasibility
