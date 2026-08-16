mod account;
mod agent_activity;
mod app_spec;
mod commands;
mod instance_manager;
mod keep_awake;
mod paths;
mod platform;
#[cfg(test)]
mod probe;
mod profile_store;
mod runtime;
mod shared_config;
mod tray;
#[cfg(test)]
mod verify;

use crate::platform::{find_for, wm_class, FocusHint};
use crate::runtime::AppState;
use anyhow::{anyhow, Result};
use tauri::{Emitter, Manager};

fn state(app: &tauri::AppHandle) -> Result<tauri::State<'_, AppState>> {
    app.try_state::<AppState>()
        .ok_or_else(|| anyhow!("Agent Profiles state is not available"))
}

/// Finds the pid for one profile, rescanning rather than trusting the menu.
///
/// A menu built seconds ago can name a process that has since exited, and the
/// pid would then belong to whatever the OS handed the number to next.
fn live_pid(app: &tauri::AppHandle, app_id: &str, profile_id: &str) -> Result<i32> {
    let state = state(app)?;
    let (runtime, profile) = state.profile(app_id, profile_id)?;
    let processes = state.platform.scan(&[instance_manager::scan_target(
        &*state.platform,
        runtime.spec,
    )?])?;
    find_for(
        &processes,
        runtime.spec.id,
        &profile.path,
        profile.is_default,
    )
    .ok_or_else(|| anyhow!("{} is no longer running", profile.label))
}

fn handle_menu_event(app: &tauri::AppHandle, id: &str) -> Result<()> {
    match tray::parse_row_id(id) {
        Some(("launch", app_id, profile_id)) => {
            let state = state(app)?;
            let (runtime, profile) = state.profile(app_id, profile_id)?;
            instance_manager::launch(&*state.platform, runtime.spec, &profile, &runtime.paths)?;
            tray::rebuild(app)?;
        }
        Some(("focus", app_id, profile_id)) => {
            let pid = live_pid(app, app_id, profile_id)?;
            let state = state(app)?;
            let (runtime, profile) = state.profile(app_id, profile_id)?;
            let hint = FocusHint {
                wm_class: &wm_class(runtime.spec.id, &profile.id),
            };
            match state.platform.focus(pid, &hint)? {
                platform::FocusOutcome::Focused => {
                    tray::rebuild(app)?;
                }
                platform::FocusOutcome::Unsupported(message) => {
                    let reason = format!("Could not focus {}: {message}", profile.label);
                    eprintln!("{reason}");
                    tray::rebuild_with_error(app, Some(&reason))?;
                }
            }
        }
        // Headers, error rows and anything else carrying a colon are inert.
        Some(_) => {}
        None if id == "manage" => {
            let window = app
                .get_webview_window("main")
                .ok_or_else(|| anyhow!("management window is not available"))?;
            window.show()?;
            window.set_focus()?;
            // The window is hidden rather than destroyed, so its DOM survives being
            // closed — including any error banner. Tell the page it is on screen
            // again so it can refresh, instead of showing a verdict from last time.
            window.emit("window-shown", ())?;
        }
        None if id == "quit_app" => {
            app.exit(0);
        }
        None => {}
    }

    Ok(())
}

const MIN_WINDOW_WIDTH: f64 = 560.0;
const MIN_WINDOW_HEIGHT: f64 = 420.0;

/// Registering autostart from a development build would point the operating
/// system at the `target/debug` binary, which moves, gets rebuilt, and is deleted
/// by `cargo clean` — leaving a login item that fails silently every boot. Only a
/// bundled release build has a stable path worth registering.
pub(crate) fn autostart_is_offered() -> bool {
    !cfg!(debug_assertions)
}

/// A tray app outlives its windows. Closing the management window must hide it,
/// never destroy it: the webview is created once, and `get_webview_window` would
/// return `None` from then on, leaving "Settings…" permanently broken.
pub(crate) fn close_hides_window(label: &str) -> bool {
    label == "main"
}

/// `None` means a person closed the last window, which for a tray app is not a
/// request to quit — the tray is still there. `Some` only ever comes from our own
/// `app.exit()`, i.e. the "Quit" row, which really must quit.
pub(crate) fn exit_should_be_prevented(code: Option<i32>) -> bool {
    code.is_none()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        // Opt-in only: the plugin registers nothing until the user asks for it.
        // `None` bakes no extra arguments into the login item.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            commands::list_apps,
            commands::add_profile,
            commands::rename_profile,
            commands::delete_profile,
            commands::profile_size_bytes,
            commands::autostart_state,
            commands::set_autostart,
            commands::data_root,
            commands::socket_budget,
            commands::open_profile,
            commands::keep_awake_status,
            commands::set_keep_awake,
            commands::authorize_keep_awake,
            commands::restore_sleep,
        ])
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if let Err(error) = handle_menu_event(app, id) {
                let reason = format!("Tray action `{id}` failed: {error}");
                eprintln!("{reason}");
                if let Err(rebuild_error) = tray::rebuild_with_error(app, Some(&reason)) {
                    eprintln!("tray rebuild failed: {rebuild_error}");
                }
            }
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if close_hides_window(window.label()) {
                    api.prevent_close();
                    if let Err(error) = window.hide() {
                        eprintln!("could not hide the management window: {error}");
                    }
                }
            }
        })
        .on_tray_icon_event(|app, event| {
            if tray::should_rebuild_for_event(&event) {
                if let Err(error) = tray::rebuild(app) {
                    eprintln!("tray rebuild failed: {error}");
                }
            }
        })
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Asserted here as well as in tauri.conf.json. The profile rows put a
            // label, a full filesystem path and two buttons on one line; below this
            // width they overlap into something unusable.
            if let Some(window) = app.get_webview_window("main") {
                let floor = tauri::LogicalSize::new(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
                if let Err(error) = window.set_min_size(Some(floor)) {
                    eprintln!("could not set the minimum window size: {error}");
                }
            }

            let platform = platform::current();
            let apps = runtime::build(&*platform)?;

            // Before anything else touches the flag: a run that crashed while
            // holding leaves both a flag and a breadcrumb, and the first sweep
            // must not inherit a hold that nothing has checked a battery against.
            let data_root = platform.data_root()?;
            let recovery = keep_awake::recover_at_startup(&data_root);
            // Whatever this platform can put back by itself, put back now.
            // Only macOS cannot — `disablesleep` is root's — and it reports the
            // machine as stranded through `recovery` instead.
            if let Err(error) = platform.recover_hold(&data_root) {
                eprintln!("could not restore the lid setting from a previous run: {error}");
            }
            let home = std::path::PathBuf::from(
                std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .unwrap_or_default(),
            );
            let keep_awake = keep_awake::Handle::new(
                data_root,
                home,
                keep_awake::Capabilities {
                    hold: platform.can_hold_awake(),
                    thermal: platform.can_read_thermal(),
                    needs_authorization: platform.needs_authorization(),
                },
                recovery,
            );

            app.manage(AppState {
                platform,
                apps,
                last_menu: std::sync::Mutex::new(None),
                keep_awake,
            });

            // The sweep the guards depend on. Nothing else in this app runs on a
            // timer, and the lid-closed case is precisely the case where nobody
            // opens the tray and nobody renders the window — so a demand-driven
            // scan could never drop the flag when the agent stopped.
            let sweeper = app.handle().clone();
            std::thread::spawn(move || keep_awake::watch(sweeper));

            if let Some(state) = app.try_state::<AppState>() {
                tray::sync_identities(&state);
            }
            tray::rebuild(app.handle())?;
            Ok(())
        })
        .build(tauri::generate_context!());

    match app {
        Ok(app) => app.run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { api, code, .. } = &event {
                if exit_should_be_prevented(*code) {
                    api.prevent_exit();
                }
            }
        }),
        Err(error) => eprintln!("Agent Profiles failed to run: {error}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_the_management_window_hides_it_rather_than_destroying_it() {
        assert!(close_hides_window("main"));
        assert!(!close_hides_window("some-future-window"));
    }

    #[test]
    fn only_our_own_quit_row_is_allowed_to_end_the_process() {
        // A person closing the last window reports no code; the tray lives on.
        assert!(exit_should_be_prevented(None));
        // `app.exit(0)` from "Quit" reports one, and must win.
        assert!(!exit_should_be_prevented(Some(0)));
    }
}
