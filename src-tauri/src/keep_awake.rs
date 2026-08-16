//! Holding the machine awake with the lid shut, and giving it back.

use crate::platform::Power;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

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

/// What the app is doing about sleep right now. The window draws this verbatim,
/// so every variant has to be something a person can act on — "paused" without
/// a reason would leave a user who trusted the feature unable to tell why their
/// machine went to sleep.
#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "kebab-case")]
pub enum Phase {
    /// The trigger is `Off`. Nothing is watched and nothing is held.
    Off,
    /// Armed, but nothing is asking for a hold.
    Idle,
    /// Holding the machine awake right now.
    Holding,
    /// Something is asking, and the battery guard said no.
    PausedLowBattery,
    /// Something is asking, and this hold has run its full duration.
    PausedCapReached,
}

impl Phase {
    /// Whether this phase wants the flag file to exist.
    pub fn holds(self) -> bool {
        matches!(self, Phase::Holding)
    }
}

/// Everything [`decide`] is allowed to look at. A struct rather than four
/// arguments so that adding a guard later is a compile error at every call site
/// rather than a silently reordered pair of booleans.
pub struct Inputs {
    pub agent_active: bool,
    pub power: Power,
    /// How long the current hold has run. Zero when not holding.
    pub held_for: Duration,
    /// Whether the cap has already fired for this hold. Latched by the caller
    /// and cleared only when the trigger stops asking.
    pub capped: bool,
}

/// The whole policy, as a pure function.
///
/// Pure on purpose: this is the code that decides whether a machine in someone's
/// bag stays awake, and it must be exercisable at a flat battery, at a four-hour
/// cap and on a desktop with no battery at all without any of those being true
/// of the machine running the tests.
pub fn decide(settings: &Settings, inputs: &Inputs) -> Phase {
    let asking = match settings.trigger {
        Trigger::Off => return Phase::Off,
        Trigger::Always => true,
        Trigger::AgentActive => inputs.agent_active,
    };
    if !asking {
        return Phase::Idle;
    }

    // The cap is checked before the battery because it is the guard that cannot
    // be recovered from: plugging in lifts a battery pause, and nothing lifts a
    // cap until the agent goes quiet. Reporting the recoverable one first would
    // send the user to find a charger for no reason.
    let cap = Duration::from_secs(u64::from(settings.max_hold_minutes) * 60);
    if inputs.capped || inputs.held_for >= cap {
        return Phase::PausedCapReached;
    }

    // Fails open, and deliberately: `None` is a desktop or a reading that did
    // not come back, neither of which is a flat battery. The cap above still
    // bounds every hold, so there is no unbounded case here.
    let flat = !inputs.power.external
        && inputs
            .power
            .percent
            .is_some_and(|percent| percent < settings.battery_floor_percent);
    if flat {
        return Phase::PausedLowBattery;
    }

    Phase::Holding
}

/// What the previous run left behind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recovery {
    /// The `SleepDisabled` value from before a run that died holding it. Handed
    /// to the next watchdog, which resets the setting to it before doing
    /// anything else.
    pub reclaimed_prior: Option<u8>,
    /// Whether the machine may be unable to sleep right now because of us.
    /// Drives a banner, and it is the difference between a user who can fix
    /// this in one click and one who has to find out that `pmset` exists.
    pub stranded: bool,
}

/// Clears whatever the last run left, and reports what has to be put back.
///
/// Runs once, in `setup`, before the watcher thread starts — so the first sweep
/// makes its decision from a clean slate rather than inheriting a hold that
/// nothing has yet checked the battery against.
pub fn recover_at_startup(data_root: &Path) -> Recovery {
    // Unconditionally, and first. A flag surviving a crash would have the app
    // asking to hold from the moment it launched.
    let _ = std::fs::remove_file(crate::paths::keep_awake_flag(data_root));

    let breadcrumb = crate::paths::keep_awake_breadcrumb(data_root);
    let Ok(raw) = std::fs::read_to_string(&breadcrumb) else {
        return Recovery {
            reclaimed_prior: None,
            stranded: false,
        };
    };
    let _ = std::fs::remove_file(&breadcrumb);

    // A breadcrumb we cannot read still means a run died owning the setting. The
    // only question is which way to guess, and the two mistakes are not
    // symmetric: guessing "ours" costs a user a `disablesleep` they can set
    // again in one command, while guessing "theirs" costs them a machine that
    // never sleeps for a reason they have no way to connect to this app.
    let prior = match raw.trim().strip_prefix("prior=") {
        Some("1") => 1,
        _ => 0,
    };
    Recovery {
        reclaimed_prior: Some(prior),
        // Only a `prior` of 0 strands anyone. At 1 the machine was already not
        // sleeping before this app ran, and putting it back is exactly right.
        stranded: prior == 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(trigger: Trigger) -> Settings {
        Settings {
            trigger,
            ..Settings::default()
        }
    }

    fn on_battery(percent: u8) -> Power {
        Power {
            percent: Some(percent),
            external: false,
        }
    }

    fn working() -> Inputs {
        Inputs {
            agent_active: true,
            power: on_battery(80),
            held_for: Duration::ZERO,
            capped: false,
        }
    }

    #[test]
    fn nothing_is_held_while_the_trigger_is_off() {
        // Even with an agent plainly working and a full battery. `Off` is the
        // default, and it must be unconditional or the default is a lie.
        assert_eq!(decide(&settings(Trigger::Off), &working()), Phase::Off);
    }

    #[test]
    fn a_working_agent_is_held_and_an_idle_one_is_not() {
        let s = settings(Trigger::AgentActive);
        assert_eq!(decide(&s, &working()), Phase::Holding);
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    agent_active: false,
                    ..working()
                }
            ),
            Phase::Idle
        );
    }

    #[test]
    fn the_always_trigger_ignores_whether_an_agent_is_working() {
        // The escape hatch for agents inside a GUI app, where there is no
        // signal to detect. It is still subject to both guards.
        let s = settings(Trigger::Always);
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    agent_active: false,
                    ..working()
                }
            ),
            Phase::Holding
        );
    }

    #[test]
    fn a_battery_below_the_floor_drops_the_hold_mid_task() {
        // The point of the guard: the agent is still working and we stop
        // anyway, because a closed lid on a flat battery loses the work either
        // way and loses the machine as well.
        let s = settings(Trigger::AgentActive);
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    power: on_battery(29),
                    ..working()
                }
            ),
            Phase::PausedLowBattery
        );
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    power: on_battery(30),
                    ..working()
                }
            ),
            Phase::Holding,
            "the floor itself is still enough to hold"
        );
    }

    #[test]
    fn a_machine_on_external_power_is_never_paused_for_its_battery() {
        // It cannot run flat. Pausing a plugged-in laptop at 5% while it charges
        // would be the guard defeating the feature for no benefit.
        let s = settings(Trigger::AgentActive);
        let plugged = Power {
            percent: Some(5),
            external: true,
        };
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    power: plugged,
                    ..working()
                }
            ),
            Phase::Holding
        );
    }

    #[test]
    fn a_machine_with_no_battery_reading_is_never_paused_for_it() {
        // A desktop has no battery, and a failed read is not evidence of a flat
        // one. Failing open here is safe because the duration cap still bounds
        // every hold.
        let s = settings(Trigger::AgentActive);
        let unknown = Power {
            percent: None,
            external: false,
        };
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    power: unknown,
                    ..working()
                }
            ),
            Phase::Holding
        );
    }

    #[test]
    fn a_hold_that_has_run_its_full_duration_stops() {
        let s = settings(Trigger::AgentActive);
        let cap = Duration::from_secs(u64::from(s.max_hold_minutes) * 60);
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    held_for: cap,
                    ..working()
                }
            ),
            Phase::PausedCapReached
        );
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    held_for: cap - Duration::from_secs(1),
                    ..working()
                }
            ),
            Phase::Holding
        );
    }

    #[test]
    fn a_cap_that_has_fired_stays_fired_while_the_trigger_keeps_asking() {
        // Without the latch the clock resets the moment the hold drops, and the
        // cap becomes a stutter rather than a stop.
        let s = settings(Trigger::AgentActive);
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    capped: true,
                    held_for: Duration::ZERO,
                    ..working()
                }
            ),
            Phase::PausedCapReached
        );
    }

    #[test]
    fn the_cap_is_reported_ahead_of_a_low_battery() {
        // Both can be true at once and the window shows one. Naming the battery
        // would invite the user to plug in and expect it to resume, which the
        // cap will not do.
        let s = settings(Trigger::AgentActive);
        let both = Inputs {
            capped: true,
            power: on_battery(5),
            ..working()
        };
        assert_eq!(decide(&s, &both), Phase::PausedCapReached);
    }

    #[test]
    fn a_trigger_that_stops_asking_reports_idle_rather_than_a_guard() {
        // Honesty: "paused, low battery" while nothing wants a hold would have
        // the user plug in to fix a problem that does not exist.
        let s = settings(Trigger::AgentActive);
        let quiet = Inputs {
            agent_active: false,
            power: on_battery(2),
            capped: true,
            ..working()
        };
        assert_eq!(decide(&s, &quiet), Phase::Idle);
    }

    #[test]
    fn a_clean_start_has_nothing_to_reclaim() {
        let d = tempfile::tempdir().unwrap();
        let found = recover_at_startup(d.path());
        assert_eq!(found.reclaimed_prior, None);
        assert!(!found.stranded);
    }

    #[test]
    fn a_stale_flag_is_cleared_before_the_first_sweep() {
        // Otherwise a run that crashed while holding would come back already
        // asking to hold, before anything had looked at the battery.
        let d = tempfile::tempdir().unwrap();
        let flag = crate::paths::keep_awake_flag(d.path());
        std::fs::write(&flag, b"").unwrap();

        recover_at_startup(d.path());

        assert!(!flag.exists(), "a stale flag must not survive startup");
    }

    #[test]
    fn a_breadcrumb_left_by_a_run_that_died_holding_is_reclaimed() {
        // The case the whole breadcrumb exists for: `disablesleep` is persistent
        // and survives reboot, so without this the machine can never sleep again
        // and the only way out is a command the user has no reason to know.
        let d = tempfile::tempdir().unwrap();
        std::fs::write(crate::paths::keep_awake_breadcrumb(d.path()), b"prior=0\n").unwrap();

        let found = recover_at_startup(d.path());

        assert_eq!(found.reclaimed_prior, Some(0));
        assert!(
            found.stranded,
            "the user must be told their Mac may not sleep"
        );
    }

    #[test]
    fn a_breadcrumb_recording_the_users_own_setting_is_reclaimed_without_alarming_them() {
        // `prior=1` means sleep was already disabled before this app touched
        // anything. Nothing is stranded — restoring to 1 is restoring it — so
        // there is nothing to warn about.
        let d = tempfile::tempdir().unwrap();
        std::fs::write(crate::paths::keep_awake_breadcrumb(d.path()), b"prior=1\n").unwrap();

        let found = recover_at_startup(d.path());

        assert_eq!(found.reclaimed_prior, Some(1));
        assert!(!found.stranded);
    }

    #[test]
    fn an_unreadable_breadcrumb_errs_toward_letting_the_machine_sleep() {
        // Two ways to be wrong. Assuming we took it costs a user their manual
        // `disablesleep`, which they can set again in one command. Assuming we
        // did not costs them a machine that can never sleep, and they have no
        // reason to suspect this app. Fail toward sleeping.
        let d = tempfile::tempdir().unwrap();
        std::fs::write(crate::paths::keep_awake_breadcrumb(d.path()), b"garbage").unwrap();

        assert_eq!(recover_at_startup(d.path()).reclaimed_prior, Some(0));
    }

    #[test]
    fn the_breadcrumb_is_removed_so_the_next_run_does_not_reclaim_twice() {
        let d = tempfile::tempdir().unwrap();
        let crumb = crate::paths::keep_awake_breadcrumb(d.path());
        std::fs::write(&crumb, b"prior=0\n").unwrap();

        recover_at_startup(d.path());

        assert!(!crumb.exists());
        assert_eq!(recover_at_startup(d.path()).reclaimed_prior, None);
    }

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
