use std::num::{NonZeroU16, NonZeroU32};
use std::time::Duration;

use rodio::{ChannelCount, SampleRate, Source};

pub const BUILTIN_PREFIX: &str = "builtin:";

const SOUNDBOX_SAMPLE_RATE_HZ: u32 = 44_100;
const NOTE_ATTACK_SECONDS: f32 = 0.005;
const NOTE_RELEASE_SECONDS: f32 = 0.020;
const BEDTIME_VOLUME: f32 = 0.14;
const RETRO_VOLUME: f32 = 0.07;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SoundboxCategory {
    Bedtime,
    Retro,
}

impl SoundboxCategory {
    pub fn label(&self) -> &'static str {
        match self {
            SoundboxCategory::Bedtime => "bedtime",
            SoundboxCategory::Retro => "retro",
        }
    }
}

/// A built-in melody described as a single-voice lead line.
///
/// Notes are `(midi_note, duration_units)` where one unit is an eighth note
/// at `tempo_bpm` (quarter-note beats per minute) and midi note 0 is a rest.
#[derive(Clone, Copy, Debug)]
pub struct Melody {
    pub slug: &'static str,
    pub title: &'static str,
    pub category: SoundboxCategory,
    pub tempo_bpm: u16,
    pub notes: &'static [(u8, u8)],
}

pub const CATALOG: [Melody; 6] = [
    Melody {
        slug: "twinkle-twinkle",
        title: "Twinkle Twinkle Little Star",
        category: SoundboxCategory::Bedtime,
        tempo_bpm: 80,
        notes: &[
            (60, 2),
            (60, 2),
            (67, 2),
            (67, 2),
            (69, 2),
            (69, 2),
            (67, 4),
            (65, 2),
            (65, 2),
            (64, 2),
            (64, 2),
            (62, 2),
            (62, 2),
            (60, 4),
            (67, 2),
            (67, 2),
            (65, 2),
            (65, 2),
            (64, 2),
            (64, 2),
            (62, 4),
            (67, 2),
            (67, 2),
            (65, 2),
            (65, 2),
            (64, 2),
            (64, 2),
            (62, 4),
            (60, 2),
            (60, 2),
            (67, 2),
            (67, 2),
            (69, 2),
            (69, 2),
            (67, 4),
            (65, 2),
            (65, 2),
            (64, 2),
            (64, 2),
            (62, 2),
            (62, 2),
            (60, 6),
        ],
    },
    Melody {
        slug: "brahms-lullaby",
        title: "Brahms' Lullaby",
        category: SoundboxCategory::Bedtime,
        tempo_bpm: 70,
        notes: &[
            (64, 1),
            (64, 1),
            (67, 4),
            (64, 1),
            (64, 1),
            (67, 4),
            (64, 1),
            (67, 1),
            (72, 3),
            (71, 1),
            (69, 2),
            (69, 2),
            (67, 4),
            (62, 1),
            (64, 1),
            (65, 3),
            (62, 1),
            (62, 1),
            (64, 1),
            (65, 4),
            (0, 2),
            (60, 1),
            (60, 1),
            (72, 4),
            (69, 2),
            (65, 2),
            (67, 2),
            (64, 2),
            (60, 4),
            (60, 1),
            (60, 1),
            (72, 4),
            (69, 2),
            (65, 2),
            (67, 2),
            (62, 2),
            (60, 6),
        ],
    },
    Melody {
        slug: "rock-a-bye-baby",
        title: "Rock-a-bye Baby",
        category: SoundboxCategory::Bedtime,
        tempo_bpm: 75,
        notes: &[
            (60, 1),
            (64, 1),
            (67, 2),
            (69, 1),
            (67, 3),
            (65, 1),
            (65, 1),
            (64, 2),
            (62, 1),
            (64, 3),
            (60, 1),
            (64, 1),
            (67, 2),
            (69, 1),
            (67, 3),
            (65, 1),
            (62, 1),
            (60, 5),
            (0, 1),
            (60, 1),
            (65, 1),
            (69, 2),
            (65, 1),
            (69, 3),
            (67, 1),
            (69, 1),
            (71, 2),
            (71, 1),
            (72, 3),
            (72, 1),
            (69, 1),
            (67, 2),
            (65, 1),
            (64, 3),
            (62, 1),
            (64, 1),
            (62, 2),
            (60, 6),
        ],
    },
    Melody {
        slug: "korobeiniki",
        title: "Korobeiniki (Tetris Theme)",
        category: SoundboxCategory::Retro,
        tempo_bpm: 140,
        notes: &[
            (76, 2),
            (71, 1),
            (72, 1),
            (74, 2),
            (72, 1),
            (71, 1),
            (69, 2),
            (69, 1),
            (72, 1),
            (76, 2),
            (74, 1),
            (72, 1),
            (71, 3),
            (72, 1),
            (74, 2),
            (76, 2),
            (72, 2),
            (69, 2),
            (69, 3),
            (0, 1),
            (74, 3),
            (77, 1),
            (81, 2),
            (79, 1),
            (77, 1),
            (76, 3),
            (72, 1),
            (76, 2),
            (74, 1),
            (72, 1),
            (71, 2),
            (71, 1),
            (72, 1),
            (74, 2),
            (76, 2),
            (72, 2),
            (69, 2),
            (69, 3),
            (0, 1),
            (76, 2),
            (71, 1),
            (72, 1),
            (74, 2),
            (72, 1),
            (71, 1),
            (69, 2),
            (69, 1),
            (72, 1),
            (76, 2),
            (74, 1),
            (72, 1),
            (71, 3),
            (72, 1),
            (74, 2),
            (76, 2),
            (72, 2),
            (69, 2),
            (69, 4),
        ],
    },
    Melody {
        slug: "mountain-king",
        title: "In the Hall of the Mountain King",
        category: SoundboxCategory::Retro,
        tempo_bpm: 126,
        notes: &[
            (69, 1),
            (71, 1),
            (72, 1),
            (74, 1),
            (76, 1),
            (72, 1),
            (76, 2),
            (75, 1),
            (71, 1),
            (75, 2),
            (74, 1),
            (70, 1),
            (74, 2),
            (69, 1),
            (71, 1),
            (72, 1),
            (74, 1),
            (76, 1),
            (72, 1),
            (76, 2),
            (69, 1),
            (71, 1),
            (72, 1),
            (74, 1),
            (76, 1),
            (72, 1),
            (76, 2),
            (75, 1),
            (71, 1),
            (75, 2),
            (74, 1),
            (70, 1),
            (74, 2),
            (69, 1),
            (71, 1),
            (72, 1),
            (74, 1),
            (76, 1),
            (72, 1),
            (76, 2),
            (76, 1),
            (78, 1),
            (79, 1),
            (81, 1),
            (83, 1),
            (79, 1),
            (83, 2),
            (82, 1),
            (78, 1),
            (82, 2),
            (81, 1),
            (77, 1),
            (81, 2),
            (76, 1),
            (78, 1),
            (79, 1),
            (81, 1),
            (83, 1),
            (79, 1),
            (83, 2),
            (76, 1),
            (72, 1),
            (69, 4),
        ],
    },
    Melody {
        slug: "flight-of-the-bumblebee",
        title: "Flight of the Bumblebee",
        category: SoundboxCategory::Retro,
        tempo_bpm: 260,
        notes: &[
            (76, 1),
            (75, 1),
            (74, 1),
            (73, 1),
            (72, 1),
            (71, 1),
            (70, 1),
            (69, 1),
            (68, 1),
            (67, 1),
            (66, 1),
            (65, 1),
            (64, 1),
            (65, 1),
            (66, 1),
            (67, 1),
            (68, 1),
            (67, 1),
            (66, 1),
            (65, 1),
            (64, 1),
            (63, 1),
            (62, 1),
            (61, 1),
            (60, 1),
            (61, 1),
            (62, 1),
            (63, 1),
            (64, 1),
            (65, 1),
            (66, 1),
            (67, 1),
            (68, 1),
            (69, 1),
            (68, 1),
            (67, 1),
            (68, 1),
            (69, 1),
            (68, 1),
            (67, 1),
            (66, 1),
            (67, 1),
            (66, 1),
            (65, 1),
            (66, 1),
            (67, 1),
            (66, 1),
            (65, 1),
            (64, 1),
            (65, 1),
            (66, 1),
            (67, 1),
            (68, 1),
            (69, 1),
            (70, 1),
            (71, 1),
            (72, 1),
            (73, 1),
            (74, 1),
            (75, 1),
            (76, 1),
            (75, 1),
            (74, 1),
            (73, 1),
            (76, 1),
            (75, 1),
            (74, 1),
            (73, 1),
            (72, 1),
            (71, 1),
            (70, 1),
            (69, 1),
            (70, 1),
            (71, 1),
            (72, 1),
            (73, 1),
            (74, 1),
            (73, 1),
            (72, 1),
            (71, 1),
            (72, 1),
            (71, 1),
            (70, 1),
            (69, 1),
            (68, 1),
            (67, 1),
            (66, 1),
            (65, 1),
            (64, 1),
            (66, 1),
            (68, 1),
            (70, 1),
            (72, 1),
            (74, 1),
            (76, 1),
            (78, 1),
            (81, 4),
        ],
    },
];

pub fn melody_for_slug(slug: &str) -> Option<&'static Melody> {
    CATALOG.iter().find(|melody| melody.slug == slug)
}

pub fn slug_from_audio_path(audio_path: &str) -> Option<&str> {
    audio_path.strip_prefix(BUILTIN_PREFIX)
}

#[derive(Clone, Copy, Debug)]
struct NoteSpan {
    frequency_hz: f32,
    samples: u64,
}

/// Synthesized playback of a built-in melody as a mono rodio source.
#[derive(Clone, Debug)]
pub struct MelodySource {
    spans: Vec<NoteSpan>,
    span_index: usize,
    sample_in_span: u64,
    samples_emitted: u64,
    total_samples: u64,
    volume: f32,
    retro: bool,
}

impl MelodySource {
    pub fn new(melody: &Melody) -> Self {
        let seconds_per_unit = 60.0 / f32::from(melody.tempo_bpm.max(1)) / 2.0;
        let spans: Vec<NoteSpan> = melody
            .notes
            .iter()
            .map(|(midi, units)| {
                let seconds = seconds_per_unit * f32::from(*units);
                NoteSpan {
                    frequency_hz: midi_to_frequency(*midi),
                    samples: (seconds * SOUNDBOX_SAMPLE_RATE_HZ as f32).round() as u64,
                }
            })
            .collect();
        let total_samples = spans.iter().map(|span| span.samples).sum();
        Self {
            spans,
            span_index: 0,
            sample_in_span: 0,
            samples_emitted: 0,
            total_samples,
            volume: match melody.category {
                SoundboxCategory::Bedtime => BEDTIME_VOLUME,
                SoundboxCategory::Retro => RETRO_VOLUME,
            },
            retro: matches!(melody.category, SoundboxCategory::Retro),
        }
    }
}

fn midi_to_frequency(midi: u8) -> f32 {
    if midi == 0 {
        return 0.0;
    }
    440.0 * 2f32.powf((f32::from(midi) - 69.0) / 12.0)
}

fn note_envelope(sample: u64, span_samples: u64) -> f32 {
    let attack_samples =
        ((NOTE_ATTACK_SECONDS * SOUNDBOX_SAMPLE_RATE_HZ as f32) as u64).min(span_samples / 4);
    let release_samples =
        ((NOTE_RELEASE_SECONDS * SOUNDBOX_SAMPLE_RATE_HZ as f32) as u64).min(span_samples / 4);
    let mut envelope = 1.0;
    if attack_samples > 0 && sample < attack_samples {
        envelope = sample as f32 / attack_samples as f32;
    }
    let remaining = span_samples.saturating_sub(sample);
    if release_samples > 0 && remaining < release_samples {
        envelope = envelope.min(remaining as f32 / release_samples as f32);
    }
    envelope
}

impl Iterator for MelodySource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let span = *self.spans.get(self.span_index)?;
            if self.sample_in_span >= span.samples {
                self.span_index += 1;
                self.sample_in_span = 0;
                continue;
            }

            let sample_index = self.sample_in_span;
            self.sample_in_span += 1;
            self.samples_emitted += 1;

            if span.frequency_hz <= 0.0 {
                return Some(0.0);
            }

            let t = sample_index as f32 / SOUNDBOX_SAMPLE_RATE_HZ as f32;
            let phase = (span.frequency_hz * t).fract();
            let wave = if self.retro {
                if phase < 0.5 {
                    1.0
                } else {
                    -1.0
                }
            } else {
                let sine = (std::f32::consts::TAU * phase).sin();
                let triangle = if phase < 0.5 {
                    4.0 * phase - 1.0
                } else {
                    3.0 - 4.0 * phase
                };
                0.8 * sine + 0.2 * triangle
            };
            let envelope = note_envelope(sample_index, span.samples);
            return Some(wave * envelope * self.volume);
        }
    }
}

impl Source for MelodySource {
    fn current_span_len(&self) -> Option<usize> {
        let remaining = self.total_samples.saturating_sub(self.samples_emitted);
        Some(remaining.min(usize::MAX as u64) as usize)
    }

    fn channels(&self) -> ChannelCount {
        NonZeroU16::new(1).expect("soundbox channel count is nonzero")
    }

    fn sample_rate(&self) -> SampleRate {
        NonZeroU32::new(SOUNDBOX_SAMPLE_RATE_HZ).expect("soundbox sample rate is nonzero")
    }

    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f64(
            self.total_samples as f64 / f64::from(SOUNDBOX_SAMPLE_RATE_HZ),
        ))
    }
}

/// Renders a melody as an in-memory 16-bit PCM mono WAV file for previews.
pub fn render_wav(melody: &Melody) -> Vec<u8> {
    let samples: Vec<i16> = MelodySource::new(melody)
        .map(|sample| (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
        .collect();
    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + samples.len() * 2);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SOUNDBOX_SAMPLE_RATE_HZ.to_le_bytes());
    wav.extend_from_slice(&(SOUNDBOX_SAMPLE_RATE_HZ * 2).to_le_bytes()); // byte rate
    wav.extend_from_slice(&2u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    wav
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_has_six_unique_kebab_case_entries() {
        assert_eq!(CATALOG.len(), 6);

        let mut slugs = HashSet::new();
        for melody in &CATALOG {
            assert!(slugs.insert(melody.slug), "duplicate slug {}", melody.slug);
            assert!(
                melody
                    .slug
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
                "slug {} is not kebab-case",
                melody.slug
            );
            assert!(!melody.title.is_empty());
            assert!(!melody.notes.is_empty());
            assert!(melody.tempo_bpm > 0);
        }

        let bedtime = CATALOG
            .iter()
            .filter(|melody| melody.category == SoundboxCategory::Bedtime)
            .count();
        assert_eq!(bedtime, 3);
        assert_eq!(CATALOG.len() - bedtime, 3);
    }

    #[test]
    fn resolves_catalog_slugs() {
        for melody in &CATALOG {
            assert_eq!(melody_for_slug(melody.slug).unwrap().slug, melody.slug);
        }
        assert!(melody_for_slug("unknown-melody").is_none());
    }

    #[test]
    fn strips_builtin_prefix_from_audio_paths() {
        assert_eq!(
            slug_from_audio_path("builtin:korobeiniki"),
            Some("korobeiniki")
        );
        assert_eq!(slug_from_audio_path("content/audio/music/song.mp3"), None);
    }

    #[test]
    fn melody_sources_stay_in_range_and_duration() {
        for melody in &CATALOG {
            let source = MelodySource::new(melody);
            let duration = source.total_duration().unwrap();
            assert!(
                (10.0..=60.0).contains(&duration.as_secs_f64()),
                "{} lasts {:?}",
                melody.slug,
                duration
            );

            let mut count: u64 = 0;
            for sample in MelodySource::new(melody) {
                assert!(
                    (-1.0..=1.0).contains(&sample),
                    "{} produced out-of-range sample {}",
                    melody.slug,
                    sample
                );
                count += 1;
            }
            assert!(count > 0, "{} produced no samples", melody.slug);
        }
    }

    #[test]
    fn renders_wav_with_valid_header() {
        let melody = melody_for_slug("twinkle-twinkle").unwrap();
        let wav = render_wav(melody);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(wav.len(), 44 + data_len);

        let sample_count = MelodySource::new(melody).count();
        assert_eq!(data_len, sample_count * 2);
    }
}
