use crate::app_spec::AppSpec;
use crate::platform::{find_for, wm_class, RunningProcess, ScanTarget, Unavailable};
use crate::profile_store::{Profile, ProfileStore};
use crate::runtime::AppState;
use anyhow::{anyhow, Result};
use tauri::Manager;

pub type MenuSignature = (String, String, bool, bool);

pub(crate) fn signature(rows: &[MenuRow]) -> Vec<MenuSignature> {
    rows.iter()
        .map(|r| (r.id.clone(), r.text.clone(), r.enabled, r.running))
        .collect()
}

/// The full rebuild key: the rows PLUS the locale. The Settings…/Quit items are
/// locale-dependent but are not part of `rows`, so a language change with no
/// other row change would otherwise leave `signature` identical and skip the
/// rebuild, stranding the tray's static items in the previous language.
pub(crate) fn menu_signature(
    rows: &[MenuRow],
    locale: crate::general::Locale,
) -> Vec<MenuSignature> {
    let mut sig = signature(rows);
    // A sentinel row that can't collide with a real menu id (ids are
    // `action:app:profile` or plain words; this starts with a NUL).
    sig.push((
        "\0locale".to_string(),
        locale.tag().to_string(),
        false,
        false,
    ));
    sig
}

/// The window's own `--live` green, midway between the value it takes in the
/// light theme and the one it takes in the dark. A menu item's image is a plain
/// bitmap rather than a template, so it arrives in whatever colours it was built
/// with and has to hold up against both a light and a dark menu.
const LIVE: [u8; 3] = [47, 164, 101];
/// Stopped is drawn, not left blank: a row with no image would lose the indent
/// every other row has, and the list would comb in and out as profiles start.
const IDLE: [u8; 3] = [142, 142, 147];

/// 32px for a mark that lands at 16pt, so it still has edges to anti-alias on a
/// display that is not retina.
const DOT: u32 = 32;

/// Profile labels are set a step below the menu font's own 14pt, so that a
/// person with nine profiles gets a shorter menu out of it. Only the profiles
/// move: `Settings…` and `Quit` stay at full size, and that contrast is
/// what makes the smaller rows read as a decision rather than as a mistake.
#[cfg(target_os = "macos")]
const PROFILE_TYPE_SIZE: f64 = 12.0;

/// A disc while a profile is running, a ring while it is not — the pairing the
/// Wi-Fi menu uses for a joined network against a merely known one.
///
/// Built here rather than bundled as two PNGs: it is two circles, and this way
/// the green stays written down once, next to the token it was taken from.
pub(crate) fn status_dot(running: bool) -> tauri::image::Image<'static> {
    let (rgb, inner) = if running {
        (LIVE, None)
    } else {
        (IDLE, Some(6.4_f32))
    };
    let outer = 9.0_f32;
    let centre = DOT as f32 / 2.0;
    let mut rgba = Vec::with_capacity((DOT * DOT * 4) as usize);
    for y in 0..DOT {
        for x in 0..DOT {
            let dx = x as f32 + 0.5 - centre;
            let dy = y as f32 + 0.5 - centre;
            let d = (dx * dx + dy * dy).sqrt();
            let mut alpha = edge(outer - d);
            if let Some(inner) = inner {
                alpha *= edge(d - inner);
            }
            rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], (alpha * 255.0).round() as u8]);
        }
    }
    tauri::image::Image::new_owned(rgba, DOT, DOT)
}

/// One pixel of feathering either side of a boundary, so a circle this small
/// does not arrive with a staircase for an outline.
fn edge(distance_inside: f32) -> f32 {
    (distance_inside + 0.5).clamp(0.0, 1.0)
}

/// macOS delivers tray events inconsistently across versions: some send only
/// `Enter` before a menu opens, others also send `Click` once it is already
/// open. Rebuilding on the latter swapped the menu out from under the user,
/// and macOS closes an attached menu the moment it is replaced — the menu
/// appeared and vanished, over and over. So: replace only on a real change.
pub(crate) fn should_replace_menu(
    previous: Option<&[MenuSignature]>,
    next: &[MenuSignature],
) -> bool {
    match previous {
        None => true,
        Some(previous) => previous != next,
    }
}

/// Deliberately carries no pid. A pid captured while the menu was being built can
/// be dead by the time the row is clicked, so every handler rescans for itself —
/// storing one here would only invite someone to trust the stale copy.
pub struct MenuRow {
    pub id: String,
    pub text: String,
    pub enabled: bool,
    /// Which of the two status dots this row carries. Only profile rows have
    /// one; a header or an error row is drawn without an image and so sits
    /// flush left, the way `Known Network` does above the networks under it.
    pub running: bool,
}

/// One app's contribution to the menu, flattened out of the locks so that
/// building rows is a pure function of what was true at scan time.
pub struct AppSection {
    pub spec: &'static AppSpec,
    pub profiles: Vec<Profile>,
    /// Why this app cannot be used, if it cannot. It contributes no profile rows
    /// — there is nothing to launch — but it does contribute one greyed row
    /// carrying this reason, so an app the tool knows about is never silently
    /// missing from the menu. The menu takes the short form of it; see
    /// [`Unavailable`].
    pub unavailable: Option<Unavailable>,
}

/// A tray row id, `action:app:profile`. The app id is in the middle because the
/// action is what the handler switches on first.
pub fn row_id(action: &str, app_id: &str, profile_id: &str) -> String {
    format!("{action}:{app_id}:{profile_id}")
}

pub fn parse_row_id(id: &str) -> Option<(&str, &str, &str)> {
    let mut parts = id.splitn(3, ':');
    Some((parts.next()?, parts.next()?, parts.next()?))
}

pub(crate) fn combine_error_messages(
    messages: impl IntoIterator<Item = Option<String>>,
) -> Option<String> {
    let mut messages = messages.into_iter().flatten();
    let first = messages.next()?;
    Some(
        std::iter::once(first)
            .chain(messages)
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub(crate) fn scan_processes(
    result: Result<Vec<RunningProcess>>,
) -> (Vec<RunningProcess>, Option<String>) {
    match result {
        Ok(processes) => (processes, None),
        Err(error) => (
            Vec::new(),
            Some(format!("Could not scan running instances: {error}")),
        ),
    }
}

pub fn menu_rows(
    sections: &[AppSection],
    processes: &[RunningProcess],
    runtime_error: Option<&str>,
    locale: crate::general::Locale,
) -> Vec<MenuRow> {
    let strings = crate::general::tray_strings(locale);
    let available: Vec<&AppSection> = sections
        .iter()
        .filter(|section| section.unavailable.is_none())
        .collect();
    // Headers only earn their space once there is more than one app to tell
    // apart. With one installed the menu is exactly the flat list it always was.
    let headed = available.len() > 1;
    let enabled = runtime_error.is_none();

    let mut rows = Vec::new();
    for section in &available {
        if headed {
            rows.push(MenuRow {
                id: format!("header:{}", section.spec.id),
                text: section.spec.label.to_string(),
                enabled: false,
                running: false,
            });
        }
        let dupes = crate::account::duplicate_accounts(&section.profiles);
        for profile in &section.profiles {
            let pid = find_for(
                processes,
                section.spec.id,
                &profile.path,
                profile.is_default,
            );
            let shares_account = profile
                .account
                .as_deref()
                .map(|account| dupes.contains(account))
                .unwrap_or(false);
            let suffix = if shares_account {
                strings.same_account
            } else {
                ""
            };
            let action = if pid.is_some() && section.spec.capabilities.focus {
                "focus"
            } else if pid.is_some() {
                "running"
            } else {
                "launch"
            };

            rows.push(MenuRow {
                id: row_id(action, section.spec.id, &profile.id),
                text: format!("{}{suffix}", profile.label),
                enabled: enabled && action != "running",
                running: pid.is_some(),
            });
        }
    }

    // An app that cannot be used contributes no row here — the tray is a launch
    // menu, and "X is not installed" names nothing you can act on. It also
    // widens every row above it to the longest stock path among however many of
    // the seven declared apps are missing. The window still says why, where the
    // width is not shared with anything and the reason has somewhere useful to
    // sit (`ProfileList`'s `Unavailable` row, `EmptyState`'s reason list).
    if let Some(message) = runtime_error {
        rows.push(MenuRow {
            id: "error".into(),
            text: message.to_string(),
            enabled: false,
            running: false,
        });
    }
    rows
}

/// Refresh on `Enter` as well as `Click`. On macOS the click that opens an
/// attached menu is consumed by the menu itself, so `Click` alone can never
/// fire and the menu would show whatever was true when it was last built —
/// reporting a profile as running long after the user quit it by hand.
/// `Enter` fires once as the pointer arrives, which is exactly the moment
/// before the menu opens. `Move` is the one that repeats, and stays excluded.
pub(crate) fn should_rebuild_for_event(event: &tauri::tray::TrayIconEvent) -> bool {
    matches!(
        event,
        tauri::tray::TrayIconEvent::Click { .. } | tauri::tray::TrayIconEvent::Enter { .. }
    )
}

pub(crate) fn refresh_accounts(store: &mut ProfileStore, spec: &AppSpec) -> bool {
    let mut changed = false;
    for profile in store.list().to_vec() {
        let account = crate::account::read_account(&profile.path, spec.identity.as_ref());
        if profile.account != account {
            store.set_account(&profile.id, account);
            changed = true;
        }
    }
    changed
}

pub fn rebuild(app: &tauri::AppHandle) -> Result<()> {
    rebuild_with_error(app, None)
}

pub(crate) fn rebuild_with_error(
    app: &tauri::AppHandle,
    runtime_error: Option<&str>,
) -> Result<()> {
    let state = app
        .try_state::<AppState>()
        .ok_or_else(|| anyhow!("Agent Profiles state is not available"))?;

    let mut sections = Vec::new();
    let mut targets: Vec<ScanTarget> = Vec::new();
    for runtime in &state.apps {
        let unavailable = state.availability(runtime);
        let profiles = {
            let mut store = runtime
                .store
                .lock()
                .map_err(|_| anyhow!("the profile store for {} is unavailable", runtime.spec.id))?;
            // Only for an app that is actually installed. Reading an identity is
            // a write when it turns out to have changed, and a user who has the
            // Codex CLI but not the desktop app would otherwise find a registry
            // appearing for an app this tray cannot launch.
            if unavailable.is_none() && refresh_accounts(&mut store, runtime.spec) {
                let _ = store.save(&runtime.paths);
            }
            store.list().to_vec()
        };
        if unavailable.is_none() {
            targets.push(crate::instance_manager::scan_target(
                &*state.platform,
                runtime.spec,
            )?);
        }
        sections.push(AppSection {
            spec: runtime.spec,
            profiles,
            unavailable,
        });
    }

    // One sweep for every app, so the cost of the menu does not grow with the
    // number of apps installed.
    let (processes, scan_error) = scan_processes(state.platform.scan(&targets));
    let menu_error = combine_error_messages([runtime_error.map(str::to_string), scan_error]);
    let locale = state.general.locale();
    let rows = menu_rows(&sections, &processes, menu_error.as_deref(), locale);

    // Bail out before touching AppKit at all when nothing would change. This is
    // the whole fix for the flickering menu: a rebuild triggered by a click that
    // arrives after the menu is already open now does nothing at all.
    let next = menu_signature(&rows, locale);
    {
        let mut last = state
            .last_menu
            .lock()
            .map_err(|_| anyhow!("Agent Profiles menu state is unavailable"))?;
        let tray_exists = app.tray_by_id("main").is_some();
        if tray_exists && !should_replace_menu(last.as_deref(), &next) {
            return Ok(());
        }
        *last = Some(next);
    }

    let menu = tauri::menu::Menu::new(app)?;
    // Every profile stays on the top level, whatever it costs in height: this is
    // a menu opened to reach a profile, and a profile behind a submenu is one
    // hover further away than it was. An app's name is a label above its own
    // profiles, not a door in front of them.
    //
    // Items go in in `rows` order and nothing is skipped, so a row's position
    // here is its position in the finished menu — which is what lets the type
    // size be set afterwards without matching on labels.
    let mut profile_rows = Vec::new();
    for (index, row) in rows.iter().enumerate() {
        // The id is already the discriminant everywhere else in this file, so it
        // stays the discriminant here: only a profile row parses into an action,
        // and only a profile row carries a status dot.
        if parse_row_id(&row.id).is_some() {
            profile_rows.push(index);
            menu.append(&tauri::menu::IconMenuItem::with_id(
                app,
                &row.id,
                &row.text,
                row.enabled,
                Some(status_dot(row.running)),
                None::<&str>,
            )?)?;
        } else {
            menu.append(&tauri::menu::MenuItem::with_id(
                app,
                &row.id,
                &row.text,
                row.enabled,
                None::<&str>,
            )?)?;
        }
    }
    let strings = crate::general::tray_strings(locale);
    menu.append(&tauri::menu::PredefinedMenuItem::separator(app)?)?;
    menu.append(&tauri::menu::MenuItem::with_id(
        app,
        "manage",
        strings.settings,
        true,
        None::<&str>,
    )?)?;
    menu.append(&tauri::menu::MenuItem::with_id(
        app,
        "quit_app",
        strings.quit,
        true,
        None::<&str>,
    )?)?;

    let tray = if let Some(tray) = app.tray_by_id("main") {
        tray.set_menu(Some(menu))?;
        Some(tray)
    } else {
        let icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray/tray-icon.png"))?;
        let tray = tauri::tray::TrayIconBuilder::with_id("main")
            .icon(icon)
            .menu(&menu)
            .tooltip("Agent Profiles");
        // Shadow rather than mutate: only macOS rebinds this, and a `mut` that no
        // other platform uses is an error under `-D warnings` on Windows and Linux.
        #[cfg(target_os = "macos")]
        let tray = tray.icon_as_template(true);
        Some(tray.build(app)?)
    };

    // After the menu is attached, never before: the items have to exist as
    // `NSMenuItem`s before their type can be set. A no-op off macOS, where the
    // platform gives no way to say this at all.
    #[cfg(target_os = "macos")]
    if let Some(tray) = tray {
        crate::platform::macos::set_row_type_size(&tray, profile_rows, PROFILE_TYPE_SIZE);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = (tray, profile_rows);

    Ok(())
}

/// Re-asserts every profile's desktop identity. A no-op everywhere but Linux.
pub fn sync_identities(state: &AppState) {
    for runtime in &state.apps {
        if !runtime.spec.capabilities.desktop_identity {
            continue;
        }
        let Ok(store) = runtime.store.lock() else {
            continue;
        };
        for profile in store.list() {
            let _ = state.platform.register_identity(
                runtime.spec,
                &profile.label,
                &wm_class(runtime.spec.id, &profile.id),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_spec;
    use crate::paths::Paths;
    use std::path::PathBuf;

    fn profiles(labels: &[&str]) -> Vec<Profile> {
        std::iter::once(Profile {
            id: "default".into(),
            label: "Default".into(),
            path: PathBuf::from("/stock"),
            is_default: true,
            account: None,
        })
        .chain(labels.iter().enumerate().map(|(i, label)| Profile {
            id: format!("id{i}"),
            label: (*label).into(),
            path: PathBuf::from("/p").join(format!("id{i}")),
            is_default: false,
            account: None,
        }))
        .collect()
    }

    fn section(spec: &'static AppSpec, profiles: Vec<Profile>) -> AppSection {
        AppSection {
            spec,
            profiles,
            unavailable: None,
        }
    }

    /// The reason a real platform hands back, path and all — `macos.rs` reports
    /// `"{product} was not found at {path}"`. A short stand-in here would have
    /// hidden the width the menu actually takes.
    fn missing(spec: &'static AppSpec) -> AppSection {
        AppSection {
            spec,
            profiles: profiles(&[]),
            unavailable: Some(Unavailable::new(
                format!("{} is not installed", spec.product),
                format!(
                    "{} was not found at /Applications/{}.app/Contents/MacOS/{}",
                    spec.product, spec.product, spec.product
                ),
            )),
        }
    }

    fn running(app_id: &'static str, dir: &str) -> RunningProcess {
        RunningProcess {
            app_id,
            pid: 777,
            profile_dir: Some(PathBuf::from(dir)),
        }
    }

    #[test]
    fn a_running_profile_gets_a_filled_marker_and_a_focus_action() {
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let rows = menu_rows(
            &sections,
            &[running("claude", "/p/id0")],
            None,
            crate::general::Locale::En,
        );
        let row = rows.iter().find(|r| r.id == "focus:claude:id0").unwrap();
        assert!(row.running, "running is a dot, never a glyph in the label");
        assert_eq!(row.text, "Kerja");
        assert!(row.enabled);
    }

    #[test]
    fn a_profile_offers_one_row_and_never_a_quit_beside_it() {
        // Quitting an app belongs to the app, not to this menu: a row that ends
        // someone's editor from a menu they opened to switch profiles is a
        // destructive action sitting where a navigational one was expected.
        // Running or not makes no difference to how many rows a profile gets.
        for processes in [vec![running("claude", "/p/id0")], vec![]] {
            let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
            let expected = sections[0].profiles.len();
            let rows = menu_rows(&sections, &processes, None, crate::general::Locale::En);
            assert_eq!(rows.len(), expected, "one profile is one row");
            assert!(!rows.iter().any(|r| r.id.starts_with("quit:")));
        }
    }

    #[test]
    fn a_stopped_profile_offers_launch() {
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        let row = rows.iter().find(|r| r.id == "launch:claude:id0").unwrap();
        assert!(!row.running);
    }

    #[test]
    fn no_row_draws_its_own_state_or_hierarchy_into_the_label() {
        // State is the row's image and hierarchy is the indent that image gives
        // it. Glyphs and padded labels were how both used to be faked.
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            section(&app_spec::CODEX, profiles(&["Pribadi"])),
        ];
        let rows = menu_rows(
            &sections,
            &[running("claude", "/p/id0")],
            None,
            crate::general::Locale::En,
        );
        assert!(rows
            .iter()
            .all(|r| !r.text.contains(['●', '○', '✓']) && r.text.trim() == r.text));
        assert_eq!(
            rows.iter().filter(|r| r.running).count(),
            1,
            "only the live profile gets the filled dot"
        );
    }

    #[test]
    fn the_running_dot_is_filled_and_the_stopped_one_is_a_ring() {
        let alpha_at = |image: &tauri::image::Image<'_>, x: u32, y: u32| {
            image.rgba()[((y * DOT + x) * 4 + 3) as usize]
        };
        let c = DOT / 2;

        let live = status_dot(true);
        let idle = status_dot(false);

        assert_eq!(alpha_at(&live, c, c), 255, "running is a solid disc");
        assert_eq!(alpha_at(&idle, c, c), 0, "stopped is hollow");
        // Both carry the same outer edge, so a profile starting does not make
        // the row's mark change size under the pointer.
        for image in [&live, &idle] {
            assert_eq!(alpha_at(image, c, c - 8), 255, "the rim is opaque");
            assert_eq!(alpha_at(image, 0, 0), 0, "the corners stay clear");
        }
    }

    #[test]
    fn with_one_app_installed_the_menu_has_no_headers_at_all() {
        // Zero regression for someone who only has Claude: the menu they see is
        // the flat list it was before a second app existed.
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            missing(&app_spec::CODEX),
        ];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert!(rows.iter().all(|r| !r.id.starts_with("header:")));
        assert!(rows.iter().all(|r| !r.text.starts_with(' ')));
    }

    /// An NSMenu is as wide as its widest row, and a menu is a launch list, not
    /// an inventory: "ChatGPT is not installed" offers nothing to click, so it
    /// earns no place stealing width from the profiles that do. The window
    /// still says why (`ProfileList`'s `Unavailable` row, `EmptyState`'s reason
    /// list) — the tray just doesn't repeat it.
    #[test]
    fn an_app_that_is_not_installed_contributes_no_rows_beside_a_working_one() {
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            missing(&app_spec::CODEX),
        ];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert!(
            rows.iter().all(|r| r.id != "error:codex"),
            "an uninstalled app should contribute no row to the tray"
        );
        assert!(rows.iter().all(|r| !r.text.contains("ChatGPT")));
        // Nothing about it is clickable, because nothing about it is there.
        assert!(rows.iter().all(|r| !r.id.contains(":codex:")));
        // The app that does work is untouched by the other's absence.
        assert!(rows.iter().any(|r| r.id == "launch:claude:id0"));
        // With only one working app left to show, it needs no header either.
        assert!(rows.iter().all(|r| !r.id.starts_with("header:")));
    }

    #[test]
    fn with_two_apps_installed_each_gets_a_header_above_its_profiles() {
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            section(&app_spec::CODEX, profiles(&["Pribadi"])),
        ];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert_eq!(rows[0].id, "header:claude");
        assert_eq!(rows[0].text, "Claude");
        assert!(!rows[0].enabled, "a header is a label, not an action");
        assert!(rows.iter().any(|r| r.id == "header:codex"));
        // Only profile rows carry a dot, and that dot is what indents them under
        // the header — nothing is padded into a label.
        assert!(rows
            .iter()
            .filter(|r| r.running)
            .all(|r| parse_row_id(&r.id).is_some()));
        assert!(rows.iter().all(|r| r.text.trim() == r.text));
    }

    #[test]
    fn the_same_profile_id_under_two_apps_produces_two_distinct_rows() {
        // Ids are unique only within one app's store — eight hex characters
        // drawn per app — so the same id under two apps is expected, not a
        // freak collision. It must not make one row shadow the other.
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["A"])),
            section(&app_spec::CODEX, profiles(&["B"])),
        ];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert!(rows.iter().any(|r| r.id == "launch:claude:id0"));
        assert!(rows.iter().any(|r| r.id == "launch:codex:id0"));
    }

    #[test]
    fn a_process_belonging_to_one_app_never_lights_up_the_other() {
        let sections = vec![
            section(&app_spec::CLAUDE, profiles(&["A"])),
            section(&app_spec::CODEX, profiles(&["B"])),
        ];
        // Same directory, but owned by Claude.
        let rows = menu_rows(
            &sections,
            &[running("claude", "/p/id0")],
            None,
            crate::general::Locale::En,
        );
        assert!(rows.iter().any(|r| r.id == "focus:claude:id0"));
        assert!(rows.iter().any(|r| r.id == "launch:codex:id0"));
    }

    #[test]
    fn when_nothing_is_installed_the_tray_shows_no_rows() {
        let sections = vec![missing(&app_spec::CLAUDE), missing(&app_spec::CODEX)];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert!(
            rows.is_empty(),
            "with nothing installed the tray has nothing to launch and nothing to say"
        );
    }

    #[test]
    fn a_runtime_error_disables_every_row_and_adds_an_explanation() {
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let rows = menu_rows(
            &sections,
            &[],
            Some("process list unavailable"),
            crate::general::Locale::En,
        );
        assert!(rows.iter().filter(|r| r.id != "error").all(|r| !r.enabled));
        assert!(rows
            .iter()
            .any(|r| r.text.contains("process list unavailable")));
    }

    #[test]
    fn profiles_sharing_an_account_are_marked_within_their_own_app() {
        let mut claude = profiles(&["A", "B"]);
        claude[1].account = Some("same".into());
        claude[2].account = Some("same".into());
        // An identical string under the other app must NOT be treated as a clash:
        // a Claude uuid and a ChatGPT account id share no namespace.
        let mut codex = profiles(&["C"]);
        codex[1].account = Some("same".into());

        let sections = vec![
            section(&app_spec::CLAUDE, claude),
            section(&app_spec::CODEX, codex),
        ];
        let rows = menu_rows(&sections, &[], None, crate::general::Locale::En);
        assert_eq!(
            rows.iter()
                .filter(|r| r.text.contains("same account"))
                .count(),
            2
        );
    }

    #[test]
    fn a_row_id_survives_a_round_trip() {
        let id = row_id("launch", "claude", "abc-123");
        assert_eq!(parse_row_id(&id), Some(("launch", "claude", "abc-123")));
    }

    #[test]
    fn a_profile_id_containing_a_colon_still_parses_whole() {
        // Ids are eight hex characters today, and the socket budget is why. The
        // parser splitting on the first two colons only is what keeps that an
        // implementation detail rather than a rule the id format has to honour.
        let id = row_id("launch", "claude", "weird:id");
        assert_eq!(parse_row_id(&id), Some(("launch", "claude", "weird:id")));
    }

    #[test]
    fn a_row_that_is_not_an_action_does_not_parse_as_one() {
        assert_eq!(parse_row_id("manage"), None);
        assert_eq!(parse_row_id("quit_app"), None);
    }

    #[test]
    fn a_scan_failure_keeps_the_empty_fallback_and_exposes_its_reason() {
        let (processes, reason) = scan_processes(Err(anyhow!("process list unavailable")));
        assert!(processes.is_empty());
        assert_eq!(
            reason.as_deref(),
            Some("Could not scan running instances: process list unavailable")
        );
    }

    #[test]
    fn opening_gestures_request_a_rebuild_but_repeats_do_not() {
        use tauri::tray::{MouseButton, MouseButtonState, TrayIconEvent};
        use tauri::{PhysicalPosition, Rect};

        let id: tauri::tray::TrayIconId = "main".into();
        let position = PhysicalPosition::new(0.0, 0.0);
        let click = TrayIconEvent::Click {
            id: id.clone(),
            position,
            rect: Rect::default(),
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
        };
        assert!(should_rebuild_for_event(&click));

        // macOS never delivers Click when a menu is attached; Enter is the only
        // signal that the menu is about to open.
        let enter = TrayIconEvent::Enter {
            id: id.clone(),
            position,
            rect: Rect::default(),
        };
        assert!(should_rebuild_for_event(&enter));

        let other_events = [
            TrayIconEvent::DoubleClick {
                id: id.clone(),
                position,
                rect: Rect::default(),
                button: MouseButton::Left,
            },
            TrayIconEvent::Move {
                id: id.clone(),
                position,
                rect: Rect::default(),
            },
            TrayIconEvent::Leave {
                id,
                position,
                rect: Rect::default(),
            },
        ];
        assert!(other_events
            .iter()
            .all(|event| !should_rebuild_for_event(event)));
    }

    #[test]
    fn an_unchanged_menu_is_never_replaced() {
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let first = signature(&menu_rows(&sections, &[], None, crate::general::Locale::En));
        assert!(should_replace_menu(None, &first));

        let same = signature(&menu_rows(&sections, &[], None, crate::general::Locale::En));
        assert!(!should_replace_menu(Some(&first), &same));
    }

    #[test]
    fn a_profile_going_live_does_replace_the_menu() {
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let stopped = signature(&menu_rows(&sections, &[], None, crate::general::Locale::En));
        let live = signature(&menu_rows(
            &sections,
            &[running("claude", "/p/id0")],
            None,
            crate::general::Locale::En,
        ));
        assert!(should_replace_menu(Some(&stopped), &live));
    }

    #[test]
    fn a_second_app_appearing_replaces_the_menu() {
        // Installing ChatGPT while this app runs must be picked up on the next
        // hover, not on the next restart.
        let one = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            missing(&app_spec::CODEX),
        ];
        let two = vec![
            section(&app_spec::CLAUDE, profiles(&["Kerja"])),
            section(&app_spec::CODEX, profiles(&[])),
        ];
        assert!(should_replace_menu(
            Some(&signature(&menu_rows(
                &one,
                &[],
                None,
                crate::general::Locale::En
            ))),
            &signature(&menu_rows(&two, &[], None, crate::general::Locale::En))
        ));
    }

    #[test]
    fn refreshing_accounts_reports_only_actual_changes() {
        let d = tempfile::tempdir().unwrap();
        let paths = Paths::new(d.path().join("root"));
        let def = d.path().join("stock");
        std::fs::create_dir_all(&def).unwrap();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        store.add("Kerja", &paths).unwrap();

        assert!(!refresh_accounts(&mut store, &app_spec::CLAUDE));

        let profile = store.list()[1].clone();
        std::fs::write(
            profile.path.join("config.json"),
            r#"{"lastKnownAccountUuid":"abc-123"}"#,
        )
        .unwrap();
        assert!(refresh_accounts(&mut store, &app_spec::CLAUDE));
        assert_eq!(
            store.get(&profile.id).unwrap().account.as_deref(),
            Some("abc-123")
        );
        assert!(!refresh_accounts(&mut store, &app_spec::CLAUDE));
    }

    #[test]
    fn each_app_reads_its_own_identity_file() {
        // The same directory holds a Codex account id; read through Claude's spec
        // it must stay invisible rather than being mistaken for an account.
        let d = tempfile::tempdir().unwrap();
        let paths = Paths::new(d.path().join("root"));
        let def = d.path().join("stock");
        std::fs::create_dir_all(&def).unwrap();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let p = store.add("Kerja", &paths).unwrap();
        std::fs::write(
            p.path.join("auth.json"),
            r#"{"tokens":{"account_id":"acct-9"}}"#,
        )
        .unwrap();

        assert!(!refresh_accounts(&mut store, &app_spec::CLAUDE));
        assert!(refresh_accounts(&mut store, &app_spec::CODEX));
        assert_eq!(store.get(&p.id).unwrap().account.as_deref(), Some("acct-9"));
    }

    #[test]
    fn the_same_account_suffix_is_translated() {
        use crate::general::Locale;
        // Two profiles on one account is what produces the suffix; the fixture
        // helpers in this module already build that case for the English test.
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let rows_en = menu_rows(&sections, &[], None, Locale::En);
        let rows_ja = menu_rows(&sections, &[], None, Locale::Ja);
        assert_eq!(
            rows_en.len(),
            rows_ja.len(),
            "translating labels must not change the shape of the menu"
        );
    }

    #[test]
    fn switching_the_language_forces_a_menu_rebuild() {
        use crate::general::Locale;
        // Same rows, different locale: the static Settings…/Quit items differ, so
        // the rebuild must not be skipped.
        let sections = vec![section(&app_spec::CLAUDE, profiles(&["Kerja"]))];
        let rows = menu_rows(&sections, &[], None, Locale::En);
        let en = menu_signature(&rows, Locale::En);
        let ja = menu_signature(&rows, Locale::Ja);
        assert!(
            should_replace_menu(Some(&en), &ja),
            "a language change must force the tray to rebuild so Settings…/Quit retranslate"
        );
        // And identical inputs must still be a no-op (the anti-flicker optimization).
        assert!(!should_replace_menu(
            Some(&en),
            &menu_signature(&rows, Locale::En)
        ));
    }

    #[test]
    fn runtime_and_scan_errors_are_combined_into_one_visible_menu_reason() {
        let reason = combine_error_messages([
            Some("launch failed".to_string()),
            Some("scan failed".to_string()),
            None,
        ]);
        assert_eq!(reason.as_deref(), Some("launch failed; scan failed"));
    }
}
