use crate::app_spec::{self, AppSpec};
use crate::paths::Paths;
use crate::platform::Platform;
use crate::profile_store::{Profile, ProfileStore};
use crate::tray::MenuSignature;
use anyhow::{anyhow, Result};
use std::sync::Mutex;

/// One app's live state: what it is, where its profiles live, and what they are.
pub struct AppRuntime {
    pub spec: &'static AppSpec,
    pub paths: Paths,
    pub store: Mutex<ProfileStore>,
}

pub struct AppState {
    pub platform: Box<dyn Platform>,
    pub apps: Vec<AppRuntime>,
    /// What the tray menu currently shows. Replacing an attached menu closes it
    /// if it happens to be open, so we only replace it when it would differ.
    pub last_menu: Mutex<Option<Vec<MenuSignature>>>,
    /// Keeping the machine awake while an agent works. Present on every
    /// platform; inert where [`Platform::can_hold_awake`] is false.
    pub keep_awake: crate::keep_awake::Handle,
    pub general: crate::general::Handle,
}

impl AppState {
    pub fn app(&self, app_id: &str) -> Result<&AppRuntime> {
        self.apps
            .iter()
            .find(|runtime| runtime.spec.id == app_id)
            .ok_or_else(|| anyhow!("no app with id {app_id}"))
    }

    pub fn profile(&self, app_id: &str, profile_id: &str) -> Result<(&AppRuntime, Profile)> {
        let runtime = self.app(app_id)?;
        let store = runtime
            .store
            .lock()
            .map_err(|_| anyhow!("the profile store for {app_id} is unavailable"))?;
        let profile = store
            .get(profile_id)
            .cloned()
            .ok_or_else(|| anyhow!("no profile with id {profile_id}"))?;
        Ok((runtime, profile))
    }

    /// Whether this app is installed, and the reason if not.
    ///
    /// Resolved on every use rather than cached: an app's binary can come and go
    /// while this runs, so re-checking greys or un-greys its section without a
    /// restart. This only covers apps `build` already listed. An app whose stock
    /// data directory was absent at startup has no runtime at all until the next
    /// launch (see `build`) — on macOS that never happens, on Windows it is the
    /// not-installed case.
    pub fn availability(&self, runtime: &AppRuntime) -> Option<String> {
        self.platform
            .binary(&runtime.spec.locations, runtime.spec.product)
            .err()
            .map(|error| error.to_string())
    }
}

/// Builds one runtime per app declared *and* installed on this platform.
///
/// An app declared here but not installed has no stock data directory to point
/// its Default profile at, so it is skipped rather than aborting startup for
/// every other app — it appears the next time this launches with the app
/// present. A missing app must never be fatal: a user with only one of the
/// declared apps still gets that one. An app not declared here is skipped
/// earlier still: nobody has checked this platform for it.
pub fn build(platform: &dyn Platform) -> Result<Vec<AppRuntime>> {
    let root = platform.data_root()?;
    let mut runtimes = Vec::new();
    for spec in app_spec::all()
        .iter()
        .filter(|spec| platform.declared_here(&spec.locations))
    {
        // Not installed: no stock directory to adopt as Default. Skip it — this
        // is the difference between "you don't have ChatGPT" and "Agent Profiles
        // won't start", and only the first is acceptable.
        let Ok(default_dir) = platform.default_profile_dir(&spec.locations) else {
            continue;
        };
        let paths = Paths::new(root.join(spec.id));
        let store = ProfileStore::load(&paths, &default_dir)?;
        runtimes.push(AppRuntime {
            spec,
            paths,
            store: Mutex::new(store),
        });
    }
    Ok(runtimes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_spec::Locations;
    use crate::platform::{FocusHint, FocusOutcome, RunningProcess, ScanTarget};
    use std::path::{Path, PathBuf};

    /// A platform whose roots live under a temporary directory, so `build` can
    /// be exercised without touching a real home directory.
    struct SandboxPlatform {
        root: PathBuf,
    }

    impl Platform for SandboxPlatform {
        fn declared_here(&self, locations: &Locations) -> bool {
            locations.macos.is_some()
        }
        fn data_root(&self) -> Result<PathBuf> {
            Ok(self.root.join("data"))
        }
        fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf> {
            Ok(self
                .root
                .join("home")
                .join(locations.macos.as_ref().unwrap().default_profile))
        }
        fn binary(&self, _locations: &Locations, product: &str) -> Result<PathBuf> {
            Err(anyhow!("{product} was not found"))
        }
        fn process_marker(&self, locations: &Locations) -> Result<String> {
            Ok(locations.macos.as_ref().unwrap().binary.to_string())
        }
        fn scan(&self, _targets: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
            Ok(Vec::new())
        }
        fn link(&self, _source: &Path, _target: &Path) -> Result<()> {
            Ok(())
        }
        fn focus(&self, _pid: i32, _hint: &FocusHint) -> Result<FocusOutcome> {
            Ok(FocusOutcome::Focused)
        }
        fn quit(&self, _pid: i32) -> Result<()> {
            Ok(())
        }
    }

    fn state(root: &Path) -> AppState {
        let platform = SandboxPlatform {
            root: root.to_path_buf(),
        };
        let apps = build(&platform).unwrap();
        AppState {
            platform: Box::new(SandboxPlatform {
                root: root.to_path_buf(),
            }),
            apps,
            last_menu: Mutex::new(None),
            keep_awake: crate::keep_awake::Handle::new(
                root.join("data"),
                root.join("home"),
                crate::keep_awake::Capabilities {
                    hold: false,
                    thermal: false,
                    needs_authorization: true,
                },
                crate::keep_awake::Recovery {
                    reclaimed_prior: None,
                    stranded: false,
                },
            ),
            general: crate::general::Handle::new(root.join("data"), None),
        }
    }

    #[test]
    fn every_app_declared_for_this_platform_gets_a_runtime() {
        let d = tempfile::tempdir().unwrap();
        let state = state(d.path());
        let declared = app_spec::all()
            .iter()
            .filter(|s| s.locations.macos.is_some())
            .count();
        assert_eq!(state.apps.len(), declared);
        assert!(state.app("claude").is_ok());
        assert!(state.app("codex").is_ok());
    }

    #[test]
    fn an_app_not_declared_for_this_platform_is_absent_rather_than_broken() {
        // Absent, not "unavailable": the user has no idea this build was never
        // checked against their platform, so offering a row that can only fail
        // would be worse than offering none.
        struct Undeclared;
        impl Platform for Undeclared {
            fn declared_here(&self, _l: &Locations) -> bool {
                false
            }
            fn data_root(&self) -> Result<PathBuf> {
                Ok(PathBuf::from("/nowhere"))
            }
            fn default_profile_dir(&self, _l: &Locations) -> Result<PathBuf> {
                unreachable!("never asked for an undeclared app")
            }
            fn binary(&self, _l: &Locations, _p: &str) -> Result<PathBuf> {
                unreachable!()
            }
            fn process_marker(&self, _l: &Locations) -> Result<String> {
                unreachable!()
            }
            fn scan(&self, _t: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
                Ok(Vec::new())
            }
            fn link(&self, _s: &Path, _t: &Path) -> Result<()> {
                Ok(())
            }
            fn focus(&self, _pid: i32, _h: &FocusHint) -> Result<FocusOutcome> {
                unreachable!()
            }
            fn quit(&self, _pid: i32) -> Result<()> {
                unreachable!()
            }
        }
        assert!(build(&Undeclared).unwrap().is_empty());
    }

    #[test]
    fn an_unknown_app_id_is_an_error_rather_than_a_panic() {
        let d = tempfile::tempdir().unwrap();
        assert!(state(d.path()).app("no-such-app").is_err());
    }

    #[test]
    fn an_app_with_no_stock_directory_is_skipped_not_fatal() {
        // The Windows crash: one declared app is not installed, so its stock
        // directory does not exist. That must skip the app, never abort the
        // build and take every other app down with it. Modelled by failing
        // `default_profile_dir` for codex (its default profile is `.codex`)
        // while claude resolves normally.
        struct PartialInstall {
            root: PathBuf,
        }
        impl Platform for PartialInstall {
            fn declared_here(&self, locations: &Locations) -> bool {
                locations.macos.is_some()
            }
            fn data_root(&self) -> Result<PathBuf> {
                Ok(self.root.join("data"))
            }
            fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf> {
                let default = locations.macos.as_ref().unwrap().default_profile;
                if default == ".codex" {
                    return Err(anyhow!("the app's data directory was not found"));
                }
                Ok(self.root.join("home").join(default))
            }
            fn binary(&self, _l: &Locations, _p: &str) -> Result<PathBuf> {
                Err(anyhow!("not installed"))
            }
            fn process_marker(&self, locations: &Locations) -> Result<String> {
                Ok(locations.macos.as_ref().unwrap().binary.to_string())
            }
            fn scan(&self, _t: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
                Ok(Vec::new())
            }
            fn link(&self, _s: &Path, _t: &Path) -> Result<()> {
                Ok(())
            }
            fn focus(&self, _pid: i32, _h: &FocusHint) -> Result<FocusOutcome> {
                Ok(FocusOutcome::Focused)
            }
            fn quit(&self, _pid: i32) -> Result<()> {
                Ok(())
            }
        }

        let d = tempfile::tempdir().unwrap();
        let apps = build(&PartialInstall {
            root: d.path().to_path_buf(),
        })
        .expect("a missing app must not fail the whole build");
        assert!(
            apps.iter().any(|r| r.spec.id == "claude"),
            "the installed app survives"
        );
        assert!(
            !apps.iter().any(|r| r.spec.id == "codex"),
            "the uninstalled app is skipped, not present"
        );
    }

    #[test]
    fn each_app_gets_its_own_registry_under_its_own_id() {
        let d = tempfile::tempdir().unwrap();
        let state = state(d.path());
        let claude = state.app("claude").unwrap().paths.profiles_json();
        let codex = state.app("codex").unwrap().paths.profiles_json();
        assert_ne!(claude, codex);
        assert!(claude.to_string_lossy().contains("claude"));
    }

    #[test]
    fn building_writes_nothing_for_an_app_the_user_does_not_have() {
        // A user with only one app installed must not find directories for the
        // other one appearing in their Application Support folder.
        let d = tempfile::tempdir().unwrap();
        let state = state(d.path());
        for runtime in &state.apps {
            assert!(!runtime.paths.profiles_json().exists());
        }
    }

    #[test]
    fn every_app_starts_with_exactly_one_stock_profile() {
        let d = tempfile::tempdir().unwrap();
        let state = state(d.path());
        for runtime in &state.apps {
            let store = runtime.store.lock().unwrap();
            assert_eq!(store.list().len(), 1);
            assert!(store.list()[0].is_default);
        }
    }

    #[test]
    fn a_missing_binary_is_reported_as_unavailable_with_a_reason() {
        let d = tempfile::tempdir().unwrap();
        let state = state(d.path());
        let reason = state.availability(state.app("codex").unwrap()).unwrap();
        assert!(reason.contains("ChatGPT"), "got: {reason}");
    }
}
