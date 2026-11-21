//! The General settings behavior (docs/kanon.html `defaultPrefs`):
//! the 12h/24h time display, the sidebar's open-in-new-tab rule, and
//! the task-completion sound. The settings rows themselves live in
//! `appearance.rs`; this module is the behavior those preferences
//! drive, so every read goes through `Store::preference` and stays
//! live (the revision signal re-runs the bound display closures when a
//! setting changes).

use nsh_core::prelude::*;

use crate::domain::LocalTime;

use super::super::tabs::Route;
use super::Store;

/// The time-display preference key: `"24h"` (default) or `"12h"`.
/// Only user-facing display reads it; time inputs stay `HH:MM`.
pub(crate) const PREF_TIME_FORMAT: &str = "time_format";
/// The sidebar open-in-new-tab preference key: `"true"` or `"false"`
/// (default). When on, a sidebar 项目/知识库 press keeps the current
/// tab and opens the target in a new tab.
pub(crate) const PREF_OPEN_IN_NEW_TAB: &str = "open_in_new_tab";
/// The task-completion sound preference key: `"true"` (default) or
/// `"false"`. The tone plays through NSH's `Sound` API
/// (`nsh.json` needs `"capabilities": { "audio": true }`).
pub(crate) const PREF_COMPLETION_SOUND: &str = "completion_sound";

impl Store {
    /// Whether the 12-hour clock is on for user-facing times.
    pub(crate) fn time_format_12h(&self) -> bool {
        self.preference(PREF_TIME_FORMAT).as_deref() == Some("12h")
    }

    /// Formats a time-of-day for display. The 24-hour default renders
    /// `HH:MM`; the 12-hour choice renders the prototype's
    /// `下午 2:00` / `上午 9:05` (noon `下午 12:00`, midnight
    /// `上午 12:00`).
    pub fn format_time(&self, time: LocalTime) -> String {
        if !self.time_format_12h() {
            return time.to_string();
        }
        let (meridiem, hour) = match time.hour {
            0 => ("上午", 12),
            1..=11 => ("上午", time.hour),
            12 => ("下午", 12),
            _ => ("下午", time.hour - 12),
        };
        format!("{meridiem} {hour}:{:02}", time.minute)
    }

    /// Whether sidebar 项目/知识库 presses open a new tab.
    pub(crate) fn open_in_new_tab(&self) -> bool {
        self.preference(PREF_OPEN_IN_NEW_TAB).as_deref() == Some("true")
    }

    /// The sidebar 项目/知识库 press. The default follows the
    /// focus-or-reuse rule (docs/ui-design.md §3.2); with the
    /// open-in-new-tab preference on the current tab is kept and the
    /// target always opens in a new tab, the prototype's
    /// `nav(route, force)` behavior (docs/kanon.html `openInNewTab`).
    pub fn open_sidebar_route(&self, route: Route) {
        if !self.open_in_new_tab() {
            self.open_or_focus(route);
            return;
        }
        let today = self.now().date;
        crate::ui::motion::tab_change();
        self.strip.update(|strip| strip.open_tab(route, today));
        if window().width.0 < 680.0 {
            self.sidebar_temp_expand.set(false);
        }
    }

    /// Plays the soft completion tone when the preference allows.
    /// The WAV is synthesized in memory (the prototype's 587→880 Hz
    /// blip), so no asset ships; a missing `audio` capability or a
    /// platform without sound fails the load and stays silent.
    pub fn play_completion_sound(&self) {
        if !self.preference_enabled(PREF_COMPLETION_SOUND) {
            return;
        }
        let bytes = completion_tone_wav();
        spawn(async move {
            match load_sound(SoundSource::Bytes(bytes.into())).await {
                Ok(sound) => sound.play(1.0, 1.0),
                Err(error) => log::debug!("completion sound unavailable: {error}"),
            }
        });
    }
}

/// The completion tone as a 16-bit mono PCM WAV: a 0.13 s sine sweep
/// from 587.3 Hz to 880 Hz under an exponential envelope
/// (0.05 → 0.001), the audible part of docs/kanon.html `playDoneTone`.
fn completion_tone_wav() -> Vec<u8> {
    const SAMPLE_RATE: u32 = 22050;
    const SAMPLES: usize = SAMPLE_RATE as usize * 13 / 100; // 0.13 s.
    const F0: f64 = 587.3;
    const F1: f64 = 880.0;
    const AMP0: f64 = 0.05;
    const AMP1: f64 = 0.001;

    let data_len = (SAMPLES * 2) as u32;
    let mut bytes = Vec::with_capacity(44 + SAMPLES * 2);
    fn push_u32(bytes: &mut Vec<u8>, value: u32) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    fn push_u16(bytes: &mut Vec<u8>, value: u16) {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(b"RIFF");
    push_u32(&mut bytes, 36 + data_len);
    bytes.extend_from_slice(b"WAVE");
    bytes.extend_from_slice(b"fmt ");
    push_u32(&mut bytes, 16);
    push_u16(&mut bytes, 1); // PCM.
    push_u16(&mut bytes, 1); // Mono.
    push_u32(&mut bytes, SAMPLE_RATE);
    push_u32(&mut bytes, SAMPLE_RATE * 2); // Byte rate.
    push_u16(&mut bytes, 2); // Block align.
    push_u16(&mut bytes, 16); // Bits per sample.
    bytes.extend_from_slice(b"data");
    push_u32(&mut bytes, data_len);

    let sweep = (F1 / F0).ln();
    let decay = (AMP1 / AMP0).ln();
    let mut phase = 0.0;
    for i in 0..SAMPLES {
        let t = i as f64 / SAMPLES as f64;
        phase += std::f64::consts::TAU * F0 * (sweep * t).exp() / SAMPLE_RATE as f64;
        let sample = phase.sin() * AMP0 * (decay * t).exp();
        let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f64) as i16;
        bytes.extend_from_slice(&pcm.to_le_bytes());
    }
    bytes
}
