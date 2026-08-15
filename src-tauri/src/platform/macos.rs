use crate::app_spec::{AppSpec, Locations};
use crate::platform::{
    unix_ps, FocusHint, FocusOutcome, Platform, RunningProcess, ScanTarget, DATA_DIR_NAME,
};
use anyhow::{anyhow, Result};
use std::path::{Path, PathBuf};

pub struct MacOs;

fn home() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var("HOME").map_err(|_| anyhow!("HOME is not set"))?,
    ))
}

fn data_root_in(home: &Path) -> PathBuf {
    home.join("Library")
        .join("Application Support")
        .join(DATA_DIR_NAME)
}

fn check_binary(bin: &Path, product: &str) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(bin)
        .map_err(|_| anyhow!("{product} was not found at {}", bin.display()))?;
    if meta.permissions().mode() & 0o111 == 0 {
        return Err(anyhow!("{} is not executable", bin.display()));
    }
    Ok(())
}

/// Resolving the row for this platform in one place, so every caller reports the
/// same thing when an app has not been declared here.
fn here<'a>(locations: &'a Locations, product: &str) -> Result<&'a crate::app_spec::MacLocation> {
    locations
        .macos
        .as_ref()
        .ok_or_else(|| anyhow!("{product} has not been declared for macOS"))
}

impl Platform for MacOs {
    fn declared_here(&self, locations: &Locations) -> bool {
        locations.macos.is_some()
    }

    fn data_root(&self) -> Result<PathBuf> {
        Ok(data_root_in(&home()?))
    }

    fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf> {
        Ok(home()?.join(here(locations, "this app")?.default_profile))
    }

    fn binary(&self, locations: &Locations, product: &str) -> Result<PathBuf> {
        let bin = PathBuf::from(here(locations, product)?.binary);
        check_binary(&bin, product)?;
        Ok(bin)
    }

    fn process_marker(&self, locations: &Locations) -> Result<String> {
        Ok(here(locations, "this app")?.binary.to_string())
    }

    fn scan(&self, targets: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
        unix_ps::scan(targets)
    }

    fn link(&self, source: &Path, target: &Path) -> Result<()> {
        std::os::unix::fs::symlink(source, target)?;
        Ok(())
    }

    fn focus(&self, pid: i32, _hint: &FocusHint) -> Result<FocusOutcome> {
        use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
        let app = NSRunningApplication::runningApplicationWithProcessIdentifier(pid)
            .ok_or_else(|| anyhow!("no running application with pid {pid}"))?;
        app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        Ok(FocusOutcome::Focused)
    }

    fn quit(&self, pid: i32) -> Result<()> {
        crate::platform::unix_signal_quit(pid)
    }

    fn register_identity(
        &self,
        _spec: &AppSpec,
        _profile_label: &str,
        _wm_class: &str,
    ) -> Result<()> {
        Ok(())
    }
}

/// Sets the profile rows of a tray menu one step below the menu's own type size.
///
/// muda has no opinion about type size and no way to express one: a menu item
/// takes a `String`, and AppKit sets it in the menu font. The size lives on
/// `NSMenuItem.attributedTitle`, so the item has to be reached directly — down
/// through the tray's `NSStatusItem` to the `NSMenu` it owns. Indices rather
/// than titles, because the menu is built from `rows` in order and two profiles
/// may legitimately share a label.
///
/// Only the profile rows shrink. `Settings…` and `Quit` are commands
/// rather than data and stay at the size every other menu on the bar uses, which
/// is also what keeps the smaller rows readable as a deliberate size rather than
/// as a menu that came out wrong.
pub(crate) fn set_row_type_size<R: tauri::Runtime>(
    tray: &tauri::tray::TrayIcon<R>,
    rows: Vec<usize>,
    points: f64,
) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2_app_kit::{NSFont, NSFontAttributeName};
    use objc2_foundation::{MainThreadMarker, NSAttributedString, NSDictionary, NSString};

    // `Retained` is not `Send`, so nothing crosses back out of the closure; the
    // whole traversal happens inside it, which is also the main thread AppKit
    // requires for any of this.
    let _ = tray.with_inner_tray_icon(move |inner| {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        let Some(status_item) = inner.ns_status_item() else {
            return;
        };
        let Some(menu) = status_item.menu(mtm) else {
            return;
        };
        let font = NSFont::menuFontOfSize(points);
        let attributes = NSDictionary::from_slices(&[unsafe { NSFontAttributeName }], &[&*font]);
        // `from_slices` types the values as `NSFont`; an attribute dictionary is
        // heterogeneous by definition, and this one holds exactly what it says.
        let attributes: Retained<NSDictionary<NSString, AnyObject>> =
            unsafe { Retained::cast_unchecked(attributes) };
        for index in rows {
            let Some(item) = menu.itemAtIndex(index as isize) else {
                continue;
            };
            let title = item.title();
            // Safe here: the dictionary holds the one attribute key it was just
            // built with, and the value under it really is an `NSFont`.
            let styled = unsafe { NSAttributedString::new_with_attributes(&title, &attributes) };
            item.setAttributedTitle(Some(&styled));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_spec;

    #[test]
    fn our_own_data_root_hangs_off_application_support() {
        assert_eq!(
            data_root_in(Path::new("/Users/h")),
            PathBuf::from("/Users/h/Library/Application Support/Agent Profiles")
        );
    }

    #[test]
    fn each_app_declares_where_its_own_stock_profile_lives() {
        // Claude keeps it under Application Support, Codex in a dotfile
        // directory. Resolving both through the home directory is what lets one
        // backend serve both without branching on the app.
        let home = Path::new("/Users/h");
        assert_eq!(
            home.join(
                app_spec::CLAUDE
                    .locations
                    .macos
                    .as_ref()
                    .unwrap()
                    .default_profile
            ),
            PathBuf::from("/Users/h/Library/Application Support/Claude")
        );
        assert_eq!(
            home.join(
                app_spec::CODEX
                    .locations
                    .macos
                    .as_ref()
                    .unwrap()
                    .default_profile
            ),
            PathBuf::from("/Users/h/.codex")
        );
    }

    #[test]
    fn an_app_not_declared_here_yields_no_marker_rather_than_an_empty_one() {
        // An empty marker is a substring of every line of the process table, so
        // the tempting default would attribute the first process on the machine
        // to this app: every profile would read as running, launching would be
        // refused forever, and Quit would signal a stranger.
        let undeclared = crate::app_spec::Locations {
            macos: None,
            linux: None,
            windows: None,
        };
        assert!(MacOs.process_marker(&undeclared).is_err());
        assert_eq!(
            MacOs.process_marker(&app_spec::CLAUDE.locations).unwrap(),
            "/Applications/Claude.app/Contents/MacOS/Claude"
        );
    }

    #[test]
    fn a_missing_binary_is_rejected_by_name_and_product() {
        let err = check_binary(Path::new("/nope/Claude"), "Claude Desktop")
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("/nope/Claude"),
            "must name the path, got: {err}"
        );
        assert!(
            err.contains("Claude Desktop"),
            "must name the product, got: {err}"
        );
    }

    #[test]
    fn a_present_but_non_executable_binary_is_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let d = tempfile::tempdir().unwrap();
        let bin = d.path().join("Claude");
        std::fs::write(&bin, b"not really a binary").unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(check_binary(&bin, "Claude Desktop").is_err());

        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(check_binary(&bin, "Claude Desktop").is_ok());
    }
}
