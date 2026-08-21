use crate::app_spec::AppSpec;
use crate::instance_manager;
use crate::paths::Paths;
use crate::platform::{find_for, wm_class, FocusHint, FocusOutcome, Platform, RunningProcess};
use crate::profile_store::{Profile, ProfileStore};
use crate::runtime::{AppRuntime, AppState};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize, Clone)]
pub struct ProfileView {
    pub id: String,
    pub app_id: String,
    pub label: String,
    pub path: String,
    pub is_default: bool,
    pub shares_account: bool,
    /// Whether this profile has a live process right now. The window draws a
    /// dot from it; it is a snapshot of scan time, never cached.
    pub running: bool,
}

#[derive(Serialize, Clone)]
pub struct AppView {
    pub id: String,
    pub label: String,
    /// `None` when the app is installed; otherwise why it is not usable.
    pub unavailable: Option<String>,
    pub profiles: Vec<ProfileView>,
}

pub fn to_views(
    spec: &AppSpec,
    store: &ProfileStore,
    processes: &[RunningProcess],
) -> Vec<ProfileView> {
    let dupes = crate::account::duplicate_accounts(store.list());
    store
        .list()
        .iter()
        .map(|p| ProfileView {
            id: p.id.clone(),
            app_id: spec.id.to_string(),
            label: p.label.clone(),
            path: p.path.display().to_string(),
            is_default: p.is_default,
            shares_account: p
                .account
                .as_deref()
                .map(|account| dupes.contains(account))
                .unwrap_or(false),
            running: find_for(processes, spec.id, &p.path, p.is_default).is_some(),
        })
        .collect()
}

#[derive(Serialize, Clone)]
pub struct SocketBudget {
    /// The directory a profile added now would get, with the id standing in as
    /// a placeholder of the right width rather than a real one — the id is not
    /// drawn until the profile exists. `x` is outside the lowercase hex alphabet
    /// `fresh_id` draws from, so this can never name a profile that turns up on
    /// disk. Drawn verbatim, but as an illustration of the length, not a path to
    /// go looking for.
    pub profile_dir: String,
    /// How long the socket path *inside* that directory would be — the directory
    /// plus the longest socket name an app puts in it, so it is a little longer
    /// than `profile_dir` itself. This is the number the meter fills.
    pub used_bytes: usize,
    /// What `used_bytes` is measured against. `None` on Windows, where named
    /// pipes live outside the profile and there is no budget to keep. The window
    /// draws nothing at all in that case.
    pub limit_bytes: Option<usize>,
}

/// The socket budget for the next profile of this app.
///
/// Deliberately independent of the label: `ProfileStore::add` names a profile
/// directory after a generated id, never after what the user typed, so this
/// number is a property of the data root and does not move as they type.
pub(crate) fn budget_for(paths: &Paths) -> SocketBudget {
    let sample = paths.profile_dir(&"x".repeat(crate::profile_store::ID_LEN));
    SocketBudget {
        profile_dir: sample.display().to_string(),
        used_bytes: crate::paths::socket_path_len(&sample),
        limit_bytes: crate::paths::SOCKET_PATH_LIMIT,
    }
}

pub(crate) fn refuse_if_running(
    platform: &dyn Platform,
    spec: &'static AppSpec,
    profile: &Profile,
) -> anyhow::Result<()> {
    let processes = platform.scan(&[crate::instance_manager::scan_target(platform, spec)?])?;
    if find_for(&processes, spec.id, &profile.path, profile.is_default).is_some() {
        anyhow::bail!("quit this profile's {} before deleting it", spec.product);
    }
    Ok(())
}

/// What a walk of a profile directory found, and what it could not reach.
///
/// Two numbers rather than one because the total on its own cannot be read
/// honestly: 5 bytes is the same 5 whether the walk saw everything or bounced
/// off a `chmod 000` subtree holding a gigabyte. `skipped` is what tells the
/// caller which of those it is holding, so a short total can be shown as short
/// rather than stated as the answer. `du` prints its skips to stderr for
/// exactly this reason.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct DirectorySize {
    pub bytes: u64,
    /// Entries the walk could not reach. Not a byte figure and deliberately not
    /// convertible into one — a skipped subtree is short by an unknown amount,
    /// which is the whole reason it is counted separately.
    pub skipped: u64,
}

pub(crate) fn directory_size(path: &Path) -> anyhow::Result<DirectorySize> {
    // The `is_dir` guard belongs here, on the entry point, and deliberately NOT
    // on the recursion below. A profile whose folder does not exist yet is a
    // legitimate 0 bytes, exact — not an error, not a skip.
    //
    // `Path::is_dir()` returns false on ANY metadata error, so putting this same
    // guard on the recursive step would make a *subdirectory* that read_dir
    // listed and symlink_metadata confirmed, then that vanished before the
    // recursion reached it, return Ok{0,0} — absorbed into the parent's `Ok(sub)`
    // arm with `skipped` untouched, a short total handed up as exact. That is the
    // one asymmetry worth avoiding: a vanished *file* is already counted (its
    // symlink_metadata fails), so a vanished *directory* must be too. `walk`
    // carries no guard, so a raced-away subtree fails at its own `read_dir` and
    // lands in the `Err(_) => skipped += 1` arm — skipped, like the file.
    if !path.is_dir() {
        return Ok(DirectorySize::default());
    }
    walk(path)
}

fn walk(path: &Path) -> anyhow::Result<DirectorySize> {
    let mut total = DirectorySize::default();
    for entry in std::fs::read_dir(path)? {
        // A live profile directory — a Chromium user-data store, a Codex rollout
        // dir — rewrites and deletes files while its app runs, so an entry can
        // vanish or turn unreadable between being listed here and being measured
        // below. Skip whatever we cannot reach this instant (the `du` convention),
        // the same way `list_apps` lets a failed scan cost the dots and not the
        // list. Aborting the whole walk over one raced file withholds the entire
        // "on disk" total for the visit; skipping it leaves the total off by at
        // most that one entry. A genuinely unreadable profile root still fails,
        // above, at `read_dir` — that is a failed row, not a mid-walk race.
        let Ok(entry) = entry else {
            total.skipped += 1;
            continue;
        };
        // `symlink_metadata` does NOT follow links. Following them would descend
        // into whatever a link points at — counting bytes that live outside this
        // profile, and recursing forever on a link that points back up its own tree.
        let Ok(metadata) = entry.path().symlink_metadata() else {
            total.skipped += 1;
            continue;
        };
        if metadata.is_dir() {
            // A subtree that cannot be descended into is not one raced entry: it
            // is an unknown number of bytes. Still not fatal — a total short by a
            // subtree beats no total at all, which was the original complaint —
            // but it counts as a skip, and a skip found further down travels up
            // with it, so a caller two levels above the fault is not told the
            // total is exact.
            match walk(&entry.path()) {
                Ok(sub) => {
                    total.bytes += sub.bytes;
                    total.skipped += sub.skipped;
                }
                Err(_) => total.skipped += 1,
            }
        } else {
            total.bytes += metadata.len();
        }
    }
    Ok(total)
}

/// The frontend trims and refuses a blank label, but a Tauri command is the real
/// API boundary. A blank label renders as a nameless tray row, and a duplicate one
/// renders as two identical rows for two different accounts — both leave the user
/// unable to tell which profile they are about to launch.
///
/// Scoped to one app: "Kerja" under Claude and "Kerja" under ChatGPT sit under
/// different headers and are never two rows a user has to tell apart.
pub(crate) fn validate_label(
    store: &ProfileStore,
    label: &str,
    exclude_id: &str,
) -> Result<String, String> {
    let label = label.trim();
    if label.is_empty() {
        return Err("a profile needs a label".into());
    }
    let taken = store
        .list()
        .iter()
        .any(|p| p.id != exclude_id && p.label.eq_ignore_ascii_case(label));
    if taken {
        return Err(format!("another profile is already called “{label}”"));
    }
    Ok(label.to_string())
}

fn register_identity(platform: &dyn Platform, runtime: &AppRuntime, profile: &Profile) {
    if !runtime.spec.capabilities.desktop_identity {
        return;
    }
    let _ = platform.register_identity(
        runtime.spec,
        &profile.label,
        &wm_class(runtime.spec.id, &profile.id),
    );
}

#[tauri::command]
pub fn list_apps(state: tauri::State<AppState>) -> Result<Vec<AppView>, String> {
    // One sweep for every app, the same shape the tray rebuild uses, so the cost
    // of the window does not grow with the number of apps installed. Availability
    // is answered by looking for a binary on disk, and it is wanted twice — to
    // decide what to scan and to fill the view — so it is carried, not re-asked.
    let mut apps = Vec::new();
    let mut targets = Vec::new();
    for runtime in &state.apps {
        let unavailable = state.availability(runtime);
        if unavailable.is_none() {
            // Failing to describe what to look for is dropped for the same reason
            // failing to look is, below. Deliberately the opposite of `tray.rs`,
            // which propagates instead — and rightly, because a tray that cannot
            // scan cannot tell launch from focus and would offer the wrong action,
            // whereas the window only loses a dot.
            if let Ok(target) = crate::instance_manager::scan_target(&*state.platform, runtime.spec)
            {
                targets.push(target);
            }
        }
        apps.push((runtime, unavailable));
    }
    // A scan that fails costs the dots, not the list. The window's job is to let
    // someone rename and delete profiles; refusing to draw any of them because a
    // process listing hiccuped would be a worse answer than an unlit dot.
    let processes = state.platform.scan(&targets).unwrap_or_default();

    apps.into_iter()
        .map(|(runtime, unavailable)| {
            let store = runtime.store.lock().map_err(|e| e.to_string())?;
            Ok(AppView {
                id: runtime.spec.id.to_string(),
                label: runtime.spec.label.to_string(),
                unavailable,
                profiles: to_views(runtime.spec, &store, &processes),
            })
        })
        .collect()
}

/// Where every profile of every app lives. Drawn at the right of the status line.
#[tauri::command]
pub fn data_root(state: tauri::State<AppState>) -> Result<String, String> {
    state
        .platform
        .data_root()
        .map(|root| root.display().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn socket_budget(
    state: tauri::State<AppState>,
    app_id: String,
) -> Result<SocketBudget, String> {
    let runtime = state.app(&app_id).map_err(|e| e.to_string())?;
    Ok(budget_for(&runtime.paths))
}

#[tauri::command]
pub fn add_profile(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    app_id: String,
    label: String,
) -> Result<ProfileView, String> {
    let runtime = state.app(&app_id).map_err(|e| e.to_string())?;
    // Before the store is touched: adding saves the whole registry, and an
    // empty store beside an unreadable file would write over profiles nobody
    // has seen. Every other mutation names a profile and already fails here.
    runtime.writable().map_err(|e| e.to_string())?;
    let mut store = runtime.store.lock().map_err(|e| e.to_string())?;
    let label = validate_label(&store, &label, "")?;
    let created = store
        .add(&label, &runtime.paths)
        .map_err(|e| e.to_string())?;
    // A profile cannot already be running the moment it is created, so an empty
    // scan is the truth here rather than a shortcut.
    let view = to_views(runtime.spec, &store, &[])
        .into_iter()
        .find(|v| v.id == created.id)
        .ok_or_else(|| "profile vanished after creation".to_string())?;
    register_identity(&*state.platform, runtime, &created);
    drop(store);
    let _ = crate::tray::rebuild(&app);
    Ok(view)
}

#[tauri::command]
pub fn rename_profile(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    app_id: String,
    id: String,
    label: String,
) -> Result<(), String> {
    let runtime = state.app(&app_id).map_err(|e| e.to_string())?;
    let mut store = runtime.store.lock().map_err(|e| e.to_string())?;
    store
        .get(&id)
        .ok_or_else(|| format!("no profile with id {id}"))?;
    // Exclude this profile from the duplicate check, so re-saving its own label
    // (or only changing its capitalisation) is not reported as a collision.
    let label = validate_label(&store, &label, &id)?;
    store
        .rename(&id, &label, &runtime.paths)
        .map_err(|e| e.to_string())?;
    if let Some(renamed) = store.get(&id) {
        register_identity(&*state.platform, runtime, renamed);
    }
    drop(store);
    let _ = crate::tray::rebuild(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_profile(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    app_id: String,
    id: String,
) -> Result<(), String> {
    let runtime = state.app(&app_id).map_err(|e| e.to_string())?;
    let mut store = runtime.store.lock().map_err(|e| e.to_string())?;
    let profile = store
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("no profile with id {id}"))?;
    refuse_if_running(&*state.platform, runtime.spec, &profile).map_err(|e| e.to_string())?;
    store
        .remove(&id, &runtime.paths)
        .map_err(|e| e.to_string())?;
    // Only drop the desktop identity once removal has actually succeeded.
    // Unregistering first means a failed remove returns early via `?`, the tray
    // is never rebuilt, and the profile comes back to the list on Linux with no
    // matching WM_CLASS — a generic icon and no window grouping until restart.
    if runtime.spec.capabilities.desktop_identity {
        let _ = state
            .platform
            .unregister_identity(&wm_class(runtime.spec.id, &profile.id));
    }
    drop(store);
    let _ = crate::tray::rebuild(&app);
    Ok(())
}

#[derive(Serialize)]
pub struct AutostartState {
    /// False in development builds, where registering a login item would point at
    /// a binary that moves. The UI hides the control rather than offering a lie.
    pub offered: bool,
    pub enabled: bool,
}

/// The operating system is the single source of truth. Deliberately not mirrored
/// into `profiles.json`: a person can turn the login item off in System Settings
/// without telling this app, and a stored copy would then be confidently wrong.
#[tauri::command]
pub fn autostart_state(app: tauri::AppHandle) -> Result<AutostartState, String> {
    use tauri_plugin_autostart::ManagerExt;
    if !crate::autostart_is_offered() {
        return Ok(AutostartState {
            offered: false,
            enabled: false,
        });
    }
    let enabled = app.autolaunch().is_enabled().map_err(|e| e.to_string())?;
    Ok(AutostartState {
        offered: true,
        enabled,
    })
}

#[tauri::command]
pub fn set_autostart(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    if !crate::autostart_is_offered() {
        return Err("launching at login is only available in an installed build".into());
    }
    let manager = app.autolaunch();
    if enabled {
        manager.enable().map_err(|e| e.to_string())
    } else {
        manager.disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
pub fn profile_size_bytes(
    state: tauri::State<AppState>,
    app_id: String,
    id: String,
) -> Result<DirectorySize, String> {
    // Take the path and let the lock go. Walking a profile directory is seconds of
    // I/O on a large account, and the tray rebuild wants this same mutex from the
    // main thread on every hover — holding it across the walk freezes the whole app.
    let (_, profile) = state.profile(&app_id, &id).map_err(|e| e.to_string())?;
    directory_size(&profile.path).map_err(|e| e.to_string())
}

/// What opening one profile amounts to, given what is running right now.
#[derive(Debug, PartialEq)]
pub(crate) enum OpenAction {
    Launch,
    Focus(i32),
    /// Live already, and this app has no window to raise — the state `tray.rs`
    /// renders as a disabled `running` row. The window has one control per
    /// profile and has to say something, so this becomes a refusal rather than a
    /// second process on a directory that already has one.
    AlreadyRunning,
}

/// Takes the app id and the focus capability rather than the whole `AppSpec`:
/// they are the only two fields that move the answer, and a `focus: false` spec
/// would have to be spelled out in full as a test fixture — every one of the six
/// declared specs sets `focus: true`. The single caller reads both from the same
/// `runtime.spec`, so the pair cannot drift.
pub(crate) fn open_action(
    processes: &[RunningProcess],
    app_id: &str,
    can_focus: bool,
    profile: &Profile,
) -> OpenAction {
    match find_for(processes, app_id, &profile.path, profile.is_default) {
        Some(pid) if can_focus => OpenAction::Focus(pid),
        Some(_) => OpenAction::AlreadyRunning,
        None => OpenAction::Launch,
    }
}

/// Launch this profile, or raise it if it is already running.
///
/// The window's copy of the running flag is a snapshot from the last `list_apps`,
/// and a person can quit an app between that render and the click, so the choice
/// is made from a fresh scan here rather than from anything the page sends.
#[tauri::command]
pub fn open_profile(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    app_id: String,
    id: String,
) -> Result<(), String> {
    let (runtime, profile) = state.profile(&app_id, &id).map_err(|e| e.to_string())?;
    // Fails closed, like `instance_manager::launch`: a scan that could not be run
    // is not evidence that nothing is running, and treating it as such is exactly
    // how a second process lands on a live profile directory.
    let target =
        instance_manager::scan_target(&*state.platform, runtime.spec).map_err(|e| e.to_string())?;
    let processes = state.platform.scan(&[target]).map_err(|error| {
        format!(
            "could not check whether {} is running ({error})",
            profile.label
        )
    })?;

    match open_action(
        &processes,
        runtime.spec.id,
        runtime.spec.capabilities.focus,
        &profile,
    ) {
        OpenAction::Focus(pid) => {
            let hint = FocusHint {
                wm_class: &wm_class(runtime.spec.id, &profile.id),
            };
            match state
                .platform
                .focus(pid, &hint)
                .map_err(|error| format!("Could not focus {}: {error}", profile.label))?
            {
                FocusOutcome::Focused => {}
                FocusOutcome::Unsupported(message) => {
                    return Err(format!("Could not focus {}: {message}", profile.label));
                }
            }
        }
        OpenAction::AlreadyRunning => {
            return Err(format!(
                "{} is already running, and {} cannot be brought to the front",
                profile.label, runtime.spec.product
            ));
        }
        OpenAction::Launch => {
            instance_manager::launch(&*state.platform, runtime.spec, &profile, &runtime.paths)
                .map_err(|e| e.to_string())?;
        }
    }
    // The tray shows running state, so it is now out of date. A tray that fails to
    // redraw must not turn a launch that worked into a reported failure.
    let _ = crate::tray::rebuild(&app);
    Ok(())
}

#[tauri::command]
pub fn keep_awake_status(
    state: tauri::State<AppState>,
) -> Result<crate::keep_awake::Status, String> {
    Ok(state.keep_awake.status())
}

#[tauri::command]
pub fn set_keep_awake(
    state: tauri::State<AppState>,
    settings: crate::keep_awake::Settings,
) -> Result<crate::keep_awake::Status, String> {
    state
        .keep_awake
        .set_settings(settings)
        .map_err(|e| e.to_string())?;
    Ok(state.keep_awake.status())
}

#[tauri::command]
pub fn general_settings(state: tauri::State<AppState>) -> crate::general::Settings {
    state.general.settings()
}

#[tauri::command]
pub fn set_general_settings(
    app: tauri::AppHandle,
    state: tauri::State<AppState>,
    settings: crate::general::Settings,
) -> Result<crate::general::Settings, String> {
    state
        .general
        .set_settings(settings)
        .map_err(|e| e.to_string())?;
    // The tray carries three of the translated strings, and it was built in the
    // old language. A tray that fails to redraw must not turn a setting that was
    // saved into a reported failure — same rule as `open_profile`.
    let _ = crate::tray::rebuild(&app);
    Ok(state.general.settings())
}

/// Asks for the administrator password, once, and starts the watchdog.
///
/// Explicitly a command rather than something `setup` does when the trigger is
/// on: an unsigned app that users already have to right-click past a "damaged"
/// warning to open, and which then asks for an admin password unprompted, is
/// shaped exactly like a malware install. The prompt has to follow a click on a
/// button that has already explained what it is for.
#[tauri::command]
pub fn authorize_keep_awake(
    state: tauri::State<AppState>,
) -> Result<crate::keep_awake::Status, String> {
    let handle = &state.keep_awake;
    if let Some(refusal) = crate::paths::unquotable_refusal(&handle.data_root) {
        return Err(refusal);
    }
    let flag = crate::paths::keep_awake_flag(&handle.data_root);
    let breadcrumb = crate::paths::keep_awake_breadcrumb(&handle.data_root);
    // Read, not taken. The spawn below asks for a password and the user can
    // cancel it; discarding the reclaim value before knowing whether a watchdog
    // actually took it on would leave a stranded machine with nothing to put
    // the setting back — and the retry would then adopt the stuck value as the
    // user's own. It is forgotten only once a loop is running with it.
    let reclaimed_prior = handle.reclaimed_prior();

    state
        .platform
        .start_awake_watchdog(&crate::platform::Watchdog {
            flag: &flag,
            breadcrumb: &breadcrumb,
            reclaimed_prior,
            app_pid: std::process::id(),
        })
        .map_err(|e| e.to_string())?;

    handle.clear_reclaimed_prior();
    handle.mark_authorized();
    Ok(handle.status())
}

/// Puts sleep back after a run that died holding it, without starting a
/// watchdog. The way out for someone who does not want the feature on.
///
/// Disarms the trigger and drops the flag before restoring anything, because
/// the way out has to stay out. It is tempting to argue that neither is needed
/// — `stranded` and `authorized` cannot both be true inside one `Handle`, so
/// this app's own sweep is writing a flag nothing is watching. That argument is
/// about a process, and `disablesleep` is a machine. Nothing stops a second
/// copy of the app running — `cargo tauri dev` beside the installed build is
/// the likeliest way, and both derive the same data root from `$HOME`. The
/// second copy's startup deletes the flag and the breadcrumb, so it reports a
/// stranded machine while the first copy's root loop is still alive and still
/// polling that same flag every three seconds. Pressing this button there put
/// sleep back and the older loop took it away again within one poll, with the
/// banner cleared and nothing on screen saying so.
///
/// So the order matters and each step earns its place: the trigger goes first
/// or this app's own sweep rewrites the flag fifteen seconds later; the flag
/// goes next because it is the only channel to a loop this process did not
/// start and cannot see — removing it is what makes a *foreign* watchdog let
/// go, since the loop is edge-triggered on the flag existing; and only then is
/// the setting put back, so nothing can re-take it between those two lines.
/// The body is in `keep_awake::restore`, where a test can reach it without a
/// `tauri::State`. That is not tidiness: the invariant this used to lean on was
/// asserted by a test that could not have observed it failing.
#[tauri::command]
pub fn restore_sleep(state: tauri::State<AppState>) -> Result<crate::keep_awake::Status, String> {
    crate::keep_awake::restore(&state.keep_awake, state.platform.as_ref())
        .map_err(|e| e.to_string())?;
    Ok(state.keep_awake.status())
}

/// Hands the machine back on the way into an update install.
///
/// The bundled updater exits the process itself — on Windows it hands off to the
/// NSIS installer and calls `std::process::exit(0)`, which reaches neither
/// `RunEvent::ExitRequested` nor `RunEvent::Exit`, so the release wired into
/// `App::run` never fires. The window calls this the moment before it installs,
/// the last point at which our own code still runs. Not `restore_sleep`:
/// quitting for an update is not "turn Keep Awake off" — the trigger is left as
/// the user set it, only the OS-level hold goes back.
///
/// Not `release_at_exit` either: that stops the sweep, which is right when the
/// app is going away but wrong here. An update install can fail — download and
/// release succeed, then `install()` throws — and on macOS and Linux the app
/// survives that. Stopping the sweep would leave keep-awake dead until a manual
/// relaunch. `release_for_update` hands the OS hold back but leaves the sweep
/// running, so a failed install self-heals when the next sweep re-arms the hold.
#[tauri::command]
pub fn release_keep_awake_for_update(state: tauri::State<AppState>) -> Result<(), String> {
    crate::keep_awake::release_for_update(&state.keep_awake, state.platform.as_ref())
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_spec;
    use crate::paths::Paths;
    use crate::platform::RunningProcess;
    use crate::shared_config::tests_support::FakePlatform;
    use std::path::PathBuf;

    fn store_in(dir: &Path) -> (Paths, ProfileStore) {
        let paths = Paths::new(dir.join("root"));
        let def = dir.join("stock");
        std::fs::create_dir_all(&def).unwrap();
        let store = ProfileStore::load(&paths, &def).unwrap();
        (paths, store)
    }

    #[test]
    fn profiles_sharing_an_account_are_flagged_in_the_view() {
        let d = tempfile::tempdir().unwrap();
        let (paths, mut store) = store_in(d.path());
        let a = store.add("A", &paths).unwrap();
        let b = store.add("B", &paths).unwrap();
        store.set_account(&a.id, Some("same".into()));
        store.set_account(&b.id, Some("same".into()));

        let views = to_views(&app_spec::CLAUDE, &store, &[]);

        assert!(views.iter().find(|v| v.id == a.id).unwrap().shares_account);
        assert!(views.iter().find(|v| v.id == b.id).unwrap().shares_account);
        assert!(!views[0].shares_account); // the stock profile has no account
    }

    #[test]
    fn a_view_carries_the_app_it_belongs_to() {
        // The window needs it to send the right app id back on every action.
        let d = tempfile::tempdir().unwrap();
        let (_, store) = store_in(d.path());
        assert_eq!(to_views(&app_spec::CODEX, &store, &[])[0].app_id, "codex");
    }

    #[test]
    fn a_view_reports_whether_that_profile_has_a_running_process() {
        // The window draws a dot per row, so the running flag has to be per
        // profile — not "this app has something running somewhere".
        let d = tempfile::tempdir().unwrap();
        let (paths, mut store) = store_in(d.path());
        let work = store.add("Work", &paths).unwrap();
        let idle = store.add("Idle", &paths).unwrap();

        let processes = vec![RunningProcess {
            app_id: "codex",
            pid: 4242,
            profile_dir: Some(work.path.clone()),
        }];

        let views = to_views(&app_spec::CODEX, &store, &processes);
        assert!(views.iter().find(|v| v.id == work.id).unwrap().running);
        assert!(!views.iter().find(|v| v.id == idle.id).unwrap().running);
    }

    #[test]
    fn nothing_is_running_when_the_scan_found_nothing() {
        let d = tempfile::tempdir().unwrap();
        let (_, store) = store_in(d.path());
        assert!(to_views(&app_spec::CODEX, &store, &[])
            .iter()
            .all(|v| !v.running));
    }

    // The literals below spell the path with `/`; on Windows `PathBuf::join`
    // writes `\` and the assert would fail on the separator alone. A socket path
    // is a Unix concept anyway — `SOCKET_PATH_LIMIT` is `None` on Windows — so
    // there is nothing here to measure off Unix.
    #[cfg(unix)]
    #[test]
    fn the_budget_spells_out_the_whole_path_it_measures() {
        // Written out as a literal rather than rebuilt from `profile_dir` and
        // `socket_path_len` — the latter would restate `budget_for`'s own body and
        // could not fail. This version breaks if the `p/` layout, the id width or
        // the socket-name budget ever move, which is exactly what it is for.
        let budget = budget_for(&Paths::new("/root/claude"));

        assert_eq!(budget.profile_dir, "/root/claude/p/xxxxxxxx");
        assert_eq!(
            budget.used_bytes,
            "/root/claude/p/xxxxxxxx".len() + "/1.13-main.sock".len()
        );
        // `limit_bytes` is deliberately not asserted here: against
        // `SOCKET_PATH_LIMIT` it would restate `budget_for`'s own body, and it is
        // already pinned non-vacuously by `a_cramped_root_reports_over_its_limit`.
    }

    #[test]
    fn the_budget_measures_a_profile_that_does_not_exist_yet() {
        // Nothing may be created just to answer "would one fit?".
        let d = tempfile::tempdir().unwrap();
        let paths = Paths::new(d.path().join("root"));

        budget_for(&paths);

        assert!(
            !paths.profiles_dir().exists(),
            "asking must not create anything"
        );
    }

    #[test]
    fn the_predicted_width_matches_a_profile_actually_created() {
        // `budget_for` guesses the directory width from `ID_LEN` before any
        // profile exists. If `fresh_id` ever produced a different width the
        // meter would quietly measure the wrong path, so pin the two together.
        let d = tempfile::tempdir().unwrap();
        let (paths, mut store) = store_in(d.path());
        let created = store.add("Work", &paths).unwrap();

        assert_eq!(
            budget_for(&paths).used_bytes,
            crate::paths::socket_path_len(&created.path)
        );
        // The sample path pads with `x`, which is safe only because `fresh_id`
        // draws lowercase hex. Swapping it for base32 or base58 would let the
        // illustration collide with a real profile directory; fail here first.
        //
        // Asserted as a character class rather than as "contains no `x`": one
        // random id drawn from base32 has no `x` about four times in five, so
        // the narrower check would pass through the very change it guards and
        // read as a flake when it finally fired.
        assert!(
            created
                .id
                .chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "the sample id is no longer impossible: {}",
            created.id
        );
    }

    #[test]
    fn a_cramped_root_reports_over_its_limit() {
        // The whole point of drawing the meter: a home directory long enough that
        // no profile can be created must say so before the user types a name.
        //
        // Asserted against the macOS number directly rather than against whatever
        // this platform's limit happens to be, so the assertion actually runs on
        // Windows CI instead of being skipped there — the same reason `paths.rs`
        // split `fits_within` out from the platform question.
        let paths = Paths::new(format!("/Users/{}/agent-profiles/claude", "n".repeat(120)));
        let used = budget_for(&paths).used_bytes;
        assert!(
            used > crate::paths::MACOS_SOCKET_PATH_LIMIT,
            "got {used} against {}",
            crate::paths::MACOS_SOCKET_PATH_LIMIT
        );
    }

    #[test]
    fn deletion_is_refused_when_the_profile_has_a_running_instance() {
        let profile = Profile {
            id: "work".into(),
            label: "Work".into(),
            path: PathBuf::from("/profiles/work"),
            is_default: false,
            account: None,
        };
        let platform = FakePlatform::with_running(vec![RunningProcess {
            app_id: "codex",
            pid: 4242,
            profile_dir: Some(profile.path.clone()),
        }]);

        let error = refuse_if_running(&platform, &app_spec::CODEX, &profile)
            .unwrap_err()
            .to_string();
        assert_eq!(
            error,
            "quit this profile's the ChatGPT desktop app before deleting it"
        );
    }

    #[test]
    fn deletion_is_allowed_when_only_the_other_app_is_running() {
        let profile = Profile {
            id: "work".into(),
            label: "Work".into(),
            path: PathBuf::from("/profiles/work"),
            is_default: false,
            account: None,
        };
        let platform = FakePlatform::with_running(vec![RunningProcess {
            app_id: "claude",
            pid: 4242,
            profile_dir: Some(profile.path.clone()),
        }]);

        assert!(refuse_if_running(&platform, &app_spec::CODEX, &profile).is_ok());
    }

    fn work() -> Profile {
        Profile {
            id: "work".into(),
            label: "Work".into(),
            path: PathBuf::from("/profiles/work"),
            is_default: false,
            account: None,
        }
    }

    fn running(app_id: &'static str, profile: &Profile) -> Vec<RunningProcess> {
        vec![RunningProcess {
            app_id,
            pid: 4242,
            profile_dir: Some(profile.path.clone()),
        }]
    }

    #[test]
    fn a_profile_with_no_process_is_launched() {
        assert_eq!(open_action(&[], "codex", true, &work()), OpenAction::Launch);
    }

    #[test]
    fn a_running_profile_is_focused_rather_than_launched_again() {
        let profile = work();
        assert_eq!(
            open_action(&running("codex", &profile), "codex", true, &profile),
            OpenAction::Focus(4242)
        );
    }

    #[test]
    fn a_running_profile_of_an_app_that_cannot_be_raised_is_left_alone() {
        // The case the window's own running flag cannot be trusted for: with no
        // window to raise, the only alternative to doing nothing is a second
        // process on a profile directory that already has one.
        let profile = work();
        assert_eq!(
            open_action(&running("codex", &profile), "codex", false, &profile),
            OpenAction::AlreadyRunning
        );
    }

    #[test]
    fn a_blank_label_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let (_, store) = store_in(d.path());
        assert!(validate_label(&store, "   ", "").is_err());
        assert_eq!(validate_label(&store, "  Kerja  ", "").unwrap(), "Kerja");
    }

    #[test]
    fn a_label_already_taken_by_another_profile_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let (paths, mut store) = store_in(d.path());
        let kerja = store.add("Kerja", &paths).unwrap();

        // Case differences still collide: two tray rows a person cannot tell apart.
        assert!(validate_label(&store, "kerja", "").is_err());
        // But a profile may keep, or re-case, its own label.
        assert_eq!(validate_label(&store, "KERJA", &kerja.id).unwrap(), "KERJA");
    }

    #[test]
    fn the_same_label_under_two_apps_is_allowed() {
        // Each app has its own store, and the tray puts them under separate
        // headers — there is nothing for a user to confuse.
        let d = tempfile::tempdir().unwrap();
        let (paths_a, mut claude) = store_in(&d.path().join("claude"));
        let (_, codex) = store_in(&d.path().join("codex"));
        claude.add("Kerja", &paths_a).unwrap();

        assert!(validate_label(&claude, "Kerja", "").is_err());
        assert!(validate_label(&codex, "Kerja", "").is_ok());
    }

    #[test]
    fn a_symlinked_directory_is_not_followed_when_measuring_a_profile() {
        let d = tempfile::tempdir().unwrap();
        let profile = d.path().join("profile");
        std::fs::create_dir(&profile).unwrap();
        std::fs::write(profile.join("real.bin"), b"1234").unwrap();

        let outside = d.path().join("outside");
        std::fs::create_dir(&outside).unwrap();
        std::fs::write(outside.join("huge.bin"), vec![0u8; 4096]).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, profile.join("link")).unwrap();

        // Only the profile's own 4 bytes count, plus the link entry itself — never
        // the 4096 bytes living outside the profile.
        assert!(directory_size(&profile).unwrap().bytes < 100);
    }

    #[test]
    fn profile_size_includes_nested_files() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("root.bin"), b"123").unwrap();
        let nested = d.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("child.bin"), b"12345").unwrap();

        assert_eq!(directory_size(d.path()).unwrap().bytes, 8);
    }

    // A live profile dir (a Chromium user-data store, a Codex rollout dir)
    // rewrites and deletes files while the app runs, so an entry can vanish or
    // turn unreadable between listing it and measuring it. That is a race, not
    // an unreadable profile: the walk must return the size of what it could see
    // (the `du` convention), not fail outright and cost the whole "on disk"
    // total for the visit.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_entry_is_skipped_rather_than_failing_the_whole_walk() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("seen.bin"), b"12345").unwrap();
        let blocked = d.path().join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("hidden.bin"), b"9999999999").unwrap();
        // A directory the walk cannot descend into stands in for the entry that
        // errored mid-walk.
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Root ignores the mode bits; the assertion is only meaningful for an
        // unprivileged user. Restore permissions either way so tempdir cleanup works.
        let readable_as_root = std::fs::read_dir(&blocked).is_ok();
        let measured = directory_size(d.path());
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755)).unwrap();

        if readable_as_root {
            return;
        }
        let measured = measured.unwrap();
        assert_eq!(
            measured.bytes, 5,
            "the readable file still counts; the unreadable directory is skipped, not fatal"
        );
        // And the walk says so. `bytes` alone is 5 whether the blocked directory
        // held ten bytes or ten gigabytes; `skipped` is the difference between a
        // total worth showing and one that quietly misleads.
        assert_eq!(
            measured.skipped, 1,
            "a subtree the walk could not descend into is reported, not absorbed"
        );
    }

    // A walk that reached everything says so, so the caller has something to
    // test: `skipped == 0` is what lets the exact total be shown as exact. The
    // byte total is asserted alongside it — `skipped == 0` on its own passes
    // against a function that hard-codes it, and against every total that
    // under-reports while still claiming to be exact.
    #[test]
    fn a_walk_that_reached_everything_reports_the_full_total_as_exact() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("a.bin"), b"123").unwrap();
        let nested = d.path().join("nested");
        std::fs::create_dir(&nested).unwrap();
        std::fs::write(nested.join("b.bin"), b"45").unwrap();

        let measured = directory_size(d.path()).unwrap();
        assert_eq!(
            (measured.bytes, measured.skipped),
            (5, 0),
            "every byte was reached, so the exact total is reported as exact",
        );
    }

    // The blocking asymmetry: a vanished *file* is counted (its symlink_metadata
    // fails, landing in a skip arm), but a vanished *directory* used to be
    // absorbed. `directory_size` opened with an `is_dir` guard that returns
    // Ok{0,0} on any metadata error, so a subdirectory that read_dir listed and
    // symlink_metadata confirmed — then that vanished in the window before the
    // recursion's own read_dir reached it — returned an empty, exact size. The
    // parent's `Ok(sub)` arm added zero to both bytes and `skipped`, and the
    // short total was handed up as exact. A live Chromium store deleting `Cache/`
    // mid-walk is the common case, not a corner.
    //
    // The race cannot be reproduced by a black-box walk of a directory: a subtree
    // that contributes zero bytes reads identically whether it was correctly
    // skipped or wrongly absorbed, and a deletion that lands before the parent's
    // read_dir even listed the entry is a legitimate zero. So the guarantee is
    // pinned at the seam where it lives. `walk` is the recursion the parent
    // descends through, and it carries no `is_dir` guard: a subtree it cannot read
    // fails at read_dir and returns Err — which the parent's `Err(_) => skipped
    // += 1` arm turns into a skip, exactly as a vanished file is skipped — rather
    // than Ok(empty), which the parent absorbs. The guard stays on the public
    // `directory_size` entry point, where a profile with no folder at all is a
    // legitimate, exact zero rather than an error.
    #[test]
    fn a_subtree_the_recursion_cannot_read_is_an_error_not_a_silent_zero() {
        let d = tempfile::tempdir().unwrap();
        // Never created: it stands in for a subdirectory the parent listed and
        // confirmed, then that vanished before the recursion reached it.
        let vanished = d.path().join("listed-then-gone");
        assert!(
            walk(&vanished).is_err(),
            "the recursion must report a subtree it cannot read as an error the \
             parent records as a skip — not Ok(empty), which is absorbed and the \
             short total handed up as exact",
        );
        // The guard the recursion sheds still lives on the entry point: a profile
        // with no folder is a legitimate, exact zero, not an error and not a skip.
        let entry = directory_size(&vanished).unwrap();
        assert_eq!((entry.bytes, entry.skipped), (0, 0));
    }

    // A skip deep in the tree is still a skip at the top. The count is summed up
    // the recursion rather than reset at each level, or a caller two directories
    // above the fault would be told the total is exact. The readable bytes
    // alongside the blocked subtree are asserted too, so the test pins that the
    // walk still counts what it could reach while flagging what it could not.
    #[cfg(unix)]
    #[test]
    fn a_skip_deep_in_the_tree_reaches_the_top_of_the_walk() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("seen.bin"), b"1234").unwrap();
        let middle = d.path().join("middle");
        std::fs::create_dir(&middle).unwrap();
        std::fs::write(middle.join("also-seen.bin"), b"567").unwrap();
        let blocked = middle.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        std::fs::write(blocked.join("hidden.bin"), b"9999999999").unwrap();
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o000)).unwrap();

        let readable_as_root = std::fs::read_dir(&blocked).is_ok();
        let measured = directory_size(d.path());
        std::fs::set_permissions(&blocked, std::fs::Permissions::from_mode(0o755)).unwrap();

        if readable_as_root {
            return;
        }
        let measured = measured.unwrap();
        assert_eq!(
            (measured.bytes, measured.skipped),
            (7, 1),
            "the seven readable bytes count; the blocked subtree two levels down is \
             one skip that reaches the top, not absorbed and not fatal",
        );
    }
}
