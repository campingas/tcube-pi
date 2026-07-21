//! Physical button input for the Raspberry Pi backend.
//!
//! The platform-neutral pieces (pin map parsing, debouncing, and the input
//! pipeline that feeds the Pomodoro chord recognizer) compile and are tested
//! on every platform. The rppal-backed listener that claims real GPIO lines
//! is gated behind the `pi-gpio` feature and only builds on Linux.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Result};

use crate::hardware::button::{
    ButtonGestureEvent, ButtonGestureEventKind, PomodoroGesture, PomodoroGestureRecognizer,
    PomodoroTrigger,
};

/// Default BCM lines per docs/hardware/hardware-assembly.md: btn1=17 (red),
/// btn2=27 (green), btn3=22 (yellow), btn4=5 (blue), btn5=6 (white).
pub(crate) const DEFAULT_BUTTON_GPIOS: &str = "17,27,22,5,6";
pub(crate) const DEBOUNCE_WINDOW: Duration = Duration::from_millis(30);
const MAX_BUTTONS: usize = 5;

/// BCM line assignment per button; position in the spec string is the button id.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PinMap {
    entries: Vec<(u8, u8)>,
}

impl PinMap {
    pub(crate) fn parse(spec: &str) -> Result<Self> {
        let mut entries: Vec<(u8, u8)> = Vec::new();
        for (index, raw) in spec.split(',').enumerate() {
            let raw = raw.trim();
            let bcm: u8 = raw.parse().map_err(|_| {
                anyhow::anyhow!("invalid BCM GPIO number {:?} in button map {:?}", raw, spec)
            })?;
            if entries.iter().any(|(_, existing)| *existing == bcm) {
                bail!("duplicate BCM GPIO {} in button map {:?}", bcm, spec);
            }
            entries.push((index as u8 + 1, bcm));
        }
        if entries.is_empty() {
            bail!("button map {:?} assigns no GPIO lines", spec);
        }
        if entries.len() > MAX_BUTTONS {
            bail!(
                "button map {:?} assigns {} lines; the device has at most {} buttons",
                spec,
                entries.len(),
                MAX_BUTTONS
            );
        }
        Ok(Self { entries })
    }

    /// Pairs of (button id, BCM line).
    pub(crate) fn entries(&self) -> &[(u8, u8)] {
        &self.entries
    }
}

impl Default for PinMap {
    fn default() -> Self {
        Self::parse(DEFAULT_BUTTON_GPIOS).expect("default pin map is valid")
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct DebounceState {
    pressed: bool,
    last_change_at: Option<Duration>,
}

/// Turns raw (possibly bouncing) edges into stable press/release transitions.
/// The first edge is accepted immediately; opposite edges inside the window
/// are ignored until sampled-level reconciliation confirms the final state.
#[derive(Clone, Debug)]
pub(crate) struct Debouncer {
    window: Duration,
    states: HashMap<u8, DebounceState>,
}

impl Debouncer {
    pub(crate) fn new(window: Duration) -> Self {
        Self {
            window,
            states: HashMap::new(),
        }
    }

    /// Returns the new stable pressed level, or None when the edge is bounce
    /// or matches the current stable level.
    pub(crate) fn edge(&mut self, button_id: u8, pressed: bool, at: Duration) -> Option<bool> {
        let state = self.states.entry(button_id).or_default();
        if state.pressed == pressed {
            return None;
        }
        if let Some(last) = state.last_change_at {
            if at.saturating_sub(last) < self.window {
                return None;
            }
        }
        state.pressed = pressed;
        state.last_change_at = Some(at);
        Some(pressed)
    }

    fn seed(&mut self, button_id: u8, pressed: bool, at: Duration) {
        self.states.insert(
            button_id,
            DebounceState {
                pressed,
                last_change_at: Some(at),
            },
        );
    }

    fn reconcile(&mut self, button_id: u8, pressed: bool, at: Duration) -> Option<bool> {
        self.edge(button_id, pressed, at)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PipelineEvent {
    Press(u8),
    PomodoroShortcut(PomodoroTrigger),
}

/// Consumes debounced GPIO edges plus periodic ticks and produces the events
/// the device loop understands. A press is emitted immediately on the down
/// edge to keep the button response path fast; a chord attempt therefore
/// briefly plays button audio, which the Pomodoro routine's leading stop cuts.
#[derive(Clone, Debug)]
pub(crate) struct InputPipeline {
    debouncer: Debouncer,
    recognizer: PomodoroGestureRecognizer,
}

impl InputPipeline {
    pub(crate) fn new() -> Self {
        Self {
            debouncer: Debouncer::new(DEBOUNCE_WINDOW),
            recognizer: PomodoroGestureRecognizer::new(),
        }
    }

    pub(crate) fn handle_edge(
        &mut self,
        button_id: u8,
        pressed: bool,
        at: Duration,
    ) -> Vec<PipelineEvent> {
        let mut events = Vec::new();
        let Some(stable_pressed) = self.debouncer.edge(button_id, pressed, at) else {
            return events;
        };
        self.apply_level(button_id, stable_pressed, at, true, &mut events);
        events
    }

    pub(crate) fn seed_level(&mut self, button_id: u8, pressed: bool, at: Duration) {
        self.debouncer.seed(button_id, pressed, at);
    }

    pub(crate) fn reconcile_level(
        &mut self,
        button_id: u8,
        pressed: bool,
        at: Duration,
    ) -> Vec<PipelineEvent> {
        let mut events = Vec::new();
        if let Some(authoritative_pressed) = self.debouncer.reconcile(button_id, pressed, at) {
            self.apply_level(button_id, authoritative_pressed, at, false, &mut events);
        }
        events
    }

    pub(crate) fn handle_tick(&mut self, at: Duration) -> Option<PipelineEvent> {
        self.recognizer
            .handle(ButtonGestureEvent {
                button_id: 0,
                kind: ButtonGestureEventKind::Tick,
                at,
            })
            .map(|PomodoroGesture::HoldCompleted(trigger)| PipelineEvent::PomodoroShortcut(trigger))
    }

    fn apply_level(
        &mut self,
        button_id: u8,
        pressed: bool,
        at: Duration,
        emit_press: bool,
        events: &mut Vec<PipelineEvent>,
    ) {
        let kind = if pressed {
            if emit_press {
                events.push(PipelineEvent::Press(button_id));
            }
            ButtonGestureEventKind::Down
        } else {
            ButtonGestureEventKind::Up
        };
        if let Some(PomodoroGesture::HoldCompleted(trigger)) =
            self.recognizer.handle(ButtonGestureEvent {
                button_id,
                kind,
                at,
            })
        {
            events.push(PipelineEvent::PomodoroShortcut(trigger));
        }
    }
}

#[cfg(all(feature = "pi-gpio", target_os = "linux"))]
mod listener {
    use std::sync::mpsc;
    use std::thread;
    use std::time::{Duration, Instant};

    use anyhow::{Context, Result};
    use rppal::gpio::{Gpio, InputPin, Trigger};

    use super::{InputPipeline, PinMap, PipelineEvent, DEBOUNCE_WINDOW};

    /// Poll timeout doubling as the tick interval that drives the Pomodoro
    /// hold detection while no edges arrive.
    const POLL_TIMEOUT: Duration = Duration::from_millis(100);

    /// Claims the configured GPIO lines with internal pull-downs (the MKE-M02
    /// modules drive SIG high on press) and feeds edge events through the
    /// input pipeline on a dedicated thread.
    pub(crate) struct GpioListener {
        receiver: mpsc::Receiver<PipelineEvent>,
    }

    impl GpioListener {
        pub(crate) fn start(pin_map: PinMap) -> Result<Self> {
            let gpio = Gpio::new().context("failed to access the GPIO peripheral")?;
            let mut pins: Vec<(u8, InputPin)> = Vec::new();
            for (button_id, bcm) in pin_map.entries() {
                let mut pin = gpio
                    .get(*bcm)
                    .with_context(|| {
                        format!("failed to claim BCM GPIO {bcm} for button {button_id}")
                    })?
                    .into_input_pulldown();
                pin.set_interrupt(Trigger::Both, Some(DEBOUNCE_WINDOW))
                    .with_context(|| format!("failed to arm interrupt on BCM GPIO {bcm}"))?;
                println!("button {button_id} listening on BCM GPIO {bcm}");
                pins.push((*button_id, pin));
            }

            let (sender, receiver) = mpsc::channel();
            thread::Builder::new()
                .name("gpio-listener".to_string())
                .spawn(move || listen(gpio, pins, sender))
                .context("failed to spawn GPIO listener thread")?;
            Ok(Self { receiver })
        }

        /// Blocks until the next pipeline event. Errors when the listener
        /// thread died so the device loop can surface the failure and let
        /// systemd restart the service.
        pub(crate) fn recv(&self) -> Result<PipelineEvent> {
            self.receiver.recv().context("GPIO listener thread stopped")
        }

        pub(crate) fn recv_timeout(
            &self,
            timeout: Duration,
        ) -> std::result::Result<PipelineEvent, mpsc::RecvTimeoutError> {
            self.receiver.recv_timeout(timeout)
        }
    }

    fn listen(gpio: Gpio, pins: Vec<(u8, InputPin)>, sender: mpsc::Sender<PipelineEvent>) {
        let mut pipeline = InputPipeline::new();
        // Edges and ticks share one clock so the recognizer timing holds.
        let epoch = Instant::now();
        for (button_id, pin) in &pins {
            pipeline.seed_level(*button_id, pin.is_high(), epoch.elapsed());
        }
        let pin_refs: Vec<&InputPin> = pins.iter().map(|(_, pin)| pin).collect();
        loop {
            let polled = gpio.poll_interrupts(&pin_refs, false, Some(POLL_TIMEOUT));
            let at = epoch.elapsed();
            let mut events = match polled {
                Ok(Some((pin, event))) => pins
                    .iter()
                    .find(|(_, candidate)| candidate.pin() == pin.pin())
                    .map(|(button_id, _)| *button_id)
                    .map(|button_id| {
                        let pressed = event.trigger == Trigger::RisingEdge;
                        pipeline.handle_edge(button_id, pressed, at)
                    })
                    .unwrap_or_default(),
                Ok(None) => Vec::new(),
                Err(error) => {
                    eprintln!("GPIO interrupt poll failed: {error}");
                    return;
                }
            };
            for (button_id, pin) in &pins {
                events.extend(pipeline.reconcile_level(*button_id, pin.is_high(), at));
            }
            events.extend(pipeline.handle_tick(at));
            for event in events {
                if sender.send(event).is_err() {
                    return;
                }
            }
        }
    }
}

#[cfg(all(feature = "pi-gpio", target_os = "linux"))]
pub(crate) use listener::GpioListener;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hardware::button::POMODORO_HOLD_DURATION;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn pin_map_parses_full_default() {
        let map = PinMap::parse("17,27,22,5,6").expect("default map parses");
        assert_eq!(map.entries(), &[(1, 17), (2, 27), (3, 22), (4, 5), (5, 6)]);
        assert_eq!(map, PinMap::default());
    }

    #[test]
    fn pin_map_allows_partial_bench_map() {
        let map = PinMap::parse("17").expect("single-button map parses");
        assert_eq!(map.entries(), &[(1, 17)]);
    }

    #[test]
    fn pin_map_accepts_whitespace() {
        let map = PinMap::parse(" 17, 27 ").expect("spaced map parses");
        assert_eq!(map.entries(), &[(1, 17), (2, 27)]);
    }

    #[test]
    fn pin_map_rejects_duplicates() {
        assert!(PinMap::parse("17,17").is_err());
    }

    #[test]
    fn pin_map_rejects_junk() {
        assert!(PinMap::parse("").is_err());
        assert!(PinMap::parse("17,abc").is_err());
        assert!(PinMap::parse("17,").is_err());
        assert!(PinMap::parse("-4").is_err());
    }

    #[test]
    fn pin_map_rejects_too_many_lines() {
        assert!(PinMap::parse("1,2,3,4,5,6").is_err());
    }

    #[test]
    fn debouncer_collapses_bounce_burst_to_one_press() {
        let mut debouncer = Debouncer::new(ms(30));
        assert_eq!(debouncer.edge(1, true, ms(0)), Some(true));
        assert_eq!(debouncer.edge(1, false, ms(3)), None);
        assert_eq!(debouncer.edge(1, true, ms(6)), None);
        assert_eq!(debouncer.edge(1, false, ms(9)), None);
        assert_eq!(debouncer.edge(1, true, ms(12)), None);
    }

    #[test]
    fn debouncer_collapses_release_bounce() {
        let mut debouncer = Debouncer::new(ms(30));
        assert_eq!(debouncer.edge(1, true, ms(0)), Some(true));
        assert_eq!(debouncer.edge(1, false, ms(100)), Some(false));
        assert_eq!(debouncer.edge(1, true, ms(103)), None);
        assert_eq!(debouncer.edge(1, false, ms(106)), None);
    }

    #[test]
    fn debouncer_passes_spaced_presses() {
        let mut debouncer = Debouncer::new(ms(30));
        assert_eq!(debouncer.edge(1, true, ms(0)), Some(true));
        assert_eq!(debouncer.edge(1, false, ms(200)), Some(false));
        assert_eq!(debouncer.edge(1, true, ms(400)), Some(true));
    }

    #[test]
    fn debouncer_tracks_buttons_independently() {
        let mut debouncer = Debouncer::new(ms(30));
        assert_eq!(debouncer.edge(1, true, ms(0)), Some(true));
        assert_eq!(debouncer.edge(2, true, ms(5)), Some(true));
    }

    #[test]
    fn pipeline_emits_press_on_down_edge() {
        let mut pipeline = InputPipeline::new();
        assert_eq!(
            pipeline.handle_edge(3, true, ms(0)),
            vec![PipelineEvent::Press(3)]
        );
        assert_eq!(pipeline.handle_edge(3, false, ms(100)), vec![]);
    }

    #[test]
    fn pipeline_completes_pomodoro_chord_once() {
        let mut pipeline = InputPipeline::new();
        let mut at = ms(100);
        for button_id in [1, 2] {
            let events = pipeline.handle_edge(button_id, true, at);
            assert_eq!(events, vec![PipelineEvent::Press(button_id)]);
            at += ms(50);
        }

        let hold_deadline = at + POMODORO_HOLD_DURATION;
        while at < hold_deadline {
            assert_eq!(pipeline.handle_tick(at), None);
            at += ms(100);
        }
        assert_eq!(
            pipeline.handle_tick(at),
            Some(PipelineEvent::PomodoroShortcut(PomodoroTrigger {
                buttons: [1, 2],
                assembly_ms: 50,
            }))
        );
        assert_eq!(pipeline.handle_tick(at + ms(100)), None);
    }

    #[test]
    fn pipeline_resets_chord_when_combo_button_releases() {
        let mut pipeline = InputPipeline::new();
        pipeline.handle_edge(1, true, ms(100));
        pipeline.handle_edge(2, true, ms(150));
        pipeline.handle_edge(2, false, ms(1000));

        let mut tick_at = ms(1000);
        let deadline = ms(1000) + POMODORO_HOLD_DURATION + ms(500);
        while tick_at < deadline {
            assert_eq!(pipeline.handle_tick(tick_at), None);
            tick_at += ms(100);
        }
    }

    #[test]
    fn pipeline_rejects_slow_pair() {
        let mut pipeline = InputPipeline::new();
        pipeline.handle_edge(1, true, ms(100));
        pipeline.handle_edge(2, true, ms(601));
        assert_eq!(pipeline.handle_tick(ms(4000)), None);
    }

    #[test]
    fn startup_high_is_ineligible_until_released_and_pressed_again() {
        let mut pipeline = InputPipeline::new();
        pipeline.seed_level(1, true, ms(100));
        assert_eq!(
            pipeline.handle_edge(2, true, ms(400)),
            vec![PipelineEvent::Press(2)]
        );
        assert_eq!(pipeline.handle_tick(ms(3400)), None);

        assert_eq!(pipeline.reconcile_level(1, false, ms(3500)), vec![]);
        assert_eq!(pipeline.handle_edge(2, false, ms(3500)), vec![]);
        assert_eq!(
            pipeline.handle_edge(1, true, ms(4000)),
            vec![PipelineEvent::Press(1)]
        );
        assert_eq!(
            pipeline.handle_edge(2, true, ms(4100)),
            vec![PipelineEvent::Press(2)]
        );
        assert!(matches!(
            pipeline.handle_tick(ms(7100)),
            Some(PipelineEvent::PomodoroShortcut(_))
        ));
    }

    #[test]
    fn sampled_level_reconciles_a_suppressed_release() {
        let mut pipeline = InputPipeline::new();
        assert_eq!(
            pipeline.handle_edge(1, true, ms(100)),
            vec![PipelineEvent::Press(1)]
        );
        assert_eq!(pipeline.handle_edge(1, false, ms(110)), vec![]);
        assert_eq!(pipeline.reconcile_level(1, false, ms(110)), vec![]);
        assert_eq!(pipeline.reconcile_level(1, false, ms(140)), vec![]);
        assert_eq!(
            pipeline.handle_edge(2, true, ms(500)),
            vec![PipelineEvent::Press(2)]
        );
        assert_eq!(pipeline.handle_tick(ms(4000)), None);
    }

    #[test]
    fn sampled_rising_level_updates_gesture_without_duplicate_press() {
        let mut pipeline = InputPipeline::new();
        pipeline.seed_level(1, false, ms(0));

        assert_eq!(pipeline.reconcile_level(1, true, ms(100)), vec![]);
        assert_eq!(pipeline.handle_tick(ms(4000)), None);
    }
}
