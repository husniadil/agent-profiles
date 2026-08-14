//! The OS axis. Nothing in here names a particular application: an app arrives
//! as an `AppSpec` to be interpreted, never as a branch.

use crate::app_spec::{AppSpec, Locations};
use anyhow::Result;
use std::path::{Path, PathBuf};

#[cfg(any(unix, test))]
pub mod unix_ps;
/// Gated like the `windows` backend that consumes it. Compiled under `test` too,
/// so its parser keeps being exercised on every platform's test run.
#[cfg(any(target_os = "windows", test))]
pub mod win_proc;

#[cfg(any(target_os = "linux", test))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(any(target_os = "windows", test))]
mod windows;

/// Our own directory name, where user-visible (macOS, Windows).
///
/// Linux reads the slug below instead, because the convention there is a lower
/// case directory under `.config` rather than a display name. That makes this
/// the one platform where the constant is genuinely unread, so the allow is
/// narrowed to it: anywhere else, going unused would be a real mistake worth
/// hearing about.
#[cfg_attr(target_os = "linux", allow(dead_code))]
pub const DATA_DIR_NAME: &str = "Agent Profiles";
/// Our own directory name, where a slug is conventional (Linux, window classes).
pub const DATA_DIR_SLUG: &str = "agent-profiles";

/// One live process, already attributed to the app that owns it.
#[derive(Debug, Clone, PartialEq)]
pub struct RunningProcess {
    pub app_id: &'static str,
    pub pid: i32,
    /// `None` means the process carries no designation, i.e. it is running the
    /// app's own stock profile.
    pub profile_dir: Option<PathBuf>,
}

/// What a scan is looking for, per app. Built once per sweep so that every app
/// is found in a single pass over the process table rather than one pass each.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanTarget {
    pub app_id: &'static str,
    /// Matched as a substring of the process's command.
    pub marker: String,
    /// The argument prefix carrying the profile directory.
    pub flag: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FocusOutcome {
    Focused,
    /// Only Linux ever returns this, for Wayland's refusal to let one app raise
    /// another's window. The other backends compile it away, hence the allow —
    /// it is unreachable on this platform, not unused in the codebase.
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    Unsupported(String),
}

/// Identifies a window without naming a process. Linux locates one by the
/// `--class` it was launched with; the others go by pid and ignore it, which is
/// what the allow is for.
pub struct FocusHint<'a> {
    #[allow(dead_code)]
    pub wm_class: &'a str,
}

pub trait Platform: Send + Sync {
    /// Whether this app has been declared for the platform we are running on.
    ///
    /// `false` means nobody has checked it here, so it is absent rather than
    /// broken: it contributes no tray section and no directory, and the user is
    /// never shown an app this build cannot honestly launch.
    fn declared_here(&self, locations: &Locations) -> bool;

    /// Where this application keeps every app's profiles. One level above the
    /// per-app roots, which are `data_root()/<app id>`.
    fn data_root(&self) -> Result<PathBuf>;

    /// The app's own stock profile directory — the one it uses when launched
    /// with no designation at all.
    fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf>;

    /// The executable to spawn, or an error naming where we looked.
    fn binary(&self, locations: &Locations, product: &str) -> Result<PathBuf>;

    /// The substring that identifies this app in the process table.
    ///
    /// Fallible because the honest answer for an app not declared here is "there
    /// isn't one" — and the tempting default, an empty string, is a substring of
    /// every line of the process table. That would attribute the first process
    /// on the machine to this app, report every profile as running, and aim the
    /// tray's Quit row at a stranger.
    fn process_marker(&self, locations: &Locations) -> Result<String>;

    /// One sweep of the process table, covering every target at once.
    fn scan(&self, targets: &[ScanTarget]) -> Result<Vec<RunningProcess>>;

    /// Make `target` refer to the same underlying file as `source`.
    fn link(&self, source: &Path, target: &Path) -> Result<()>;

    fn focus(&self, pid: i32, hint: &FocusHint) -> Result<FocusOutcome>;

    /// Ending a running instance.
    ///
    /// Nothing in the shipping binary calls this any more: the tray used to
    /// offer a `Quit <profile>` row beside each running profile and no longer
    /// does — quitting an application belongs to that application, not to a
    /// menu opened to switch between profiles. It stays on the contract because
    /// the verification harness and the probe both launch real applications and
    /// have to be able to put them away again.
    #[cfg_attr(not(test), allow(dead_code))]
    fn quit(&self, pid: i32) -> Result<()>;

    /// Extra arguments this OS needs on every launch, whatever the app.
    fn os_launch_args(&self, _wm_class: &str) -> Vec<String> {
        Vec::new()
    }

    /// Give a profile its own desktop-level identity. A no-op off Linux.
    fn register_identity(
        &self,
        _spec: &AppSpec,
        _profile_label: &str,
        _wm_class: &str,
    ) -> Result<()> {
        Ok(())
    }

    fn unregister_identity(&self, _wm_class: &str) -> Result<()> {
        Ok(())
    }
}

/// The window class, and Linux desktop-entry name, for one profile of one app.
/// Keyed on ids rather than labels because two profiles may be labelled alike.
pub fn wm_class(app_id: &str, profile_id: &str) -> String {
    format!("{DATA_DIR_SLUG}-{app_id}-{profile_id}")
}

/// Finds the pid running `profile_dir` for `app_id`, if any.
///
/// The stock profile is the process carrying no designation at all, which is
/// why `is_default` cannot be inferred from the path.
pub fn find_for(
    processes: &[RunningProcess],
    app_id: &str,
    profile_dir: &Path,
    is_default: bool,
) -> Option<i32> {
    processes
        .iter()
        .find(|process| {
            process.app_id == app_id
                && match (&process.profile_dir, is_default) {
                    (None, true) => true,
                    (Some(dir), false) => dir == profile_dir,
                    _ => false,
                }
        })
        .map(|process| process.pid)
}

pub fn current() -> Box<dyn Platform> {
    #[cfg(target_os = "macos")]
    return Box::new(macos::MacOs);
    #[cfg(target_os = "linux")]
    return Box::new(linux::Linux);
    #[cfg(target_os = "windows")]
    return Box::new(windows::Windows);
}

/// How long an application is given to close on its own before we insist.
#[cfg_attr(not(test), allow(dead_code))]
const QUIT_GRACE: std::time::Duration = std::time::Duration::from_secs(10);
/// How often it is asked whether it has gone.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) const QUIT_POLL: std::time::Duration = std::time::Duration::from_millis(100);

/// Waits out the grace period, reporting whether the process left on its own.
///
/// The answer is what licenses a force-kill, and the two questions are not the
/// same: "the grace period elapsed" says nothing about whether the process is
/// still there, and an operating system hands a pid to the next process that
/// asks. Escalating on the clock alone therefore aims a kill at whatever
/// inherited the number.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) fn waited_for_exit(
    mut still_alive: impl FnMut() -> bool,
    mut wait: impl FnMut(),
) -> bool {
    let attempts = QUIT_GRACE.as_millis() / QUIT_POLL.as_millis();
    for _ in 0..attempts {
        wait();
        if !still_alive() {
            return true;
        }
    }
    false
}

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
pub fn unix_signal_quit(pid: i32) -> Result<()> {
    unsafe { libc::kill(pid, libc::SIGTERM) };
    let gone = waited_for_exit(
        || unsafe { libc::kill(pid, 0) } == 0,
        || std::thread::sleep(QUIT_POLL),
    );
    if !gone {
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn processes() -> Vec<RunningProcess> {
        vec![
            RunningProcess {
                app_id: "claude",
                pid: 1,
                profile_dir: Some(PathBuf::from("/p/work")),
            },
            RunningProcess {
                app_id: "claude",
                pid: 2,
                profile_dir: None,
            },
            RunningProcess {
                app_id: "codex",
                pid: 3,
                profile_dir: Some(PathBuf::from("/p/work")),
            },
            RunningProcess {
                app_id: "codex",
                pid: 4,
                profile_dir: None,
            },
        ]
    }

    #[test]
    fn a_profile_matches_only_its_exact_directory() {
        let p = processes();
        assert_eq!(
            find_for(&p, "claude", &PathBuf::from("/p/work"), false),
            Some(1)
        );
        assert_eq!(
            find_for(&p, "claude", &PathBuf::from("/p/none"), false),
            None
        );
    }

    #[test]
    fn two_apps_sharing_a_profile_path_are_never_confused() {
        // Nothing stops a user pointing two apps at similarly named folders, and
        // the pid returned must still be the one belonging to the app asked for.
        let p = processes();
        assert_eq!(
            find_for(&p, "claude", &PathBuf::from("/p/work"), false),
            Some(1)
        );
        assert_eq!(
            find_for(&p, "codex", &PathBuf::from("/p/work"), false),
            Some(3)
        );
    }

    #[test]
    fn the_stock_profile_matches_the_process_with_no_designation() {
        let p = processes();
        assert_eq!(
            find_for(&p, "claude", &PathBuf::from("/ignored"), true),
            Some(2)
        );
        assert_eq!(
            find_for(&p, "codex", &PathBuf::from("/ignored"), true),
            Some(4)
        );
    }

    #[test]
    fn an_app_with_nothing_running_reports_nothing() {
        let only_claude = vec![RunningProcess {
            app_id: "claude",
            pid: 9,
            profile_dir: None,
        }];
        assert_eq!(
            find_for(&only_claude, "codex", &PathBuf::from("/x"), true),
            None
        );
    }

    #[test]
    fn a_process_that_leaves_on_its_own_is_never_escalated_against() {
        let mut polls = 0;
        // Gone on the second look, long before the grace period is up.
        let gone = waited_for_exit(
            || {
                polls += 1;
                polls < 2
            },
            || {},
        );
        assert!(gone, "an exit inside the grace period must be noticed");
        assert_eq!(polls, 2, "polling must stop the moment it has its answer");
    }

    #[test]
    fn a_process_that_outlives_the_grace_period_reports_that_it_did() {
        let mut polls = 0;
        let gone = waited_for_exit(
            || {
                polls += 1;
                true
            },
            || {},
        );
        assert!(!gone);
        assert_eq!(
            polls,
            (QUIT_GRACE.as_millis() / QUIT_POLL.as_millis()) as usize,
            "the whole grace period must be spent before insisting"
        );
    }

    #[test]
    fn a_window_class_is_unique_per_app_and_profile() {
        // Same profile id under two apps must not collide, or focusing one would
        // raise the other's window.
        assert_ne!(wm_class("claude", "abc"), wm_class("codex", "abc"));
        assert_eq!(wm_class("claude", "abc"), "agent-profiles-claude-abc");
    }
}
