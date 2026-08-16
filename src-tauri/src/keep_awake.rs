//! Holding the machine awake with the lid shut, and giving it back.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// What arms the hold.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Trigger {
    /// Nothing is held, and no admin password is ever asked for. The default,
    /// because this changes a system-wide power setting.
    #[default]
    Off,
    /// Held while an agent session transcript has been written recently. See
    /// [`crate::agent_activity`]: the transcript moves even when the process is
    /// idle waiting on the network, which is precisely when a CPU heuristic
    /// fails.
    AgentActive,
    /// Held for as long as the app runs, still subject to both guards. The
    /// escape hatch for agents running inside a GUI app, where there is no
    /// honest signal to detect.
    Always,
}

/// The lower bound on each limit is the value below which the setting stops
/// meaning anything: a zero cap releases the instant it holds, a zero window
/// can never see an agent. The upper bounds keep a hand-edited file from
/// disabling a guard by making it unreachable.
const IDLE_WINDOW_RANGE: std::ops::RangeInclusive<u32> = 1..=60;
const BATTERY_FLOOR_RANGE: std::ops::RangeInclusive<u8> = 0..=95;
const MAX_HOLD_RANGE: std::ops::RangeInclusive<u32> = 5..=1440;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    #[serde(default)]
    pub trigger: Trigger,
    /// How long a transcript may go untouched before its agent counts as idle.
    /// Generous on purpose: a single long tool call writes nothing while it
    /// runs, and the cost of guessing "idle" too early is the machine sleeping
    /// mid-task, which is the whole bug.
    #[serde(default = "default_idle_window")]
    pub idle_window_minutes: u32,
    /// Below this charge, on battery, the hold is dropped even mid-task.
    #[serde(default = "default_battery_floor")]
    pub battery_floor_percent: u8,
    /// The longest a single hold may run. With the lid shut nothing can be
    /// reported to the user, so this is a silent protection and is deliberately
    /// conservative rather than generous.
    #[serde(default = "default_max_hold")]
    pub max_hold_minutes: u32,
}

fn default_idle_window() -> u32 {
    5
}
fn default_battery_floor() -> u8 {
    30
}
fn default_max_hold() -> u32 {
    240
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: Trigger::Off,
            idle_window_minutes: default_idle_window(),
            battery_floor_percent: default_battery_floor(),
            max_hold_minutes: default_max_hold(),
        }
    }
}

impl Settings {
    /// Every value forced into a range where it still means what it says.
    ///
    /// Applied on load as well as on save: the file is plain JSON in the user's
    /// own folder and nothing stops it being edited by hand, so validating only
    /// at the command boundary would leave the guards defeatable with a text
    /// editor.
    pub fn clamped(self) -> Self {
        Self {
            trigger: self.trigger,
            idle_window_minutes: self
                .idle_window_minutes
                .clamp(*IDLE_WINDOW_RANGE.start(), *IDLE_WINDOW_RANGE.end()),
            battery_floor_percent: self
                .battery_floor_percent
                .clamp(*BATTERY_FLOOR_RANGE.start(), *BATTERY_FLOOR_RANGE.end()),
            max_hold_minutes: self
                .max_hold_minutes
                .clamp(*MAX_HOLD_RANGE.start(), *MAX_HOLD_RANGE.end()),
        }
    }

    /// Falls back to the defaults for anything unreadable. Deliberately not an
    /// error: the defaults ask for no password and hold nothing, so landing
    /// there costs a user their preference and never their machine.
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or_default()
            .clamped()
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, serde_json::to_vec_pretty(&self.clamped())?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_feature_is_off_until_someone_asks_for_it() {
        // It changes a system-wide power setting and needs an admin password.
        // Nothing about that may happen because the app was installed.
        assert_eq!(Settings::default().trigger, Trigger::Off);
    }

    #[test]
    fn a_hand_edited_file_cannot_produce_a_setting_that_defeats_a_guard() {
        // A zero cap would release the instant it held; a 100% floor would
        // never hold at all; a zero idle window would never see an agent. All
        // three read as "the feature is broken", so they are clamped rather
        // than honoured.
        let wild = Settings {
            trigger: Trigger::Always,
            idle_window_minutes: 0,
            battery_floor_percent: 100,
            max_hold_minutes: 0,
        };
        let sane = wild.clamped();
        assert_eq!(sane.idle_window_minutes, 1);
        assert_eq!(sane.battery_floor_percent, 95);
        assert_eq!(sane.max_hold_minutes, 5);

        let huge = Settings {
            trigger: Trigger::Always,
            idle_window_minutes: 9999,
            battery_floor_percent: 200,
            max_hold_minutes: 99999,
        };
        let sane = huge.clamped();
        assert_eq!(sane.idle_window_minutes, 60);
        assert_eq!(sane.battery_floor_percent, 95);
        assert_eq!(sane.max_hold_minutes, 1440);
    }

    #[test]
    fn settings_survive_a_round_trip_to_disk() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("keep-awake.json");
        let written = Settings {
            trigger: Trigger::AgentActive,
            idle_window_minutes: 7,
            battery_floor_percent: 40,
            max_hold_minutes: 120,
        };
        written.save(&file).unwrap();
        assert_eq!(Settings::load(&file), written);
    }

    #[test]
    fn a_missing_or_corrupt_file_reads_as_the_defaults() {
        // Never a panic and never a hold: an unreadable preference must land on
        // the side that asks for no password and touches no power setting.
        let d = tempfile::tempdir().unwrap();
        assert_eq!(
            Settings::load(&d.path().join("absent.json")),
            Settings::default()
        );

        let broken = d.path().join("broken.json");
        std::fs::write(&broken, b"{ not json").unwrap();
        assert_eq!(Settings::load(&broken), Settings::default());
    }

    #[test]
    fn a_file_written_by_an_older_version_keeps_the_fields_it_had() {
        // `serde(default)` per field, so adding a fifth setting later does not
        // silently reset the four a user already chose.
        let d = tempfile::tempdir().unwrap();
        let partial = d.path().join("partial.json");
        std::fs::write(&partial, br#"{"trigger":"always"}"#).unwrap();

        let loaded = Settings::load(&partial);
        assert_eq!(loaded.trigger, Trigger::Always);
        assert_eq!(
            loaded.max_hold_minutes,
            Settings::default().max_hold_minutes
        );
    }
}
