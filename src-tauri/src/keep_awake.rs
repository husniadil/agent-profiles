//! Holding the machine awake with the lid shut, and giving it back.

use crate::agent_activity::Freshness;
use crate::platform::{Power, Thermal};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Settings {
    #[serde(default)]
    pub trigger: Trigger,
    /// How long an agent that is part-way through a turn may write nothing
    /// before it is assumed dead.
    ///
    /// This no longer decides idleness — the transcript does, by saying whether
    /// the turn ended — so a finished agent releases the machine at once rather
    /// than after this many minutes. What is left is the bound on the other
    /// case: a session killed between a tool call and its result stays mid-turn
    /// on disk forever, and without this would hold the machine forever with it.
    ///
    /// The cost of setting it short is the one failure this feature exists to
    /// prevent: a single tool call that runs longer than this — a full test
    /// suite, a long build — writes nothing while it runs, so its agent is
    /// declared dead and the Mac sleeps mid-task.
    #[serde(default = "default_idle_window")]
    pub idle_window_minutes: u32,
    /// Below this charge, on battery, the hold is dropped even mid-task.
    #[serde(default = "default_battery_floor")]
    pub battery_floor_percent: u8,
    /// Whether an overheating machine releases the hold.
    ///
    /// On by default, and the default a settings file written before this
    /// existed also gets: heat is the one guard where continuing does damage
    /// rather than spending charge, so an upgrade must not silently arrive with
    /// it off. Off is offered because the reading is the system's judgement, not
    /// a temperature — someone running a deliberately hot machine on a bench may
    /// disagree with it, and would otherwise have no way to say so.
    #[serde(default = "default_thermal_guard")]
    pub thermal_guard: bool,
}

/// Ten minutes, not two: the two losses are not comparable. Holding a dead
/// session too long spends battery, which the floor guard already bounds
/// independently; releasing a live one loses the run, and the user sees a dead
/// build rather than a released hold and cannot attribute it. A foreground
/// command running five minutes is the most common thing an agent does that
/// takes real time, so two minutes fails at the purpose of the feature.
fn default_idle_window() -> u32 {
    10
}
fn default_battery_floor() -> u8 {
    30
}
fn default_thermal_guard() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            trigger: Trigger::Off,
            idle_window_minutes: default_idle_window(),
            battery_floor_percent: default_battery_floor(),
            thermal_guard: default_thermal_guard(),
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
            // A bool has no range to be dragged into; it is carried through so
            // that adding a field here stays a compile error rather than a
            // setting that silently resets on every save.
            thermal_guard: self.thermal_guard,
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
    /// Something is asking, and the machine is too hot to make it worse.
    PausedTooHot,
}

impl Phase {
    /// Whether this phase wants the machine held. What that costs is the
    /// platform's business — a flag file, an inhibitor lock, a power scheme.
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
    /// How hot the machine is right now.
    pub thermal: Thermal,
}

/// The whole policy, as a pure function.
///
/// Pure on purpose: this is the code that decides whether a machine in someone's
/// bag stays awake, and it must be exercisable at a flat battery, on an
/// overheating laptop and on a desktop with no battery at all without any of
/// those being true of the machine running the tests.
pub fn decide(settings: &Settings, inputs: &Inputs) -> Phase {
    let asking = match settings.trigger {
        Trigger::Off => return Phase::Off,
        Trigger::Always => true,
        Trigger::AgentActive => inputs.agent_active,
    };
    if !asking {
        return Phase::Idle;
    }

    // Heat is checked first because it is the only guard where continuing does
    // damage rather than merely running a battery down. It also replaced a
    // duration cap that existed solely to stand in for this reading — a clock
    // is a poor proxy for a temperature, and it stopped holds that were
    // perfectly fine while missing a machine cooking in a bag after ten minutes.
    if settings.thermal_guard && inputs.thermal.is_danger() {
        return Phase::PausedTooHot;
    }

    // Fails open, and deliberately: `None` is a desktop or a reading that did
    // not come back, neither of which is a flat battery.
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

/// The phase to act on, once authorization is taken into account.
///
/// [`decide`] is pure and knows nothing about the admin password the hold needs
/// where the platform demands one — so on a working agent it returns `Holding`
/// whether or not that password has been given. But an unauthorized run cannot
/// hold anything: on macOS the flag it writes has no privileged watchdog reading
/// it. Left alone it would light the window green and start the "held" clock for
/// a machine that is free to sleep. So a decision to hold, while authorization is
/// still pending, is downgraded to `Idle` — the run watches, it just does not
/// claim a hold it cannot make. The window already says as much: before
/// authorization the honest answer is always "you have not authorized yet".
pub fn effective_phase(decided: Phase, awaiting_authorization: bool) -> Phase {
    if awaiting_authorization {
        Phase::Idle
    } else {
        decided
    }
}

/// How often the app revises its decision.
///
/// This is the one loop in the app whose cost is paid when nobody is watching —
/// the lid is shut and the machine is on battery. Fifteen seconds is a few
/// hundred `stat` calls and one `pmset` spawn, and it bounds how long a hold
/// outlives the agent that asked for it.
const SWEEP: Duration = Duration::from_secs(15);

/// How long the sweep will stand aside for an update handoff before deciding the
/// install is never reporting back.
///
/// The pause exists because the fifteen-second sweep is faster than the gap
/// between handing the machine over and the installer taking the process, and a
/// sweep that re-armed in that gap would put the Windows lid-close action back to
/// "do nothing" and then the process would end with it stuck. But the only thing
/// that clears the pause is the window's `catch`, and a webview that dies — or an
/// `install()` that never settles — never reaches it. Without a deadline that
/// leaves keep-awake switched off for the life of the process, with no log line
/// and no banner: exactly the state `release_for_update` refuses to latch
/// `stopping` in order to avoid.
///
/// Ten minutes, chosen from what the two mistakes cost rather than from a round
/// number:
///
/// * Expiring too early re-arms mid-install, which is the defect the pause is
///   for. The bundle is already downloaded by the time the handoff starts, so
///   what has to fit inside this is `install()` plus `relaunch()` — seconds,
///   minutes at the outside with an antivirus scanning the bundle. Ten minutes is
///   an order of magnitude clear of that.
/// * Expiring too late leaves the feature dead. On a *successful* install the
///   process is gone long before the deadline can matter, so this only ever fires
///   on a handoff that already failed to take the process — and on Windows, the
///   one platform where a re-arm is destructive, the NSIS `exit(0)` arrives
///   within seconds of `install()`, so a ten-minute-old pause on a process that
///   is still alive means the installer is not coming back for it.
///
/// It is a backstop, not a schedule: [`resume_after_failed_update`] still clears
/// the pause in seconds on every path the window survives.
const UPDATE_HANDOFF: Duration = Duration::from_secs(10 * 60);

/// Creates or removes the flag the root loop watches.
///
/// Empty on purpose. The loop tests only for existence, and a flag with contents
/// is an invitation for some later change to read them in a root shell.
///
/// This is `Platform::hold`'s default, not the mechanism everywhere: a flag is
/// how an unprivileged process asks a privileged one for something. Linux needs
/// no privileged one and overrides it.
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

/// Puts sleep back after a run that died holding it, and makes that stick.
///
/// Disarms the trigger and drops the flag before restoring anything, because
/// the way out has to stay out. It used to be arguable that neither was needed:
/// `stranded` and `authorized` could not both be true inside one `Handle` —
/// where a password was needed the app started unauthorized, and the only thing
/// that authorized it cleared `stranded` in the same breath — so this app's own
/// sweep would be writing a flag that nothing was watching.
///
/// Since the grant outlives the process (#55) that is no longer true: a launch
/// can find itself authorized *and* stranded, start a loop straight away, and be
/// watching that flag within milliseconds. The argument below was already the
/// load-bearing one; this is now the second reason rather than the only one.
///
/// That argument is about a process, and `disablesleep` is a machine. Nothing
/// stops a second copy of the app running; `cargo tauri dev` beside the
/// installed build is the likeliest way, and both derive the same data root
/// from `$HOME`. The second copy's `recover_at_startup` deletes the flag and
/// the breadcrumb, so it reports a stranded machine while the first copy's root
/// loop is still alive and still polling that same flag every three seconds.
/// Pressing the button there put sleep back and the older loop took it away
/// again within one poll, with the banner cleared and nothing on screen saying
/// so — which is the whole failure, arriving through the one door the state
/// machine cannot see.
///
/// Each step earns its place, in this order:
///
/// 1. The trigger, or this app's own sweep rewrites the flag fifteen seconds
///    later. Persisted rather than held in memory, so the next launch does not
///    quietly start holding again either.
/// 2. The flag, because it is the only channel to a loop this process did not
///    start and cannot see. The loop is edge-triggered on the flag existing, so
///    removing it is what makes a *foreign* watchdog let go.
/// 3. The setting, last, so nothing can re-take it in between.
///
/// A flag that will not delete is reported but does not skip the restore: a
/// machine that cannot sleep is the thing being fixed, and refusing to try
/// would leave the user with neither half.
pub fn restore(handle: &Handle, platform: &dyn crate::platform::Platform) -> Result<()> {
    handle.set_settings(Settings {
        trigger: Trigger::Off,
        ..handle.settings()
    })?;
    let dropped = platform.hold(&handle.data_root, false);
    platform.restore_sleep()?;
    dropped?;
    handle.mark_restored();
    Ok(())
}

/// Hands the machine back on the way out.
///
/// Two of the three backends survive a quit without this: macOS's root loop
/// watches our pid and puts `disablesleep` back within one poll, and Linux's
/// inhibitor lock dies with the pipe it was spawned on. Windows does not — its
/// hold *is* a power-scheme write, so a quit while holding leaves the lid-close
/// action on "do nothing" until the next launch runs `recover_hold`. That is a
/// laptop that goes in a bag awake, and the app that owns the way back is the
/// one that just exited.
///
/// Not `restore`: quitting is not "turn Keep Awake off". The trigger is left
/// exactly as the user set it, so the next launch holds again if it should —
/// only the hold itself goes back.
///
/// Stops the sweep before releasing, not after: the sweep is a detached loop
/// that runs until the process ends, and on a tray Quit it can be part-way
/// through an iteration whose final act is a `hold(true)`. Releasing without
/// stopping it first would let that iteration re-take the hold — writing the
/// Windows lid action back to "do nothing" — one instant after we handed the
/// machine back, and then the process would end with it stuck. Order matters:
/// the flag is set first so a sweep that checks it after we release still sees
/// the stop.
pub fn release_at_exit(handle: &Handle, platform: &dyn crate::platform::Platform) -> Result<()> {
    handle.stop_sweeping();
    platform.hold(&handle.data_root, false)
}

/// Hands the machine back before an update installs, and holds the sweep off
/// until that install either takes the process or fails.
///
/// Unlike [`release_at_exit`], this does not latch `stopping`. Installing an
/// update is not the same as exiting: the bundled updater relaunches on success,
/// but `install()` can fail — the download and this release both succeed, then
/// the install itself throws — and on macOS and Linux the app is still running
/// afterwards. If this release killed the sweep, keep-awake would be dead for the
/// rest of that run with no record, and the sweep's whole promise (hold a working
/// agent's machine awake) would silently stop being kept until a manual relaunch.
///
/// So it pauses instead of stopping. A pause the sweep observes but that does not
/// end it keeps both properties at once:
///
/// * Nothing re-arms during the handoff. The gap between this call and the
///   installer taking over is easily longer than the fifteen-second [`SWEEP`], so
///   without the pause a sweep could tick, re-take the hold, and write the Windows
///   lid-close action back to "do nothing" — undoing the handoff moments before
///   the NSIS installer's `exit(0)` ends the process with it stuck, with no run
///   left to fix it. That is the failure this pause exists for, and it is the one
///   that does not self-heal: on Windows the hold *is* a power-scheme write, so an
///   install that fails after `exit(0)` (an antivirus block, a disk error, a
///   killed installer) leaves the lid doing nothing until the user next opens
///   Agent Profiles.
/// * A failed install still leaves keep-awake working, because the window clears
///   the pause in its `catch` — see [`resume_after_failed_update`] — and the very
///   next sweep re-arms the hold from the trigger the user still has set.
///
/// Pauses before releasing, in that order and for the same reason
/// [`release_at_exit`] latches before releasing: a sweep that checks the flag
/// after we release still sees it, whereas the other order leaves a window where
/// it does not.
///
/// If the window is gone before it can clear the pause, the sweep stays paused
/// for the rest of the run — which is the safe direction. Paused means "did not
/// re-take the hold": the machine is left free to sleep and the lid works, the
/// same state the release just put it in.
pub fn release_for_update(handle: &Handle, platform: &dyn crate::platform::Platform) -> Result<()> {
    handle.pause_sweeping();
    let released = platform.hold(&handle.data_root, false);
    // `publish` is the only writer of the phase, the held clock and the hold
    // error, and the paused sweep does not reach it — so without this the window
    // would go on showing `Holding` and "held 42m" for the whole install, for a
    // machine that is free to sleep. Re-publishing `Holding` from the sweep would
    // be the same lie one tick later; the truth is said here, where it becomes
    // true. If the release itself failed, that is what gets published, exactly as
    // the `Trigger::Off` branch of the sweep does.
    handle.publish_released(released.as_ref().err().map(|error| error.to_string()));
    released
}

/// Undoes the handoff, for an install that did not take the process with it.
///
/// Called from the window's `catch`, which covers every way the install can end
/// without exiting — including the release itself failing. Deliberately not
/// conditional on *why* it failed: the app is still running, so keep-awake has to
/// go on working, and a pause left set would be a feature silently switched off.
pub fn resume_after_failed_update(handle: &Handle) {
    handle.resume_sweeping();
}

/// The one thing a sweep carries forward: how long this stretch of holding has
/// run, which the window reports as "held 15m".
///
/// It used to latch a duration cap as well. The cap is gone — a Keep Awake
/// feature that stops on a clock is answering a question nobody asked, and the
/// thermal reading now covers what it was standing in for.
#[derive(Default)]
pub struct Sweep {
    pub held_for: Duration,
    /// Whether the *previous* sweep left the machine actually held: it wanted a
    /// hold and the platform took it. The interval a sweep observes was lived
    /// under that outcome, not under the phase this sweep has just decided.
    held: bool,
}

impl Sweep {
    /// Only the trigger going quiet starts a fresh stretch. A pause for heat or
    /// battery interrupts one rather than ending it, so plugging in or cooling
    /// down resumes the same figure instead of restarting it.
    ///
    /// What the interval gets credited against is the *previous* sweep's
    /// outcome, not this sweep's phase, because that is who was holding the
    /// machine while the interval was being lived. So a `Holding` phase whose
    /// hold errored adds nothing on the sweep after it: the phase is what the
    /// app asked for, and a machine free to sleep is not being held no matter
    /// what the phase says. The first paused sweep still credits the interval
    /// it spent held, since the release only happens further down this same
    /// sweep; every paused sweep after it adds nothing.
    pub fn observe(&mut self, phase: Phase, elapsed: Duration) {
        match phase {
            Phase::Off | Phase::Idle => self.held_for = Duration::ZERO,
            _ if self.held => self.held_for = self.held_for.saturating_add(elapsed),
            _ => {}
        }
    }

    /// What the sweep achieved, told after the hold was attempted: the phase it
    /// asked for and whether the platform obliged. Nothing else may set this —
    /// the whole point is that the clock answers to the hold, not to the phase.
    ///
    /// Private, and deliberately: the only production caller is [`hold_step`],
    /// which is the one function that attempts the write this is the record of.
    /// Keeping it out of the module's surface means "the clock answers to the
    /// hold" is enforced by the compiler rather than by a comment.
    fn settle(&mut self, phase: Phase, hold_error: Option<&str>) {
        self.held = phase.holds() && hold_error.is_none();
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

/// What this machine can actually do, asked of the platform once at startup.
///
/// A struct rather than two bools in a row, for the reason every pair of
/// adjacent booleans is one: swapped, they compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capabilities {
    /// Whether the machine can be held awake with the lid shut.
    pub hold: bool,
    /// Whether it can report how hot it is. False on Windows, and on a Linux
    /// box with no thermal zones — see `Platform::can_read_thermal`.
    pub thermal: bool,
    /// Whether holding costs the user an administrator prompt. True only on
    /// macOS; elsewhere the app is already allowed to do this, and the window
    /// skips the whole ask-first band.
    pub needs_authorization: bool,
    /// Whether that prompt has already been answered on this machine, by some
    /// earlier run. Meaningless where nothing needs authorizing, and never read
    /// there — see [`starts_authorized`].
    pub authorization_installed: bool,
}

/// Whether a run may consider itself authorized before asking anybody anything.
///
/// The decision issue #55 turned on. It used to be `!needs_authorization`, which
/// made every macOS launch start unauthorized and put the password prompt in
/// front of the user again — once per run, forever, because a grant obtained by
/// elevating *this process* dies with it.
///
/// The fix is not to remember an answer, which would let the window claim an
/// authorization the run does not hold. It is to ask a question whose answer
/// outlives the process: the grant lives on the machine, so a later launch can
/// go and look. `authorization_installed` is that look.
pub fn starts_authorized(capabilities: Capabilities) -> bool {
    !capabilities.needs_authorization || capabilities.authorization_installed
}

/// Everything the window asks for in one call.
#[derive(Serialize, Clone)]
pub struct Status {
    /// Whether this platform can hold the machine awake at all.
    pub supported: bool,
    /// Whether the thermal guard has a reading to act on here. False means the
    /// window leaves the switch out entirely: a guard that cannot fire is worse
    /// than an absent one, because it looks like protection.
    pub thermal_supported: bool,
    /// Whether this machine asks for an administrator password before it can
    /// hold. False means the window shows no authorization step at all, rather
    /// than a button that grants something already granted.
    pub needs_authorization: bool,
    /// Whether the privileged watchdog is running for this app run. Always true
    /// where nothing needed authorizing in the first place.
    pub authorized: bool,
    pub stranded: bool,
    pub phase: Phase,
    pub settings: Settings,
    pub roots: Vec<Freshness>,
    pub battery_percent: Option<u8>,
    pub on_external_power: bool,
    /// How hot the machine is, so the window can say which guard stopped a hold
    /// rather than just that one did.
    pub thermal: Thermal,
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
    /// Wakes the sweep before its fifteen-second timer is up. A settings change
    /// nudges this so a toggled trigger takes effect now, not on the next tick —
    /// the boolean carries the nudge across the gap where the sweep is busy
    /// computing rather than waiting, which a bare `notify` would drop.
    wake: std::sync::Condvar,
    wake_pending: Mutex<bool>,
    /// Set once, when the app is handing the machine back on its way out via
    /// [`release_at_exit`]. The sweep is an unconditional loop that outlives every
    /// quit path bar the process ending, so a release on the exit path could be
    /// undone by a sweep still mid-iteration — on Windows the hold is a
    /// power-scheme write that stands until the next launch. The sweep reads this
    /// immediately before it would re-arm, and stops instead. One-way: nothing
    /// clears it, because the only thing that sets it is the exit path, where the
    /// app really is going away. The update path deliberately does *not* latch it
    /// (see [`release_for_update`]): an install can fail and leave the app
    /// running, and a latched flag would kill the sweep for the rest of that run.
    stopping: AtomicBool,
    /// When the update handoff currently in progress runs out of patience, if one
    /// is in progress at all.
    ///
    /// The difference from `stopping` is that this one comes back — twice over.
    /// The exit path really is going away, so latching is right there; an install
    /// is only *probably* going away — `install()` can throw, and on macOS and
    /// Linux the app is still running afterwards — so a latch would leave
    /// keep-awake dead for the rest of that run with nothing on screen saying so.
    /// The window clears it in its `catch`, and if the window never gets that far
    /// the deadline clears it anyway: a pause is a promise that an installer is
    /// about to take this process, and after [`UPDATE_HANDOFF`] that promise is
    /// no longer credible.
    ///
    /// A deadline rather than a flag plus a timestamp, so there is one thing to
    /// read and no way for the two to disagree.
    handoff_until: Mutex<Option<Instant>>,
}

impl Handle {
    pub fn new(
        data_root: PathBuf,
        home: PathBuf,
        capabilities: Capabilities,
        recovery: Recovery,
    ) -> Self {
        let settings = Settings::load(&crate::paths::keep_awake_settings(&data_root));
        let refusal = crate::paths::unquotable_refusal(&data_root);
        let status = Status {
            // A root that cannot be quoted is as unsupported as a platform that
            // cannot hold: in both cases the honest answer to "can this machine
            // do it?" is no, and the tab says which.
            supported: capabilities.hold && refusal.is_none(),
            thermal_supported: capabilities.thermal,
            needs_authorization: capabilities.needs_authorization,
            // Authorized from the start wherever there was nothing to
            // authorize — Linux takes a logind inhibitor as the user and Windows
            // writes a power scheme the user owns — and, since #55, also
            // wherever the one-time grant is already on the machine.
            authorized: starts_authorized(capabilities),
            stranded: recovery.stranded,
            phase: Phase::Off,
            settings,
            roots: Vec::new(),
            battery_percent: None,
            on_external_power: false,
            thermal: Thermal::Unknown,
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
            wake: std::sync::Condvar::new(),
            wake_pending: Mutex::new(false),
            stopping: AtomicBool::new(false),
            handoff_until: Mutex::new(None),
        }
    }

    /// Tells the sweep to stop before it next touches the hold. Called from the
    /// exit path so a release cannot be re-armed by a sweep this process did not
    /// wait for. Wakes the sweep as well, so a thread parked on its timer sees
    /// the flag now rather than up to a tick later.
    pub fn stop_sweeping(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.nudge();
    }

    /// Whether the app is on its way out. Read by the sweep immediately before it
    /// would re-take the hold.
    pub fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::SeqCst)
    }

    /// Tells the sweep to stop re-arming the hold, without ending it. Called from
    /// the update path, where the process is probably about to be replaced but
    /// may yet survive a failed install.
    pub fn pause_sweeping(&self) {
        self.pause_sweeping_for(UPDATE_HANDOFF);
    }

    /// The same pause, with the budget named. Production always takes
    /// [`UPDATE_HANDOFF`]; this exists so the expiry can be driven at any age
    /// without a test sleeping through ten minutes of it.
    pub fn pause_sweeping_for(&self, budget: Duration) {
        if let Ok(mut until) = self.handoff_until.lock() {
            *until = Some(Instant::now() + budget);
        }
    }

    /// Lets the sweep re-arm again, after an install that did not happen. Nudges,
    /// so a machine the user still has a trigger armed for is held again now
    /// rather than up to a tick later.
    pub fn resume_sweeping(&self) {
        if let Ok(mut until) = self.handoff_until.lock() {
            *until = None;
        }
        self.nudge();
    }

    /// Whether an update handoff is in progress *and still credible*. Read by the
    /// sweep immediately before it would re-take the hold.
    ///
    /// Clears an expired handoff rather than merely reporting it expired, so the
    /// line below is printed once, when the sweep is given back, and not every
    /// fifteen seconds for the rest of the run. That makes this a read with a
    /// side effect, which is why the only caller is [`hold_step`] — the one place
    /// that acts on the answer.
    pub fn is_paused(&self) -> bool {
        let Ok(mut until) = self.handoff_until.lock() else {
            return false;
        };
        let Some(deadline) = *until else {
            return false;
        };
        if Instant::now() < deadline {
            return true;
        }
        *until = None;
        eprintln!(
            "update install has not reported back in {}s: holding the machine \
             awake again rather than leaving keep-awake off for the rest of \
             this run",
            UPDATE_HANDOFF.as_secs()
        );
        false
    }

    /// Wakes the sweep now instead of at the end of its timer.
    pub fn nudge(&self) {
        if let Ok(mut pending) = self.wake_pending.lock() {
            *pending = true;
            self.wake.notify_all();
        }
    }

    /// Waits until the next sweep is due or a [`nudge`](Self::nudge) arrives,
    /// whichever comes first. A nudge that lands while the caller was not yet
    /// waiting is not lost — it is left pending and consumed here at once.
    pub fn wait_for_sweep(&self, timeout: Duration) {
        let Ok(mut pending) = self.wake_pending.lock() else {
            return;
        };
        if *pending {
            *pending = false;
            return;
        }
        if let Ok((mut guard, _)) = self.wake.wait_timeout(pending, timeout) {
            *guard = false;
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
        // Re-decide now rather than up to a tick later: a user who just turned
        // this on is watching for the status to answer, and a fifteen-second
        // wait reads as the toggle having done nothing.
        self.nudge();
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

    /// Whether the hold still needs an admin password before it can act.
    ///
    /// The sweep reads this to keep an unauthorized run from claiming a hold it
    /// cannot make — see [`effective_phase`].
    pub fn awaiting_authorization(&self) -> bool {
        self.status
            .lock()
            .map(|status| status.needs_authorization && !status.authorized)
            .unwrap_or(false)
    }

    pub fn mark_authorized(&self) {
        if let Ok(mut status) = self.status.lock() {
            status.authorized = true;
            status.stranded = false;
        }
    }

    /// Walks back the optimism in [`starts_authorized`].
    ///
    /// A run that finds the grant already on the machine reports itself
    /// authorized before anything has been started, because that is what the
    /// window has to render on its first frame. If the loop then fails to start,
    /// the claim stops being true, and the honest thing is to put the Authorize
    /// button back rather than leave a window saying the machine is being held
    /// by a loop that is not running.
    pub fn mark_unauthorized(&self) {
        if let Ok(mut status) = self.status.lock() {
            status.authorized = false;
        }
    }

    pub fn mark_restored(&self) {
        if let Ok(mut status) = self.status.lock() {
            status.stranded = false;
        }
    }

    /// Says the one thing that just became true — the machine is no longer being
    /// held — without inventing the readings only a sweep can take.
    ///
    /// The battery, thermal and root readings are left exactly as the last sweep
    /// left them: this is not a sweep and has nothing fresh to say about them.
    /// What it must not leave standing is `Holding` and a running "held …", for a
    /// machine whose lid is working again.
    fn publish_released(&self, hold_error: Option<String>) {
        if let Ok(mut status) = self.status.lock() {
            status.phase = Phase::Idle;
            status.held_for_secs = 0;
            status.hold_error = hold_error;
        }
    }

    fn publish(
        &self,
        phase: Phase,
        roots: Vec<Freshness>,
        power: Power,
        thermal: Thermal,
        held: Duration,
        hold_error: Option<String>,
    ) {
        if let Ok(mut status) = self.status.lock() {
            status.phase = phase;
            status.roots = roots;
            status.battery_percent = power.percent;
            status.on_external_power = power.external;
            status.thermal = thermal;
            status.held_for_secs = held.as_secs();
            status.hold_error = hold_error;
            // The watchdog writes the breadcrumb at spawn and removes it on a
            // clean exit, so its presence is a free liveness check — no `pgrep`,
            // no second channel that can disagree with the first. Only where
            // there is a watchdog: elsewhere no breadcrumb is ever written, and
            // this would report a working hold as dead on its first sweep.
            if status.needs_authorization && status.authorized {
                status.authorized = crate::paths::keep_awake_breadcrumb(&self.data_root).exists();
            }
        }
    }
}

/// Every root worth watching: the agent CLIs, plus any profile directory whose
/// app relocates its sessions into the profile.
fn roots_for(state: &crate::runtime::AppState, home: &Path) -> Vec<crate::agent_activity::Root> {
    let mut roots = crate::agent_activity::cli_roots(home);
    let now = std::time::SystemTime::now();
    for runtime in &state.apps {
        let Some(trace) = runtime.spec.session_trace else {
            continue;
        };
        let Ok(store) = runtime.store.lock() else {
            continue;
        };
        for profile in store.list() {
            let path = profile.path.join(trace);
            // Only once this profile has actually written a session. The Default
            // profile is the app's own existing install, listed for every app
            // whether or not it has ever run — so an untouched ChatGPT showed
            // `ChatGPT · Default … never`, a row watching an empty or absent
            // folder. A profile earns its row by having been used, the same rule
            // the Claude projects and the Codex CLI root already follow.
            if crate::agent_activity::newest_age(&path, now).is_none() {
                continue;
            }
            roots.push(crate::agent_activity::Root {
                label: format!("{} · {}", runtime.spec.label, profile.label),
                path,
                // A profile's session trace is a directory this app does not
                // read the contents of. Freshness is all there is.
                reading: crate::agent_activity::Reading::Mtime,
            });
        }
    }
    roots
}

/// What the one re-arming step of a sweep iteration decided to do.
pub enum HoldStep {
    /// The app is on its way out: the step touched nothing, and the sweep must
    /// end rather than run on.
    Stopped,
    /// An update install is handing the machine over: the step touched nothing,
    /// and the sweep runs on — the pause is cleared if that install fails.
    Skipped,
    /// The hold decision was applied through the platform, carrying whatever
    /// error the write returned so the window can report it.
    Applied(Option<String>),
}

/// The one step of a sweep iteration that can re-arm the hold, pulled out of
/// [`watch`] so the re-arm guard can be exercised on its own.
///
/// The `is_stopping` check lives *here*, immediately before the only `hold`
/// write that can *take or re-arm* a hold, and not at the top of the loop: the
/// race being closed is a sweep already mid-iteration when [`release_at_exit`]
/// ran, which would rewrite the Windows lid action to "do nothing" one instant
/// after the exit path handed the machine back, then end the process with it
/// stuck. The sweep makes one other `hold` write — the `Trigger::Off` branch in
/// [`watch`] — but that one only ever *releases*, so a stop landing there needs
/// no guard: releasing again is a no-op, and it can never re-arm the lid action.
/// (A future change that let that branch re-arm would have to move the guard, or
/// duplicate it, rather than lean on this one.) `watch`
/// calls this in the same order it used to run inline, so runtime behaviour is
/// unchanged — extracting it only makes the guard testable, which it was not
/// before: a suite could delete this check and stay green.
///
/// The sweep clock is settled *here*, for the same reason. [`Sweep::held`] is
/// the record of whether the machine was really held, and it is only knowable
/// once this function has attempted the write. Left as a separate line in
/// `watch` it was untestable in exactly the way the guard above used to be:
/// delete it, reorder it, or jump it with a `continue`, and `held` goes stale
/// while every test stays green — the window then reports a pause, or an
/// outage, as time the machine was held. Every exit from this function settles,
/// so there is no path out of the one place that takes a hold which leaves the
/// clock believing a stale answer.
///
/// The update handoff's pause sits *beside* that check rather than replacing it,
/// and for the same reason: it is the re-arm that must not happen. A paused sweep
/// must still be allowed to release — the `Trigger::Off` branch in [`watch`] is
/// unguarded and stays that way, because releasing again is a no-op and can never
/// strand the lid. The two are separate because they end differently: `stopping`
/// ends the loop, a pause only skips this step, so the sweep is still there to
/// re-arm the moment a failed install clears it.
pub fn hold_step(
    handle: &Handle,
    platform: &dyn crate::platform::Platform,
    phase: Phase,
    sweep: &mut Sweep,
) -> HoldStep {
    if handle.is_stopping() {
        // Nothing was written, and `release_at_exit` has already handed the
        // machine back — so whatever the phase asked for, nothing is held.
        sweep.settle(Phase::Idle, None);
        return HoldStep::Stopped;
    }
    if handle.is_paused() {
        // Nothing was written and `release_for_update` has already handed the
        // machine back, so nothing is held — and the clock has to be told, or the
        // whole install is credited to the next sweep as time the machine was
        // awake. This is #48: settling in `watch` put the line where a `continue`
        // could jump it, and settling here means the only way out of this branch
        // goes through it.
        sweep.settle(Phase::Idle, None);
        return HoldStep::Skipped;
    }

    // Carried into the status rather than only logged. Whatever the platform's
    // channel is — a flag file the root loop watches, an inhibitor lock, a power
    // scheme — a failure here means the machine is not being held no matter what
    // the phase says, and a window that goes on claiming otherwise is the one
    // failure this feature cannot afford.
    let hold_error = platform
        .hold(&handle.data_root, phase.holds())
        .err()
        .map(|error| {
            eprintln!(
                "could not {}: {error}",
                if phase.holds() {
                    "hold the machine awake"
                } else {
                    "release the machine"
                }
            );
            error.to_string()
        });
    // Told what actually happened, so the next sweep credits its interval to
    // the hold that was really in place rather than to the phase that asked
    // for one.
    sweep.settle(phase, hold_error.as_deref());
    HoldStep::Applied(hold_error)
}

/// The sweep, forever.
///
/// One thread for the life of the app rather than one started and stopped with
/// the trigger. While the trigger is `Off` a sweep is a mutex read and a wait,
/// which is cheaper than the lifecycle management that avoiding it would need.
///
/// The wait is nudgeable: fifteen seconds is the idle cadence, but a settings
/// change wakes it at once so a toggled trigger is answered now rather than on
/// the next tick.
///
/// ponytail: a fifteen-second idle tick. If the `stat` sweep ever shows up in a
/// power profile, watch the roots with FSEvents and keep the tick only for the
/// guards.
pub fn watch(app: tauri::AppHandle) {
    use tauri::Manager;

    let mut sweep = Sweep::default();
    let mut last = std::time::Instant::now();

    loop {
        let Some(state) = app.try_state::<crate::runtime::AppState>() else {
            // No state yet (early startup): a plain sleep, so this is not a busy
            // loop while there is nothing to wait on.
            std::thread::sleep(SWEEP);
            continue;
        };
        let handle = &state.keep_awake;
        // Wait out the tick, but return the moment a settings change nudges us,
        // so a toggled trigger is re-decided now instead of up to a tick later.
        handle.wait_for_sweep(SWEEP);
        // The app is on its way out and `release_at_exit` has handed the machine
        // back. Stop the whole loop rather than run another iteration: on the
        // parked-thread case this is what actually ends the sweep. The narrower
        // case — a release that lands while this iteration is already past here
        // — is caught by the second check, just before the hold itself.
        if handle.is_stopping() {
            return;
        }
        let elapsed = last.elapsed();
        last = std::time::Instant::now();

        let settings = handle.settings();

        // `Off` costs one lock and nothing else — no process table, no `pmset`,
        // no filesystem walk. The default has to be free.
        if settings.trigger == Trigger::Off {
            sweep = Sweep::default();
            let released = state.platform.hold(&handle.data_root, false);
            handle.publish(
                Phase::Off,
                Vec::new(),
                Power {
                    percent: None,
                    external: false,
                },
                Thermal::Unknown,
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
        let thermal = state.platform.thermal();

        let decided = decide(
            &settings,
            &Inputs {
                agent_active: crate::agent_activity::any_working(&roots, window),
                power,
                thermal,
            },
        );
        // An unauthorized run cannot hold, so it must not say it is holding, try
        // to, or start the "held" clock. Downgraded to `Idle` before anything
        // acts on it.
        let phase = effective_phase(decided, handle.awaiting_authorization());
        sweep.observe(phase, elapsed);

        // The app may be handing the machine back on its way out. The stopping
        // check and the hold write are one extracted step — `hold_step` — so the
        // re-arm guard can be tested; the check sits immediately before the only
        // call that re-takes the hold, because the race being closed is exactly a
        // sweep already mid-iteration when `release_at_exit` ran.
        // `hold_step` settles the clock itself, on every path out of it, so no
        // line here can be deleted or jumped in a way that leaves `held` stale.
        let hold_error = match hold_step(handle, state.platform.as_ref(), phase, &mut sweep) {
            HoldStep::Stopped => return,
            // An update install is taking the machine over. Nothing was
            // written, so nothing is being held — and that is what gets
            // published. Re-publishing `Holding` would claim a hold
            // `release_for_update` gave back; publishing nothing leaves the same
            // claim standing, because it is what the window last saw. `Idle`
            // with a zero clock is the true statement, and the readings around
            // it are still this sweep's own. If the install fails the pause
            // clears and the next sweep publishes a real hold again.
            HoldStep::Skipped => {
                handle.publish(Phase::Idle, roots, power, thermal, Duration::ZERO, None);
                continue;
            }
            HoldStep::Applied(hold_error) => hold_error,
        };
        handle.publish(phase, roots, power, thermal, sweep.held_for, hold_error);
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
            thermal: Thermal::Nominal,
        }
    }

    #[test]
    fn an_unauthorized_run_watches_but_never_claims_to_hold() {
        // `decide` knows nothing about the admin password the hold needs, so on
        // a working agent it returns Holding regardless. Until that password is
        // given the machine cannot be held — the reported defect was the window
        // going green and the "held" clock starting for a machine free to sleep.
        assert_eq!(effective_phase(Phase::Holding, true), Phase::Idle);
        assert_eq!(effective_phase(Phase::PausedLowBattery, true), Phase::Idle);
        // Authorized, the phase is exactly the decision.
        assert_eq!(effective_phase(Phase::Holding, false), Phase::Holding);
        assert_eq!(effective_phase(Phase::Idle, false), Phase::Idle);
    }

    #[test]
    fn a_machine_that_was_authorized_once_does_not_ask_again_on_the_next_launch() {
        // Issue #55, as a decision. A macOS run used to start unauthorized every
        // time, because the old answer was `!needs_authorization` and nothing
        // outlived the process that had been elevated. What outlives it is the
        // grant on the machine, so a later launch that finds one starts holding
        // without putting a password prompt in front of anybody.
        let macos = |installed| Capabilities {
            hold: true,
            thermal: true,
            needs_authorization: true,
            authorization_installed: installed,
        };
        assert!(!starts_authorized(macos(false)), "first ever launch asks");
        assert!(
            starts_authorized(macos(true)),
            "every launch after does not"
        );
    }

    #[test]
    fn a_platform_with_nothing_to_authorize_never_consults_the_grant() {
        // Linux takes a logind inhibitor as the user and Windows writes a power
        // scheme the user already owns. Neither has anything to install, so
        // neither may be made to depend on finding it installed — a false here
        // must not be able to lock those platforms out of holding.
        for installed in [false, true] {
            assert!(starts_authorized(Capabilities {
                hold: true,
                thermal: false,
                needs_authorization: false,
                authorization_installed: installed,
            }));
        }
    }

    #[test]
    fn a_pending_nudge_wakes_the_sweep_at_once() {
        // The delay the user hit: a toggled trigger waited up to a full tick
        // before anything re-decided. A nudge that lands while the sweep is busy
        // (not yet waiting) must still be honoured — so the wait short-circuits
        // on a pending nudge rather than blocking for the timeout.
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("data")).unwrap();
        let handle = Handle::new(
            d.path().join("data"),
            d.path().join("home"),
            Capabilities {
                hold: true,
                thermal: false,
                needs_authorization: true,
                authorization_installed: false,
            },
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
        );

        handle.nudge();
        let start = std::time::Instant::now();
        handle.wait_for_sweep(Duration::from_secs(10));
        assert!(
            start.elapsed() < Duration::from_secs(2),
            "a pending nudge must not block: waited {:?}",
            start.elapsed()
        );
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
    fn a_machine_the_system_calls_too_hot_stops_holding() {
        // The guard that replaced the duration cap. Continuing here is the one
        // case where holding does damage rather than merely spending charge.
        let s = settings(Trigger::AgentActive);
        for hot in [Thermal::Serious, Thermal::Critical] {
            assert_eq!(
                decide(
                    &s,
                    &Inputs {
                        thermal: hot,
                        ..working()
                    }
                ),
                Phase::PausedTooHot,
                "{hot:?} must stop the hold"
            );
        }
    }

    #[test]
    fn a_thermal_guard_switched_off_keeps_holding_a_hot_machine() {
        // The point of offering the switch: the reading is the system's
        // judgement, and someone running a deliberately hot machine on a bench
        // may disagree with it.
        let s = Settings {
            thermal_guard: false,
            ..settings(Trigger::AgentActive)
        };
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    thermal: Thermal::Critical,
                    ..working()
                }
            ),
            Phase::Holding
        );
    }

    #[test]
    fn switching_the_thermal_guard_off_leaves_the_battery_guard_alone() {
        // Two independent guards. Turning one off must not quietly disarm the
        // other — that is the shape a "just disable the annoying one" setting
        // usually takes.
        let s = Settings {
            thermal_guard: false,
            ..settings(Trigger::AgentActive)
        };
        assert_eq!(
            decide(
                &s,
                &Inputs {
                    thermal: Thermal::Critical,
                    power: on_battery(5),
                    ..working()
                }
            ),
            Phase::PausedLowBattery
        );
    }

    #[test]
    fn the_thermal_guard_is_on_by_default_and_for_older_settings_files() {
        // Heat is the one guard where continuing does damage rather than
        // spending charge, so neither a fresh install nor an upgrade may arrive
        // with it silently off.
        assert!(Settings::default().thermal_guard);

        let d = tempfile::tempdir().unwrap();
        let before = d.path().join("older.json");
        std::fs::write(
            &before,
            br#"{"trigger":"always","battery_floor_percent":30}"#,
        )
        .unwrap();
        assert!(
            Settings::load(&before).thermal_guard,
            "a file written before this setting existed must read as on"
        );
    }

    #[test]
    fn a_warm_machine_keeps_holding() {
        // `Fair` is Apple's "slightly elevated, fans audible" — the ordinary
        // state of a laptop doing work. Releasing there would mean a hold that
        // never survives the build it was taken out for.
        let s = settings(Trigger::AgentActive);
        for fine in [Thermal::Nominal, Thermal::Fair, Thermal::Unknown] {
            assert_eq!(
                decide(
                    &s,
                    &Inputs {
                        thermal: fine,
                        ..working()
                    }
                ),
                Phase::Holding,
                "{fine:?} must not stop the hold"
            );
        }
    }

    #[test]
    fn heat_is_reported_ahead_of_a_low_battery() {
        // Both can be true at once and the window shows one. Heat wins: a user
        // told to plug in would carry a hot machine to a charger and keep it
        // working, which is the wrong instruction.
        let s = settings(Trigger::AgentActive);
        let both = Inputs {
            thermal: Thermal::Critical,
            power: on_battery(5),
            ..working()
        };
        assert_eq!(decide(&s, &both), Phase::PausedTooHot);
    }

    #[test]
    fn a_platform_that_cannot_read_heat_is_never_paused_for_it() {
        // Windows and Linux report `Unknown`. A missing reading is not evidence
        // of a hot machine, and treating it as one would make the feature
        // useless everywhere it cannot measure.
        assert!(!Thermal::Unknown.is_danger());
        assert!(!Thermal::Nominal.is_danger());
        assert!(!Thermal::Fair.is_danger());
        assert!(Thermal::Serious.is_danger());
        assert!(Thermal::Critical.is_danger());
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
    fn a_trigger_that_stops_asking_reports_idle_rather_than_a_guard() {
        // Honesty: "paused, low battery" while nothing wants a hold would have
        // the user plug in to fix a problem that does not exist.
        let s = settings(Trigger::AgentActive);
        let quiet = Inputs {
            agent_active: false,
            power: on_battery(2),
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
    fn a_hold_that_ends_forgets_its_clock() {
        // Otherwise the next agent inherits the last one's elapsed time and the
        // window reports a stretch that never happened.
        let mut sweep = Sweep::default();
        sweep.observe(Phase::Holding, Duration::from_secs(60));
        sweep.settle(Phase::Holding, None);
        sweep.observe(Phase::Holding, Duration::from_secs(60));
        sweep.settle(Phase::Holding, None);
        assert!(sweep.held_for > Duration::ZERO);

        sweep.observe(Phase::Idle, Duration::from_secs(15));
        assert_eq!(sweep.held_for, Duration::ZERO);
    }

    #[test]
    fn a_hold_that_failed_counts_no_time_as_held() {
        // `Holding` is what the app *asked* for; it is not evidence the machine
        // is awake. If every hold errors — no `systemd-inhibit` to spawn, a
        // power scheme that will not write — the machine is free to sleep, and
        // the clock the window shows as "held" must not grow through it. The
        // error hides the band today, but nothing resets the figure, so it
        // resurfaces as an hour of holding that never happened.
        let mut sweep = Sweep::default();
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, None);
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, Some("could not hold the machine awake"));
        let before = sweep.held_for;

        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        sweep.settle(Phase::Holding, Some("could not hold the machine awake"));
        assert_eq!(
            sweep.held_for, before,
            "a hold that failed held nothing, so it must not count as held"
        );

        // And a hold that takes again resumes from the honest figure: the
        // sweep that recovers credits nothing for the interval it spent
        // failing, and only the one after it starts adding again.
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, None);
        assert_eq!(sweep.held_for, before);
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, None);
        assert_eq!(sweep.held_for, before + Duration::from_secs(15));
    }

    #[test]
    fn a_pause_freezes_the_clock_without_resetting_it() {
        // The window shows this figure as "held", so a pause — which drops the
        // hold — must not add to it. It must not reset it either: plugging in
        // resumes the same stretch rather than starting a fresh one.
        let mut sweep = Sweep::default();
        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        sweep.settle(Phase::Holding, None);
        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        sweep.settle(Phase::Holding, None);

        // The sweep that decides to pause is also the one that releases, so the
        // interval before it was still genuinely held and is still credited.
        sweep.observe(Phase::PausedLowBattery, Duration::from_secs(15));
        sweep.settle(Phase::PausedLowBattery, None);
        let before = sweep.held_for;

        sweep.observe(Phase::PausedTooHot, Duration::from_secs(15));
        sweep.settle(Phase::PausedTooHot, None);
        assert_eq!(
            sweep.held_for, before,
            "a heat pause holds nothing, so it must not count as held"
        );
        sweep.observe(Phase::PausedLowBattery, Duration::from_secs(15));
        sweep.settle(Phase::PausedLowBattery, None);
        assert_eq!(
            sweep.held_for, before,
            "a battery pause holds nothing either"
        );

        // Coming back resumes the same stretch: the sweep that re-takes the
        // hold credits nothing for the paused interval it just ended.
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, None);
        assert_eq!(sweep.held_for, before);
        sweep.observe(Phase::Holding, Duration::from_secs(15));
        sweep.settle(Phase::Holding, None);
        assert_eq!(
            sweep.held_for,
            before + Duration::from_secs(15),
            "and the stretch continues from where the pause left it"
        );
    }

    fn handle_with(recovery: Recovery, root: &Path) -> Handle {
        Handle::new(
            root.to_path_buf(),
            root.join("home"),
            Capabilities {
                hold: true,
                thermal: true,
                needs_authorization: true,
                authorization_installed: false,
            },
            recovery,
        )
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
    fn restoring_sleep_disarms_the_trigger_and_drops_the_flag() {
        // The failure this prevents is invisible: the user presses **Restore
        // sleep**, the banner clears, and sleep is disabled again with nothing
        // on screen saying so.
        //
        // The tempting argument is that it cannot happen, because `stranded`
        // and `authorized` are mutually exclusive inside one `Handle`, so a
        // sweep during the banner writes a flag nothing is watching. That
        // reasons about a process; `disablesleep` is a machine. A second copy
        // of the app — `cargo tauri dev` beside the installed build, same data
        // root — starts by deleting the flag and breadcrumb and so reports a
        // stranded machine, while the first copy's root loop is still alive and
        // still polling that flag. It re-disabled sleep one poll after the
        // restore.
        //
        // So the flag is what this asserts on, not the fields: it is the only
        // channel to a loop this process did not start, and the loop is
        // edge-triggered on it existing. The previous version of this test
        // checked field transitions alone and stayed green through exactly
        // that failure.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: Some(0),
                stranded: true,
            },
            d.path(),
        );
        handle
            .set_settings(Settings {
                trigger: Trigger::AgentActive,
                ..Settings::default()
            })
            .unwrap();
        // As a sweep would have left it, and as a foreign watchdog reads it.
        apply(d.path(), true).unwrap();
        let flag = crate::paths::keep_awake_flag(d.path());
        assert!(flag.exists(), "the sweep's flag is the state being escaped");

        restore(
            &handle,
            &crate::shared_config::tests_support::FakePlatform::with_running(Vec::new()),
        )
        .unwrap();

        assert!(
            !flag.exists(),
            "a watchdog this process cannot see only lets go when the flag does"
        );
        let after = handle.status();
        assert_eq!(
            after.settings.trigger,
            Trigger::Off,
            "an armed trigger rewrites the flag on the next sweep"
        );
        assert!(!after.stranded, "the banner has served its purpose");
        // And it has to outlive the process, or the next launch holds again.
        assert_eq!(
            Settings::load(&crate::paths::keep_awake_settings(d.path())).trigger,
            Trigger::Off
        );
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
            Thermal::Nominal,
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
            Thermal::Nominal,
            Duration::ZERO,
            Some("disk full".into()),
        );
        assert!(handle.status().hold_error.is_some());

        handle.publish(
            Phase::Holding,
            Vec::new(),
            power,
            Thermal::Nominal,
            Duration::ZERO,
            None,
        );
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
            thermal_guard: true,
        };
        let sane = wild.clamped();
        assert_eq!(sane.idle_window_minutes, 1);
        assert_eq!(sane.battery_floor_percent, 95);

        let huge = Settings {
            trigger: Trigger::Always,
            idle_window_minutes: 9999,
            battery_floor_percent: 200,
            thermal_guard: true,
        };
        let sane = huge.clamped();
        assert_eq!(sane.idle_window_minutes, 60);
        assert_eq!(sane.battery_floor_percent, 95);
    }

    #[test]
    fn settings_survive_a_round_trip_to_disk() {
        let d = tempfile::tempdir().unwrap();
        let file = d.path().join("keep-awake.json");
        let written = Settings {
            trigger: Trigger::AgentActive,
            idle_window_minutes: 7,
            battery_floor_percent: 40,
            thermal_guard: false,
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
    }

    /// A platform whose only job is to record the `hold` calls made through it.
    ///
    /// The default `Platform::hold` writes the flag file, and so does
    /// `shared_config::tests_support::FakePlatform` — but the one backend this
    /// fix is *for*, Windows, overrides `hold` and never touches that flag.
    /// Asserting a flag disappeared therefore proves the wrong backend released,
    /// on a platform where the leak never happened. This records the calls
    /// instead, so a test can assert the release actually went through the
    /// `Platform` contract every backend implements.
    #[derive(Default)]
    struct RecordingHold {
        calls: Mutex<Vec<bool>>,
        /// Make every `hold` write fail, so a test can drive the case the clock
        /// has to answer to: the app asked for a hold and the platform refused.
        fails: bool,
    }

    impl crate::platform::Platform for RecordingHold {
        fn declared_here(&self, _locations: &crate::app_spec::Locations) -> bool {
            true
        }
        fn data_root(&self) -> Result<PathBuf> {
            unimplemented!()
        }
        fn default_profile_dir(&self, _locations: &crate::app_spec::Locations) -> Result<PathBuf> {
            unimplemented!()
        }
        fn binary(
            &self,
            _locations: &crate::app_spec::Locations,
            _product: &str,
        ) -> Result<PathBuf> {
            unimplemented!()
        }
        fn process_marker(&self, _locations: &crate::app_spec::Locations) -> Result<String> {
            unimplemented!()
        }
        fn scan(
            &self,
            _targets: &[crate::platform::ScanTarget],
        ) -> Result<Vec<crate::platform::RunningProcess>> {
            unimplemented!()
        }
        fn link(&self, _source: &Path, _target: &Path) -> Result<()> {
            unimplemented!()
        }
        fn focus(
            &self,
            _pid: i32,
            _hint: &crate::platform::FocusHint,
        ) -> Result<crate::platform::FocusOutcome> {
            unimplemented!()
        }
        fn quit(&self, _pid: i32) -> Result<()> {
            unimplemented!()
        }
        fn hold(&self, _data_root: &Path, on: bool) -> Result<()> {
            self.calls.lock().unwrap().push(on);
            if self.fails {
                return Err(anyhow::anyhow!("could not hold the machine awake"));
            }
            Ok(())
        }
    }

    #[test]
    fn quitting_hands_the_machine_back_but_keeps_the_trigger_armed() {
        // Windows holds by writing the lid-close action into the power scheme,
        // and that write outlives the process: without a release on the exit
        // path a quit leaves the lid doing nothing until the next launch.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        handle
            .set_settings(Settings {
                trigger: Trigger::Always,
                ..Settings::default()
            })
            .unwrap();

        let platform = RecordingHold::default();
        release_at_exit(&handle, &platform).unwrap();

        // The release went through the platform's own `hold`, not a flag file
        // some backends never write — this is the call every backend, Windows
        // included, implements, and it must have been asked to let go.
        assert_eq!(
            *platform.calls.lock().unwrap(),
            vec![false],
            "quitting has to release the hold through the platform itself"
        );
        // And the sweep is told to stop first, so a still-running iteration
        // cannot re-take the hold one tick after we handed the machine back —
        // the tray-Quit race the flag-file version of this test could not see.
        assert!(
            handle.is_stopping(),
            "the sweep must be stopped before release, or it re-arms the hold"
        );
        // Quitting is not "turn Keep Awake off": the next launch honours the
        // trigger the user chose, and only the OS-level hold is handed back.
        assert_eq!(handle.settings().trigger, Trigger::Always);
        assert_eq!(
            Settings::load(&crate::paths::keep_awake_settings(d.path())).trigger,
            Trigger::Always,
            "the armed trigger has to outlive the process, or the next launch \
             starts with the feature off"
        );
    }

    #[test]
    fn a_stopped_sweep_will_not_re_arm_a_released_hold() {
        // The guard that actually closes the exit-path re-arm race, driven end to
        // end rather than by poking the flag. A sweep can be mid-iteration when
        // `release_at_exit` stops it, and its next act would be a `hold(true)`
        // that rewrites the Windows lid action to "do nothing" one instant after
        // the exit path handed the machine back. `hold_step` is that act, with
        // the `is_stopping` guard immediately before the write — so a stopped
        // sweep must reach the platform's `hold` not at all.
        //
        // This asserts on the calls that reach the platform, not on the flag's
        // getter: the previous version of this test only checked
        // `stop_sweeping()` set `is_stopping()`, so deleting the in-loop guard
        // left it green while the race it names was wide open.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();
        assert!(!handle.is_stopping(), "a live app must let the sweep run");

        handle.stop_sweeping();
        assert!(handle.is_stopping(), "the sweep must see the stop");

        // Drive the extracted step exactly as the sweep would, asking to hold —
        // the worst case, the one that re-arms. The guard must turn it into a
        // no-op that ends the loop instead.
        let mut sweep = Sweep::default();
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Stopped => {}
            HoldStep::Skipped | HoldStep::Applied(_) => {
                panic!("a stopped sweep must end the loop, not reach the hold write")
            }
        }
        assert!(
            platform.calls.lock().unwrap().is_empty(),
            "no hold call may reach the platform after stop_sweeping — the sweep \
             must not re-arm the hold the exit path just released"
        );
        // And the clock was told, on this path too: a step that wrote nothing
        // held nothing, so nothing after it may be credited as held.
        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        assert_eq!(
            sweep.held_for,
            Duration::ZERO,
            "a step that never reached the hold write must not leave the clock \
             believing the machine is held"
        );
    }

    #[test]
    fn the_step_that_holds_is_the_step_that_settles_the_clock() {
        // The invariant this test exists to hold down is not "`settle` computes
        // the right boolean" — the `Sweep` tests already cover that. It is that
        // *nothing between the hold write and the next sweep can forget to tell
        // the clock what happened*. When `settle` was a separate line in `watch`,
        // deleting it, reordering it, or jumping it with a `continue` left `held`
        // stale and the whole suite green: `watch` takes a `tauri::AppHandle` and
        // no test can construct one. Settling inside `hold_step` puts the
        // invariant under a function tests can drive, which is the same treatment
        // and the same reason the `is_stopping` guard was pulled in here.
        //
        // So this drives `hold_step` exactly as the sweep does and asserts on the
        // clock afterwards, never on `settle` directly.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let mut sweep = Sweep::default();

        // A hold the platform took: the interval after it is genuinely held, so
        // the next sweep credits it.
        let taken = RecordingHold::default();
        match hold_step(&handle, &taken, Phase::Holding, &mut sweep) {
            HoldStep::Applied(None) => {}
            HoldStep::Applied(Some(error)) => panic!("the hold should have taken: {error}"),
            HoldStep::Stopped | HoldStep::Skipped => {
                panic!("a live, unpaused app must reach the hold write")
            }
        }
        sweep.observe(Phase::Holding, Duration::from_secs(60));
        assert_eq!(
            sweep.held_for,
            Duration::from_secs(60),
            "a hold the platform took has to start the clock — if it does not, \
             the step is not settling at all and the next assertion proves \
             nothing"
        );

        // A hold the platform refused: the phase still says `Holding`, but the
        // machine is free to sleep, so the interval after it counts for nothing.
        // This is #41 exactly, reached through the production call path.
        let refused = RecordingHold {
            fails: true,
            ..Default::default()
        };
        match hold_step(&handle, &refused, Phase::Holding, &mut sweep) {
            HoldStep::Applied(Some(_)) => {}
            HoldStep::Applied(None) => panic!("the hold should have failed"),
            HoldStep::Stopped | HoldStep::Skipped => {
                panic!("a live, unpaused app must reach the hold write")
            }
        }
        sweep.observe(Phase::Holding, Duration::from_secs(3600));
        assert_eq!(
            sweep.held_for,
            Duration::from_secs(60),
            "a hold that failed held nothing, so the hour after it must not be \
             counted as held — and no line outside `hold_step` may be what \
             makes that true"
        );
    }

    #[test]
    fn releasing_for_an_update_hands_the_machine_back_without_stopping_the_sweep() {
        // An update install is not necessarily an exit: the download and this
        // release can both succeed and then `install()` throw, and on macOS and
        // Linux the app is still running afterwards. `release_at_exit` would have
        // latched `stopping` and killed the sweep, leaving keep-awake dead until
        // a manual relaunch with nothing on screen saying so. `release_for_update`
        // hands the OS hold back but leaves the sweep alive, so a failed install
        // self-heals — the next sweep re-arms the hold from the trigger the user
        // still has set.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();

        release_for_update(&handle, &platform).unwrap();

        // The OS hold went back through the platform every backend implements.
        assert_eq!(
            *platform.calls.lock().unwrap(),
            vec![false],
            "the update path has to hand the OS hold back"
        );
        // But the sweep is untouched: a failed install must leave it able to
        // re-arm, which a latched `stopping` would forbid for the rest of the run.
        assert!(
            !handle.is_stopping(),
            "the update path must not stop the sweep — a failed install leaves \
             the app running and keep-awake must keep working"
        );
    }

    #[test]
    fn a_sweep_cannot_undo_the_update_handoff() {
        // The handoff is only worth making if it survives the next tick. The
        // sweep runs every fifteen seconds and the window between
        // `release_for_update` and the installer's `exit(0)` is easily longer
        // than that, so a sweep that re-armed here would write the Windows
        // lid-close action back to "do nothing" and then the process would end
        // with it stuck — exactly the handoff being undone.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();

        release_for_update(&handle, &platform).unwrap();

        // Drive the one step of a sweep that can re-take the hold, asking for
        // the worst case.
        let mut sweep = Sweep::default();
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Skipped => {}
            HoldStep::Stopped => panic!("the handoff must pause the sweep, not end it"),
            HoldStep::Applied(_) => {
                panic!("a sweep during the update handoff must not reach the hold write")
            }
        }
        assert_eq!(
            *platform.calls.lock().unwrap(),
            vec![false],
            "no hold call may reach the platform after the update handoff — the \
             sweep must not re-arm the hold the installer is about to outlive"
        );
    }

    #[test]
    fn the_handoff_is_not_banked_as_time_the_machine_was_held() {
        // #48. `release_for_update` hands the OS hold back and then the sweep
        // skips. Every skipped tick used to be credited to `held_for` anyway —
        // by two different routes, one from each of the PRs that met here — so
        // an `install()` that ran ninety seconds and threw left the window
        // saying "held 21m" for a machine that spent three of those minutes
        // free to sleep. Same class of lie as #41, and invisible, because
        // nothing publishes until the pause clears.
        //
        // Driven in the order `watch` runs: observe the interval just lived,
        // then take the step that decides and settles.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();
        let mut sweep = Sweep::default();

        // Twenty minutes of genuine holding first, so the assertion below is
        // about what the handoff adds rather than about a clock at zero.
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Applied(None) => {}
            _ => panic!("a live app must take the hold"),
        }
        sweep.observe(Phase::Holding, Duration::from_secs(20 * 60));
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Applied(None) => {}
            _ => panic!("a live app must take the hold"),
        }
        assert_eq!(sweep.held_for, Duration::from_secs(20 * 60));

        // The update starts: the hold goes back and the sweep is paused.
        release_for_update(&handle, &platform).unwrap();

        // Six sweeps pass while `install()` runs — ninety seconds.
        for tick in 0..6 {
            sweep.observe(Phase::Holding, SWEEP);
            match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
                HoldStep::Skipped => {}
                _ => panic!("sweep {tick} during the handoff must skip the hold write"),
            }
        }

        // Only the first of those ticks counts, and only because the interval it
        // covers began while the hold was still in place — the release happened
        // somewhere inside it. That is the same boundary the thermal and battery
        // pauses already have, and it is bounded by one `SWEEP`. Every tick
        // after it adds nothing, because the machine is genuinely free to sleep.
        assert_eq!(
            sweep.held_for,
            Duration::from_secs(20 * 60) + SWEEP,
            "the update handoff gave the OS hold back, so the install must not \
             be banked as time the machine was held — at most the one tick the \
             release happened inside"
        );
    }

    #[test]
    fn the_handoff_stops_the_window_claiming_a_hold_it_gave_back() {
        // While the sweep is skipping, nothing publishes, and `publish` is the
        // only writer of `phase`, `held_for_secs` and `hold_error`. So the Keep
        // Awake tab went on showing `Holding` and "held 42m" for the whole
        // install, for a machine whose lid was working normally.
        //
        // Re-publishing `Holding` would be the same lie. Saying nothing
        // preserves it. The truth is said at the moment it becomes true — in
        // `release_for_update`, not one sweep later.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();

        handle.publish(
            Phase::Holding,
            Vec::new(),
            Power {
                percent: Some(80),
                external: true,
            },
            crate::platform::Thermal::Unknown,
            Duration::from_secs(42 * 60),
            None,
        );
        assert_eq!(handle.status().phase, Phase::Holding);
        assert_eq!(handle.status().held_for_secs, 42 * 60);

        release_for_update(&handle, &platform).unwrap();

        let status = handle.status();
        assert_eq!(
            status.phase,
            Phase::Idle,
            "the handoff gave the hold back, so the window must not go on \
             saying the machine is being held"
        );
        assert_eq!(
            status.held_for_secs, 0,
            "a stretch of holding that has ended cannot go on being reported as \
             running"
        );
        // The readings the sweep owns are left alone: this says one true thing
        // about the hold, it does not fabricate a battery or thermal reading.
        assert_eq!(status.battery_percent, Some(80));
        assert!(status.on_external_power);
    }

    #[test]
    fn a_handoff_that_never_reports_back_gives_the_sweep_up_rather_than_keeps_it() {
        // The pause is cleared from exactly one production place: the `catch` in
        // `useUpdater`. If the webview dies, or `install()` never settles, that
        // `catch` never runs and the pause is set for the life of the process —
        // every sweep skips, the machine is never held again, and there is no
        // log line and no banner. That is keep-awake silently switched off for
        // the rest of the run, which is the state the non-latching design exists
        // to prevent.
        //
        // So the pause carries a deadline. Driven by taking the same pause with
        // no time left on it, rather than by sleeping through `UPDATE_HANDOFF`.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();
        let mut sweep = Sweep::default();

        release_for_update(&handle, &platform).unwrap();
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Skipped => {}
            _ => panic!("a fresh handoff still owns the sweep"),
        }

        // The same handoff, out of time. Nothing resumed it — no `catch` ran,
        // no command came back.
        handle.pause_sweeping_for(Duration::ZERO);

        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Applied(None) => {}
            HoldStep::Skipped => panic!(
                "an update handoff that never reported back must not keep the \
                 sweep off for the rest of the process"
            ),
            HoldStep::Stopped => panic!("the handoff must never stop the sweep"),
            HoldStep::Applied(Some(error)) => panic!("the hold should have taken: {error}"),
        }
        assert_eq!(
            *platform.calls.lock().unwrap(),
            vec![false, true],
            "once the handoff is out of time the sweep has to hold the machine \
             again, from the trigger the user still has set"
        );
        assert!(
            !handle.is_paused(),
            "an expired handoff must clear itself, not be re-tested every sweep"
        );
    }

    #[test]
    fn a_failed_install_leaves_keep_awake_working() {
        // The other half, and why this is a pause rather than a stop: the
        // install can throw after the handoff and on macOS and Linux the app is
        // still running. A latched stop would leave keep-awake dead for the rest
        // of the run with nothing on screen saying so, so the window clears the
        // pause in its `catch` and the very next sweep re-arms.
        let d = tempfile::tempdir().unwrap();
        let handle = handle_with(
            Recovery {
                reclaimed_prior: None,
                stranded: false,
            },
            d.path(),
        );
        let platform = RecordingHold::default();

        release_for_update(&handle, &platform).unwrap();
        resume_after_failed_update(&handle);

        assert!(
            !handle.is_stopping(),
            "a failed install must never leave the sweep stopped"
        );
        let mut sweep = Sweep::default();
        match hold_step(&handle, &platform, Phase::Holding, &mut sweep) {
            HoldStep::Applied(None) => {}
            _ => panic!("after a failed install the sweep must re-arm the hold"),
        }
        assert_eq!(
            *platform.calls.lock().unwrap(),
            vec![false, true],
            "a failed install has to leave keep-awake able to hold again"
        );
    }
}
