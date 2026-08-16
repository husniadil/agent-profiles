//! Holding the machine awake with the lid shut, and giving it back.

use crate::agent_activity::Freshness;
use crate::platform::Power;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
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

/// How often the app revises its decision.
///
/// This is the one loop in the app whose cost is paid when nobody is watching —
/// the lid is shut and the machine is on battery. Fifteen seconds is a few
/// hundred `stat` calls and one `pmset` spawn, and it bounds how long a hold
/// outlives the agent that asked for it.
const SWEEP: Duration = Duration::from_secs(15);

/// Creates or removes the flag the root loop watches.
///
/// Empty on purpose. The loop tests only for existence, and a flag with contents
/// is an invitation for some later change to read them in a root shell.
pub fn apply(data_root: &Path, hold: bool) -> Result<()> {
    let flag = crate::paths::keep_awake_flag(data_root);
    if hold {
        if let Some(parent) = flag.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&flag, b"")?;
    } else if flag.exists() {
        std::fs::remove_file(&flag)?;
    }
    Ok(())
}

/// What one sweep has to remember from the last one.
///
/// Kept in the thread rather than in the [`Handle`]: nothing outside the loop
/// has any business advancing the clock, and a shared copy would let a command
/// that merely reads the state reset a running cap.
#[derive(Default)]
pub struct Sweep {
    pub held_for: Duration,
    pub capped: bool,
}

impl Sweep {
    /// Folds one decision into the running state.
    ///
    /// The clock runs while anything is asking, not only while holding: the cap
    /// bounds one stretch of keeping the machine from sleeping, and a pause for
    /// low battery is an interruption of that stretch rather than the end of it.
    /// Only the trigger going quiet — `Off` or `Idle` — starts a fresh one.
    pub fn observe(&mut self, phase: Phase, elapsed: Duration) {
        match phase {
            Phase::Off | Phase::Idle => {
                self.held_for = Duration::ZERO;
                self.capped = false;
            }
            Phase::PausedCapReached => {
                self.capped = true;
                self.held_for = self.held_for.saturating_add(elapsed);
            }
            Phase::Holding | Phase::PausedLowBattery => {
                self.held_for = self.held_for.saturating_add(elapsed);
            }
        }
    }
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

/// Everything the window asks for in one call.
#[derive(Serialize, Clone)]
pub struct Status {
    /// Whether this platform can hold the machine awake at all.
    pub supported: bool,
    /// Whether the privileged watchdog is running for this app run.
    pub authorized: bool,
    /// Whether a previous run may have left the machine unable to sleep.
    pub stranded: bool,
    pub phase: Phase,
    pub settings: Settings,
    pub roots: Vec<Freshness>,
    pub battery_percent: Option<u8>,
    pub on_external_power: bool,
    pub held_for_secs: u64,
    /// Why the feature cannot be offered on this machine, if it cannot. Today
    /// only a data root that cannot be safely quoted.
    pub refusal: Option<String>,
    /// Why the last sweep could not make the flag match its decision, if it
    /// could not.
    ///
    /// The flag is the only channel to the privileged loop, so a write that
    /// fails means the machine is not being held whatever the phase says. The
    /// window has to be able to say that: this feature's whole promise is that
    /// a user who trusted it and shut the lid can find out why it did not work.
    pub hold_error: Option<String>,
}

/// The shared state, owned by `AppState` and read by both the thread and the
/// commands.
pub struct Handle {
    pub data_root: PathBuf,
    pub home: PathBuf,
    settings: Mutex<Settings>,
    status: Mutex<Status>,
    /// The value the next watchdog must reset the sleep setting to, taken from
    /// a breadcrumb at startup. Consumed by the first authorization.
    reclaimed_prior: Mutex<Option<u8>>,
}

impl Handle {
    pub fn new(data_root: PathBuf, home: PathBuf, supported: bool, recovery: Recovery) -> Self {
        let settings = Settings::load(&crate::paths::keep_awake_settings(&data_root));
        let refusal = crate::paths::unquotable_refusal(&data_root);
        let status = Status {
            // A root that cannot be quoted is as unsupported as a platform that
            // cannot hold: in both cases the honest answer to "can this machine
            // do it?" is no, and the tab says which.
            supported: supported && refusal.is_none(),
            authorized: false,
            stranded: recovery.stranded,
            phase: Phase::Off,
            settings,
            roots: Vec::new(),
            battery_percent: None,
            on_external_power: false,
            held_for_secs: 0,
            refusal,
            hold_error: None,
        };
        Self {
            data_root,
            home,
            settings: Mutex::new(settings),
            status: Mutex::new(status),
            reclaimed_prior: Mutex::new(recovery.reclaimed_prior),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|held| *held).unwrap_or_default()
    }

    pub fn status(&self) -> Status {
        match self.status.lock() {
            Ok(held) => held.clone(),
            Err(poisoned) => poisoned.into_inner().clone(),
        }
    }

    pub fn set_settings(&self, next: Settings) -> Result<()> {
        let next = next.clamped();
        next.save(&crate::paths::keep_awake_settings(&self.data_root))?;
        if let Ok(mut held) = self.settings.lock() {
            *held = next;
        }
        if let Ok(mut status) = self.status.lock() {
            status.settings = next;
        }
        Ok(())
    }

    /// The prior value a watchdog about to be spawned must reset the setting to.
    ///
    /// Read without consuming, because spawning can fail — the password prompt
    /// is cancellable, and it is the most likely thing to go wrong here. Taking
    /// the value up front meant one cancelled prompt discarded the only record
    /// of a stranded machine: the retry would spawn with nothing to reclaim,
    /// adopt the live `SleepDisabled=1` as the user's own, and clear the banner
    /// that was the last thing telling them their Mac could not sleep.
    pub fn reclaimed_prior(&self) -> Option<u8> {
        self.reclaimed_prior.lock().ok().and_then(|it| *it)
    }

    /// Forgets the reclaim value, once a watchdog has actually taken it on.
    ///
    /// Separate from reading it so the two happen either side of the spawn: a
    /// second watchdog in the same run must not reclaim a value the first one
    /// has already put back.
    pub fn clear_reclaimed_prior(&self) {
        if let Ok(mut it) = self.reclaimed_prior.lock() {
            *it = None;
        }
    }

    pub fn mark_authorized(&self) {
        if let Ok(mut status) = self.status.lock() {
            status.authorized = true;
            status.stranded = false;
        }
    }

    pub fn mark_restored(&self) {
        if let Ok(mut status) = self.status.lock() {
            status.stranded = false;
        }
    }

    fn publish(
        &self,
        phase: Phase,
        roots: Vec<Freshness>,
        power: Power,
        held: Duration,
        hold_error: Option<String>,
    ) {
        if let Ok(mut status) = self.status.lock() {
            status.phase = phase;
            status.roots = roots;
            status.battery_percent = power.percent;
            status.on_external_power = power.external;
            status.held_for_secs = held.as_secs();
            status.hold_error = hold_error;
            // The watchdog writes the breadcrumb at spawn and removes it on a
            // clean exit, so its presence is a free liveness check — no `pgrep`,
            // no second channel that can disagree with the first.
            if status.authorized {
                status.authorized = crate::paths::keep_awake_breadcrumb(&self.data_root).exists();
            }
        }
    }
}

/// Every root worth watching: the agent CLIs, plus any profile directory whose
/// app relocates its sessions into the profile.
fn roots_for(state: &crate::runtime::AppState, home: &Path) -> Vec<crate::agent_activity::Root> {
    let mut roots = crate::agent_activity::cli_roots(home);
    for runtime in &state.apps {
        let Some(trace) = runtime.spec.session_trace else {
            continue;
        };
        let Ok(store) = runtime.store.lock() else {
            continue;
        };
        for profile in store.list() {
            roots.push(crate::agent_activity::Root {
                label: format!("{} · {}", runtime.spec.label, profile.label),
                path: profile.path.join(trace),
            });
        }
    }
    roots
}

/// The sweep, forever.
///
/// One thread for the life of the app rather than one started and stopped with
/// the trigger. While the trigger is `Off` a sweep is a mutex read and a sleep,
/// which is cheaper than the lifecycle management that avoiding it would need.
///
/// ponytail: a fixed fifteen-second tick. If the `stat` sweep ever shows up in a
/// power profile, watch the roots with FSEvents and keep the tick only for the
/// guards.
pub fn watch(app: tauri::AppHandle) {
    use tauri::Manager;

    let mut sweep = Sweep::default();
    let mut last = std::time::Instant::now();

    loop {
        std::thread::sleep(SWEEP);
        let elapsed = last.elapsed();
        last = std::time::Instant::now();

        let Some(state) = app.try_state::<crate::runtime::AppState>() else {
            continue;
        };
        let handle = &state.keep_awake;
        let settings = handle.settings();

        // `Off` costs one lock and nothing else — no process table, no `pmset`,
        // no filesystem walk. The default has to be free.
        if settings.trigger == Trigger::Off {
            sweep = Sweep::default();
            let released = apply(&handle.data_root, false);
            handle.publish(
                Phase::Off,
                Vec::new(),
                Power {
                    percent: None,
                    external: false,
                },
                Duration::ZERO,
                released.err().map(|error| error.to_string()),
            );
            continue;
        }

        // `state.inner()`, not `&state`: `roots_for` takes an `&AppState` and
        // `state` is a `tauri::State` wrapper around one.
        let roots = crate::agent_activity::scan(
            &roots_for(state.inner(), &handle.home),
            std::time::SystemTime::now(),
        );
        let window = Duration::from_secs(u64::from(settings.idle_window_minutes) * 60);
        let power = state.platform.power().unwrap_or(Power {
            percent: None,
            external: false,
        });

        let phase = decide(
            &settings,
            &Inputs {
                agent_active: crate::agent_activity::any_within(&roots, window),
                power,
                held_for: sweep.held_for,
                capped: sweep.capped,
            },
        );
        sweep.observe(phase, elapsed);

        // Carried into the status rather than only logged. The flag is the only
        // channel to the privileged loop, so a write that failed means the
        // machine is not being held no matter what the phase says — and a
        // window that goes on claiming otherwise is the one failure this
        // feature cannot afford.
        let hold_error = apply(&handle.data_root, phase.holds()).err().map(|error| {
            eprintln!("could not update the keep-awake flag: {error}");
            error.to_string()
        });
        handle.publish(phase, roots, power, sweep.held_for, hold_error);
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
    fn the_flag_appears_and_disappears_with_the_decision() {
        // The only channel between this app and the root loop. Everything else
        // in the feature exists to get this one file right.
        let d = tempfile::tempdir().unwrap();
        let flag = crate::paths::keep_awake_flag(d.path());

        apply(d.path(), true).unwrap();
        assert!(flag.exists());
        apply(d.path(), false).unwrap();
        assert!(!flag.exists());
    }

    #[test]
    fn applying_the_same_decision_twice_is_harmless() {
        // The sweep runs every fifteen seconds and most sweeps change nothing.
        let d = tempfile::tempdir().unwrap();
        apply(d.path(), true).unwrap();
        apply(d.path(), true).unwrap();
        assert!(crate::paths::keep_awake_flag(d.path()).exists());
        apply(d.path(), false).unwrap();
        apply(d.path(), false).unwrap();
        assert!(!crate::paths::keep_awake_flag(d.path()).exists());
    }

    #[test]
    fn the_flag_carries_nothing_the_root_loop_could_be_made_to_run() {
        // The loop only tests for existence, and this is the belt to that
        // braces: even if a future change read it, there is nothing there.
        let d = tempfile::tempdir().unwrap();
        apply(d.path(), true).unwrap();
        assert_eq!(
            std::fs::read(crate::paths::keep_awake_flag(d.path())).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn a_hold_that_ends_forgets_its_clock_and_its_cap() {
        // Otherwise the next agent inherits the last one's elapsed time and is
        // capped early, or inherits its latch and never holds at all.
        let mut sweep = Sweep::default();
        sweep.observe(Phase::Holding, Duration::from_secs(60));
        assert!(sweep.held_for > Duration::ZERO);

        sweep.observe(Phase::Idle, Duration::from_secs(15));
        assert_eq!(sweep.held_for, Duration::ZERO);
        assert!(!sweep.capped);
    }

    #[test]
    fn a_hold_that_reaches_the_cap_latches_it() {
        let mut sweep = Sweep::default();
        sweep.observe(Phase::PausedCapReached, Duration::from_secs(15));
        assert!(sweep.capped, "the latch is what stops the cap stuttering");
        // Still asking, still capped: the clock must not restart.
        sweep.observe(Phase::PausedCapReached, Duration::from_secs(15));
        assert!(sweep.capped);
    }

    #[test]
    fn a_battery_pause_keeps_the_clock_running() {
        // The cap measures how long the machine has been kept from sleeping in
        // one stretch. A pause for low battery does not restart that stretch —
        // plugging in resumes it rather than granting a fresh four hours.
        let mut sweep = Sweep::default();
        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        let before = sweep.held_for;
        sweep.observe(Phase::PausedLowBattery, Duration::from_secs(15));
        assert!(
            sweep.held_for >= before,
            "a battery pause must not reset the clock"
        );
    }

    fn handle_with(recovery: Recovery, root: &Path) -> Handle {
        Handle::new(root.to_path_buf(), root.join("home"), true, recovery)
    }

    #[test]
    fn a_reclaim_value_survives_an_authorization_that_never_happened() {
        // The password prompt is cancellable, and it is the likeliest thing to
        // fail here. Discarding the reclaim value on the way *into* the spawn
        // meant one cancelled prompt lost the only record that a previous run
        // had died holding the setting: the retry would find nothing to
        // reclaim, adopt the stuck `SleepDisabled=1` as the user's own, and
        // clear the banner that was telling them their Mac could not sleep.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: Some(0),
                stranded: true,
            },
            d.path(),
        );

        // Reading it — as a spawn attempt does — must not consume it.
        assert_eq!(handle.reclaimed_prior(), Some(0));
        assert_eq!(
            handle.reclaimed_prior(),
            Some(0),
            "a failed attempt must leave the next one something to reclaim"
        );

        // Only a watchdog that actually started forgets it.
        handle.clear_reclaimed_prior();
        assert_eq!(handle.reclaimed_prior(), None);
    }

    #[test]
    fn a_flag_that_could_not_be_written_is_reported_rather_than_only_logged() {
        // The flag is the only channel to the privileged loop. If the write
        // fails the machine is not being held, whatever the phase says, and a
        // window that keeps claiming otherwise is the single failure this
        // feature cannot afford.
        let d = tempfile::tempdir().unwrap();
        let unwritable = d.path().join("not-a-directory");
        // A file where the data root should be: every write beneath it fails.
        std::fs::write(&unwritable, b"").unwrap();

        let error = apply(&unwritable, true).unwrap_err().to_string();
        assert!(!error.is_empty(), "the failure must carry a reason");

        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        handle.publish(
            Phase::Holding,
            Vec::new(),
            Power {
                percent: Some(80),
                external: false,
            },
            Duration::ZERO,
            Some(error.clone()),
        );
        assert_eq!(handle.status().hold_error, Some(error));
    }

    #[test]
    fn a_sweep_that_wrote_the_flag_clears_a_previous_failure() {
        // Otherwise a single transient failure would leave the window warning
        // about a hold that is now working perfectly well.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let power = Power {
            percent: Some(80),
            external: false,
        };
        handle.publish(
            Phase::Holding,
            Vec::new(),
            power,
            Duration::ZERO,
            Some("disk full".into()),
        );
        assert!(handle.status().hold_error.is_some());

        handle.publish(Phase::Holding, Vec::new(), power, Duration::ZERO, None);
        assert_eq!(handle.status().hold_error, None);
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
