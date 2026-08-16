//! Compiled on every platform so its tests keep running, but the code that
//! calls these helpers is the Windows `Platform` impl, which exists only there.
#![cfg_attr(not(target_os = "windows"), allow(dead_code))]

use crate::app_spec::{WinPath, WinRoot};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

/// Turns an app's declared locations into real paths. Kept pure — the OS knows
/// what `%LOCALAPPDATA%` means, the app only knows what hangs off it.
pub fn expand(paths: &[WinPath], local: &Path, roaming: &Path) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|path| match path.root {
            WinRoot::Local => local.join(path.rest),
            WinRoot::Roaming => roaming.join(path.rest),
        })
        .collect()
}

pub fn pick_default_profile(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|candidate| candidate.is_dir())
        .cloned()
}

pub fn pick_binary(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates
        .iter()
        .find(|candidate| candidate.is_file())
        .cloned()
}

pub fn looked_in(candidates: &[PathBuf]) -> String {
    candidates
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn env_path(var: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var(var).map_err(|_| anyhow!("{var} is not set"))?,
    ))
}

/// What closing the lid did before this app took the setting over: the power
/// scheme's lid-close action on mains and on battery.
///
/// Two values because Windows keeps two, and restoring one of them would leave
/// a laptop that never sleeps on the lid on battery — the exact machine this
/// feature must not create.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LidPrior {
    pub ac: u32,
    pub dc: u32,
}

impl LidPrior {
    /// Written before the setting is changed, so a process killed between the
    /// two still leaves behind what to put back.
    pub fn render(self) -> String {
        format!("ac={},dc={}\n", self.ac, self.dc)
    }

    /// `None` for anything unreadable, and deliberately not a guess: the whole
    /// file exists to answer "what was this before", and inventing a value
    /// would hand the user a lid action they never chose. An unreadable file
    /// leaves the setting alone, which is recoverable; a wrong one is not.
    pub fn parse(raw: &str) -> Option<Self> {
        let mut ac = None;
        let mut dc = None;
        for field in raw.trim().split(',') {
            match field.trim().split_once('=') {
                Some(("ac", value)) => ac = value.parse().ok(),
                Some(("dc", value)) => dc = value.parse().ok(),
                _ => return None,
            }
        }
        Some(Self { ac: ac?, dc: dc? })
    }
}

/// The two bytes `GetSystemPowerStatus` fills in, as the battery guard reads
/// them.
///
/// Both use 255 for "unknown", and neither may be mistaken for a real value: a
/// charge of 255 is not a full battery, and it must not become a flat one
/// either. The API is the one every Windows battery indicator is built on — no
/// WMI query, no elevation, no dependency beyond a crate this build already
/// links for its file and window calls.
pub fn classify_power_status(ac_line: u8, battery_percent: u8) -> crate::platform::Power {
    crate::platform::Power {
        percent: (battery_percent <= 100).then_some(battery_percent),
        // 0 is offline, 1 online, 2 on a UPS, 255 unknown. Anything but a
        // definite offline counts as external, so an unreadable line status
        // leaves a hold alone rather than pausing a machine that is very
        // probably plugged in — and where it genuinely is not, the charge is
        // unreadable too and the guard has nothing to fire on regardless.
        external: ac_line != 0,
    }
}

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use crate::app_spec::Locations;
    use crate::platform::{
        win_proc, FocusHint, FocusOutcome, Platform, RunningProcess, ScanTarget, DATA_DIR_NAME,
    };

    pub struct Windows;

    fn roots() -> Result<(PathBuf, PathBuf)> {
        Ok((env_path("LOCALAPPDATA")?, env_path("APPDATA")?))
    }

    fn here<'a>(
        locations: &'a Locations,
        product: &str,
    ) -> Result<&'a crate::app_spec::WindowsLocation> {
        locations
            .windows
            .as_ref()
            .ok_or_else(|| anyhow!("{product} has not been declared for Windows"))
    }

    /// Whether Windows still has a process under this id.
    ///
    /// An unanswerable question is treated as "yes". Being unable to run
    /// `tasklist` means Windows itself is in a state we cannot reason about,
    /// and leaving an application that ignored the close request running
    /// forever is the worse of the two outcomes.
    fn still_running(pid: i32) -> bool {
        std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .map(|out| win_proc::still_listed(&String::from_utf8_lossy(&out.stdout), pid))
            .unwrap_or(true)
    }

    impl Platform for Windows {
        fn declared_here(&self, locations: &Locations) -> bool {
            locations.windows.is_some()
        }

        fn data_root(&self) -> Result<PathBuf> {
            Ok(env_path("APPDATA")?.join(DATA_DIR_NAME))
        }

        fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf> {
            let (local, roaming) = roots()?;
            let candidates = expand(
                here(locations, "this app")?.default_profiles,
                &local,
                &roaming,
            );
            pick_default_profile(&candidates).ok_or_else(|| {
                anyhow!(
                    "the app's data directory was not found. Looked in: {}",
                    looked_in(&candidates)
                )
            })
        }

        fn binary(&self, locations: &Locations, product: &str) -> Result<PathBuf> {
            let (local, roaming) = roots()?;
            let candidates = expand(here(locations, product)?.binaries, &local, &roaming);
            pick_binary(&candidates).ok_or_else(|| {
                anyhow!(
                    "{product} was not found. Looked in: {}",
                    looked_in(&candidates)
                )
            })
        }

        fn process_marker(&self, locations: &Locations) -> Result<String> {
            Ok(here(locations, "this app")?.process_name.to_string())
        }

        fn scan(&self, targets: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
            win_proc::scan(targets)
        }

        fn link(&self, source: &Path, target: &Path) -> Result<()> {
            std::fs::hard_link(source, target).map_err(|error| {
                anyhow!(
                    "could not link the shared config to {}: {error}. \
                     Both paths must be on the same drive.",
                    target.display()
                )
            })?;
            Ok(())
        }

        fn focus(&self, pid: i32, _hint: &FocusHint) -> Result<FocusOutcome> {
            // `BOOL` lives in windows-core as of the 0.61 reshuffle; only the
            // handle and message types stayed behind in Win32::Foundation.
            use windows::core::BOOL;
            use windows::Win32::Foundation::{HWND, LPARAM};
            use windows::Win32::UI::WindowsAndMessaging::{
                EnumWindows, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow,
            };

            struct Search {
                pid: u32,
                found: Option<HWND>,
            }

            unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
                let search = &mut *(lparam.0 as *mut Search);
                let mut owner = 0u32;
                GetWindowThreadProcessId(hwnd, Some(&mut owner));
                if owner == search.pid && IsWindowVisible(hwnd).as_bool() {
                    search.found = Some(hwnd);
                    return BOOL(0);
                }
                BOOL(1)
            }

            let mut search = Search {
                pid: pid as u32,
                found: None,
            };
            unsafe {
                let _ = EnumWindows(Some(callback), LPARAM(&mut search as *mut _ as isize));
            }

            match search.found {
                Some(hwnd) => {
                    let raised = unsafe { SetForegroundWindow(hwnd) }.as_bool();
                    if raised {
                        Ok(FocusOutcome::Focused)
                    } else {
                        Ok(FocusOutcome::Unsupported(
                            "Windows refused to bring the window forward; \
                             click its taskbar entry instead"
                                .into(),
                        ))
                    }
                }
                None => Ok(FocusOutcome::Unsupported(
                    "no visible window was found for this instance".into(),
                )),
            }
        }

        fn quit(&self, pid: i32) -> Result<()> {
            std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string()])
                .status()?;
            // Check before insisting. Sleeping out the grace period and then
            // force-killing unconditionally aims `/F` at a pid that Windows may
            // already have handed to something else entirely — and Windows
            // reuses process ids briskly.
            if crate::platform::waited_for_exit(
                || still_running(pid),
                || std::thread::sleep(crate::platform::QUIT_POLL),
            ) {
                return Ok(());
            }
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/F"])
                .status();
            Ok(())
        }

        fn power(&self) -> Result<crate::platform::Power> {
            use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

            let mut status = SYSTEM_POWER_STATUS::default();
            // SAFETY: the call writes into a struct this frame owns, and the
            // pointer cannot outlive it.
            unsafe { GetSystemPowerStatus(&mut status) }?;
            Ok(classify_power_status(
                status.ACLineStatus,
                status.BatteryLifePercent,
            ))
        }

        fn can_hold_awake(&self) -> bool {
            true
        }

        /// No prompt, and no privileged loop behind it. The lid-close action
        /// lives in the power scheme the signed-in user already owns, so this
        /// backend simply sets it and puts it back — where macOS has to ask for
        /// a password because `SleepDisabled` is root's.
        ///
        /// If a machine's policy does lock the scheme down, the write fails and
        /// the failure reaches the window through `hold_error` rather than
        /// being swallowed: a hold that did not happen must never be reported
        /// as one that did.
        fn needs_authorization(&self) -> bool {
            false
        }

        fn hold(&self, data_root: &Path, on: bool) -> Result<()> {
            let record = crate::paths::keep_awake_lid_prior(data_root);
            // Existence *is* the "we own it" bit. Reading the live lid action
            // instead would be ambiguous the moment a user's own choice happens
            // to be `Do nothing`.
            let owned = record.exists();

            match (on, owned) {
                (true, true) | (false, false) => Ok(()),
                (true, false) => {
                    let prior = lid::read()?;
                    if let Some(parent) = record.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    // Recorded before the change, never after. A process killed
                    // between these two lines leaves a machine whose lid does
                    // nothing and a file saying what it used to do; the reverse
                    // order leaves the same machine and no way back.
                    std::fs::write(&record, prior.render())?;
                    lid::write(lid::DO_NOTHING, lid::DO_NOTHING)
                }
                (false, true) => restore_lid(&record),
            }
        }

        fn recover_hold(&self, data_root: &Path) -> Result<()> {
            let record = crate::paths::keep_awake_lid_prior(data_root);
            if record.exists() {
                restore_lid(&record)?;
            }
            Ok(())
        }

        fn restore_sleep(&self) -> Result<()> {
            // The manual escape hatch. It should never be needed — a crash is
            // undone by `recover_hold` at the next launch, without asking — but
            // a user who has uninstalled the app and found their lid doing
            // nothing has no other way back.
            let data_root = self.data_root()?;
            self.recover_hold(&data_root)
        }

        // `thermal` is deliberately not implemented — see the note by
        // `classify_power_status`.
    }

    /// Puts the lid action back and forgets that we owned it.
    ///
    /// The record is removed only once the setting is actually restored: a
    /// delete-first order would, on a failed write, leave a machine whose lid
    /// does nothing and nothing anywhere saying so.
    fn restore_lid(record: &Path) -> Result<()> {
        let raw = std::fs::read_to_string(record)?;
        let Some(prior) = LidPrior::parse(&raw) else {
            // Unreadable, and there is no safe guess. Left in place rather than
            // deleted, so the next run tries again and the user is not quietly
            // stranded with a lid that does nothing.
            return Err(anyhow!(
                "{} does not say what the lid close action used to be",
                record.display()
            ));
        };
        lid::write(prior.ac, prior.dc)?;
        std::fs::remove_file(record)?;
        Ok(())
    }

    /// The lid-close action, read and written through `powrprof` rather than by
    /// driving `powercfg.exe`.
    ///
    /// `powercfg /q` prints its field names in the machine's display language,
    /// so a build that parsed them would work here and find nothing at all on a
    /// Japanese or German install. These calls take GUIDs and hand back a
    /// number, and there is nothing in between to translate.
    mod lid {
        use anyhow::{anyhow, Result};
        use windows::core::GUID;
        use windows::Win32::Foundation::{LocalFree, ERROR_SUCCESS, HLOCAL};
        use windows::Win32::System::Power::{
            PowerGetActiveScheme, PowerReadACValueIndex, PowerReadDCValueIndex,
            PowerSetActiveScheme, PowerWriteACValueIndex, PowerWriteDCValueIndex,
        };

        /// `powercfg`'s SUB_BUTTONS and LIDACTION. Fixed by Windows, so they
        /// are spelled out rather than discovered.
        const SUB_BUTTONS: GUID = GUID::from_u128(0x4f971e89_eebd_4455_a8de_9e59040e7347);
        const LID_ACTION: GUID = GUID::from_u128(0x5ca83367_6e45_459f_a27b_476b1d01c936);
        /// Index 0 of the lid-close action: "Do nothing". 1 is Sleep, 2
        /// Hibernate, 3 Shut down — which is why the prior value is recorded
        /// rather than assumed.
        pub const DO_NOTHING: u32 = 0;

        /// The active scheme's GUID, freed when it goes out of scope:
        /// `PowerGetActiveScheme` allocates and hands ownership over.
        struct ActiveScheme(*mut GUID);

        impl Drop for ActiveScheme {
            fn drop(&mut self) {
                // SAFETY: the pointer came from `PowerGetActiveScheme`, which
                // documents `LocalFree` as the way to release it, and nothing
                // else ever frees it.
                unsafe { LocalFree(Some(HLOCAL(self.0 as *mut core::ffi::c_void))) };
            }
        }

        /// Read every time rather than cached: someone who switches to Battery
        /// Saver mid-session has a different scheme, and writing the lid action
        /// into the one they left would change a plan they are not using while
        /// leaving the live one asleep on the lid.
        fn active_scheme() -> Result<ActiveScheme> {
            let mut guid: *mut GUID = std::ptr::null_mut();
            // SAFETY: the call writes one pointer into a local this frame owns.
            let status = unsafe { PowerGetActiveScheme(None, &mut guid) };
            if status != ERROR_SUCCESS || guid.is_null() {
                return Err(anyhow!(
                    "could not read the active power scheme (error {})",
                    status.0
                ));
            }
            Ok(ActiveScheme(guid))
        }

        /// What closing the lid does right now, on mains and on battery.
        pub fn read() -> Result<super::LidPrior> {
            let scheme = active_scheme()?;
            let (mut ac, mut dc) = (0u32, 0u32);
            // SAFETY: every pointer here borrows a local that outlives the
            // call, and the scheme GUID is alive until `scheme` drops.
            let (read_ac, read_dc) = unsafe {
                (
                    PowerReadACValueIndex(
                        None,
                        Some(scheme.0 as *const GUID),
                        Some(&SUB_BUTTONS),
                        Some(&LID_ACTION),
                        &mut ac,
                    ),
                    PowerReadDCValueIndex(
                        None,
                        Some(scheme.0 as *const GUID),
                        Some(&SUB_BUTTONS),
                        Some(&LID_ACTION),
                        &mut dc,
                    ),
                )
            };
            if read_ac != ERROR_SUCCESS || read_dc != ERROR_SUCCESS.0 {
                return Err(anyhow!(
                    "could not read the lid close action (errors {}, {read_dc})",
                    read_ac.0
                ));
            }
            Ok(super::LidPrior { ac, dc })
        }

        /// Sets the lid action on mains and on battery, and makes it take
        /// effect.
        ///
        /// The `PowerSetActiveScheme` at the end is not redundant: a written
        /// value sits in the registry until the scheme is applied again, so
        /// without it the machine would go on doing exactly what it did before
        /// while every reading said otherwise.
        pub fn write(ac: u32, dc: u32) -> Result<()> {
            let scheme = active_scheme()?;
            // SAFETY: as above — the GUIDs outlive the calls.
            let (wrote_ac, wrote_dc, applied) = unsafe {
                (
                    PowerWriteACValueIndex(
                        None,
                        scheme.0 as *const GUID,
                        Some(&SUB_BUTTONS),
                        Some(&LID_ACTION),
                        ac,
                    ),
                    PowerWriteDCValueIndex(
                        None,
                        scheme.0 as *const GUID,
                        Some(&SUB_BUTTONS),
                        Some(&LID_ACTION),
                        dc,
                    ),
                    PowerSetActiveScheme(None, Some(scheme.0 as *const GUID)),
                )
            };
            if wrote_ac != ERROR_SUCCESS || wrote_dc != ERROR_SUCCESS.0 || applied != ERROR_SUCCESS
            {
                return Err(anyhow!(
                    "could not set the lid close action (errors {}, {wrote_dc}, {}) — a policy on \
                     this machine may be holding the power scheme",
                    wrote_ac.0,
                    applied.0
                ));
            }
            Ok(())
        }
    }
}

#[cfg(target_os = "windows")]
pub use imp::Windows;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_spec;

    #[test]
    fn declared_locations_expand_against_the_right_environment_root() {
        let local = Path::new(r"C:\Users\h\AppData\Local");
        let roaming = Path::new(r"C:\Users\h\AppData\Roaming");
        let expanded = expand(
            app_spec::CLAUDE
                .locations
                .windows
                .as_ref()
                .unwrap()
                .default_profiles,
            local,
            roaming,
        );
        assert!(expanded[0].starts_with(local));
        assert!(expanded[1].starts_with(roaming));
    }

    #[test]
    fn an_app_declared_for_windows_says_where_to_look() {
        // An empty candidate list would report "not found. Looked in: " with
        // nothing after it, which tells the user precisely nothing. Apps not
        // declared for Windows are skipped: absent is not the same as empty.
        for spec in app_spec::all() {
            let Some(windows) = spec.locations.windows.as_ref() else {
                continue;
            };
            assert!(
                !windows.binaries.is_empty(),
                "{} declares no Windows binary",
                spec.id
            );
            assert!(
                !windows.default_profiles.is_empty(),
                "{} declares no Windows profile directory",
                spec.id
            );
        }
    }

    #[test]
    fn the_first_existing_candidate_wins() {
        let d = tempfile::tempdir().unwrap();
        let msix = d.path().join("msix");
        let classic = d.path().join("classic");
        std::fs::create_dir_all(&classic).unwrap();

        assert_eq!(
            pick_default_profile(&[msix.clone(), classic.clone()]),
            Some(classic)
        );

        std::fs::create_dir_all(&msix).unwrap();
        assert_eq!(
            pick_default_profile(&[msix.clone(), d.path().join("classic")]),
            Some(msix)
        );
    }

    #[test]
    fn no_existing_candidate_yields_none() {
        assert_eq!(pick_default_profile(&[PathBuf::from("/nope")]), None);
        assert_eq!(pick_binary(&[PathBuf::from("/nope/claude.exe")]), None);
    }

    #[test]
    fn a_binary_candidate_must_be_a_file_not_a_directory() {
        let d = tempfile::tempdir().unwrap();
        let dir_named_like_exe = d.path().join("claude.exe");
        std::fs::create_dir_all(&dir_named_like_exe).unwrap();
        assert_eq!(pick_binary(&[dir_named_like_exe]), None);
    }

    #[test]
    fn the_failure_message_lists_every_place_that_was_tried() {
        let looked = looked_in(&[PathBuf::from(r"C:\a"), PathBuf::from(r"C:\b")]);
        assert_eq!(looked, r"C:\a, C:\b");
    }

    #[test]
    fn the_recorded_lid_action_survives_a_round_trip() {
        let prior = LidPrior { ac: 1, dc: 2 };
        assert_eq!(LidPrior::parse(&prior.render()), Some(prior));
    }

    #[test]
    fn mains_and_battery_are_never_swapped() {
        // The pair this file exists to keep straight. Restored the wrong way
        // round, a laptop hibernates on the lid at the desk and stays awake in
        // the bag — both of them settings the user never chose.
        let parsed = LidPrior::parse("ac=0,dc=3").unwrap();
        assert_eq!(parsed.ac, 0);
        assert_eq!(parsed.dc, 3);
    }

    #[test]
    fn a_record_that_does_not_say_both_values_is_refused_rather_than_guessed() {
        // A half-written file must not restore half a setting: the caller keeps
        // it and tries again instead, which is recoverable. Inventing the
        // missing half is not.
        assert_eq!(LidPrior::parse("ac=1"), None);
        assert_eq!(LidPrior::parse(""), None);
        assert_eq!(LidPrior::parse("ac=1,dc=x"), None);
        assert_eq!(LidPrior::parse("lid=1,dc=1"), None);
    }

    #[test]
    fn a_laptop_on_battery_reports_its_charge() {
        let power = classify_power_status(0, 42);
        assert_eq!(power.percent, Some(42));
        assert!(!power.external);
    }

    #[test]
    fn a_plugged_in_machine_never_trips_the_battery_guard() {
        assert!(classify_power_status(1, 5).external);
    }

    #[test]
    fn the_unknown_sentinel_is_never_read_as_a_charge() {
        // 255 is the API's "no answer". Taken literally it is over 100%, and
        // the mistake worth guarding is the other direction — a build that
        // clamped it to a number would hand the guard a reading it would then
        // act on.
        assert_eq!(classify_power_status(255, 255).percent, None);
        assert!(
            classify_power_status(255, 255).external,
            "an unreadable line status must leave the hold alone, not pause it"
        );
    }

    #[test]
    fn a_desktop_reports_no_charge_rather_than_a_flat_one() {
        // No battery is 128 in `BatteryFlag`, and 255 in the percentage.
        assert_eq!(classify_power_status(1, 255).percent, None);
    }
}
