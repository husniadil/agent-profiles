use crate::app_spec::{AppSpec, Locations};
use crate::platform::{
    unix_ps, FocusHint, FocusOutcome, Platform, Power, RunningProcess, ScanTarget, Unavailable,
    DATA_DIR_NAME,
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
    let meta = std::fs::metadata(bin).map_err(|_| {
        anyhow!(Unavailable::new(
            format!("{product} is not installed"),
            format!("{product} was not found at {}", bin.display()),
        ))
    })?;
    if meta.permissions().mode() & 0o111 == 0 {
        return Err(anyhow!(Unavailable::new(
            format!("{product} is installed but cannot be run"),
            format!(
                "{product} is installed but {} is not executable",
                bin.display()
            ),
        )));
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

/// Reads `pmset -g batt`.
///
/// Shelling out rather than reaching for IOKit through `objc2`: this is two
/// lines of text asked for once every sweep, and the IOKit version is a
/// `CFDictionary` walk that would have to be kept correct across macOS releases
/// for the same two numbers.
///
/// ponytail: shells out once per sweep. Swap for `IOPSCopyPowerSourcesInfo` if
/// the process spawn ever shows up in a profile.
fn parse_batt(raw: &str) -> Power {
    let external = raw.contains("'AC Power'");
    // Only the battery line is trusted for the number. The remaining-time field
    // on the same line also carries digits, and a future macOS adding a line
    // that does would otherwise be read as a charge.
    let percent = raw
        .lines()
        .find(|line| line.trim_start().starts_with("-InternalBattery"))
        .and_then(|line| line.split_once('%'))
        .and_then(|(before, _)| {
            let digits: String = before
                .chars()
                .rev()
                .take_while(char::is_ascii_digit)
                .collect();
            digits.chars().rev().collect::<String>().parse().ok()
        });
    Power { percent, external }
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

    fn can_hold_awake(&self) -> bool {
        true
    }

    fn can_schedule_wake(&self) -> bool {
        true
    }

    fn app_icon(&self, path: &str) -> Option<String> {
        let png = render_app_icon_png(path)?;
        Some(format!(
            "data:image/png;base64,{}",
            crate::schedule::base64_encode(&png)
        ))
    }

    fn set_wakes(&self, cancel: &[String], schedule: &[String]) -> Result<()> {
        // Nothing to cancel and nothing to arm: no privileged step, no prompt.
        // The caller reaches here only when the wake set actually changed, so an
        // app-only edit or a no-op save never gets this far.
        let Some(command) = pmset_batch_command(cancel, schedule) else {
            return Ok(());
        };
        // Root, waited-for, and reported: a silent failure here would leave the
        // window claiming a schedule the machine will not honour. `privileged_now`
        // is the same one-shot elevation `restore_sleep` uses.
        run_osascript(&privileged_now(&command))
    }

    fn refresh_launch_agent(&self, plan: &crate::schedule::WakePlan) -> Result<()> {
        // The launchd half needs no root. Replace any existing agent: write the
        // plist, then bootout-and-bootstrap so a changed schedule takes effect now
        // rather than at the next login.
        if let Some(parent) = plan.plist_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&plan.plist_path, plan.plist_xml.as_bytes())?;
        let domain = launchd_gui_domain()?;
        // A bootout of an agent that is not loaded errors; ignore it — the point
        // is only that nothing stale remains before the bootstrap.
        let _ = launchctl(&[
            "bootout",
            &format!("{domain}/{}", plist_label(&plan.plist_path)),
        ]);
        launchctl(&["bootstrap", &domain, &plan.plist_path.display().to_string()])
    }

    fn remove_launch_agent(&self) -> Result<()> {
        // No root: the LaunchAgent is the user's own. `bootout` and the unlink are
        // both best-effort — a disable must not fail because an agent the user
        // already removed by hand is missing. The one-off wakes are cancelled
        // separately, by `set_wakes`, which is the half that needs the password.
        let home = home()?;
        let plist = crate::paths::launch_agent_plist(&home, BUNDLE_ID);
        if plist.exists() {
            let domain = launchd_gui_domain()?;
            let _ = launchctl(&["bootout", &format!("{domain}/{}", plist_label(&plist))]);
            let _ = std::fs::remove_file(&plist);
        }
        Ok(())
    }

    fn needs_authorization(&self) -> bool {
        // `pmset -a disablesleep` is root's, and nothing short of root can set
        // it. The hold is a flag file precisely so the password is asked once
        // rather than on every sweep — see `start_awake_watchdog`.
        true
    }

    fn power(&self) -> Result<Power> {
        let out = std::process::Command::new("pmset")
            .args(["-g", "batt"])
            .output()?;
        Ok(parse_batt(&String::from_utf8_lossy(&out.stdout)))
    }

    fn thermal(&self) -> crate::platform::Thermal {
        use crate::platform::Thermal;
        use objc2_foundation::{NSProcessInfo, NSProcessInfoThermalState};

        match NSProcessInfo::processInfo().thermalState() {
            NSProcessInfoThermalState::Nominal => Thermal::Nominal,
            NSProcessInfoThermalState::Fair => Thermal::Fair,
            NSProcessInfoThermalState::Serious => Thermal::Serious,
            NSProcessInfoThermalState::Critical => Thermal::Critical,
            // A level this build has never heard of. Reported as unknown rather
            // than guessed at in either direction: a future macOS adding a step
            // above `Critical` must not read as cool, and one added below
            // `Nominal` must not read as hot.
            _ => Thermal::Unknown,
        }
    }

    fn start_awake_watchdog(&self, watchdog: &crate::platform::Watchdog) -> Result<()> {
        run_osascript(&privileged_background(&watchdog_script(watchdog)))
    }

    fn restore_sleep(&self) -> Result<()> {
        // Deliberately not the watchdog: someone who has turned the feature off
        // and is only digging themselves out of a stranded run should not end up
        // with a root loop running for the rest of the session.
        run_osascript(&privileged_now("pmset -a disablesleep 0"))
    }
}

/// How often the root loop looks at the flag and at the app's pid.
///
/// The upper bound on how long sleep stays disabled after the app quits, so it
/// is short; it is also a root shell waking on a timer, so it is not shorter
/// than it needs to be. The app itself only revises its decision every fifteen
/// seconds, so anything below this buys nothing.
const WATCHDOG_POLL_SECONDS: u32 = 3;

/// The body of the privileged loop.
///
/// Held to one line because it is embedded in an AppleScript string literal,
/// which cannot span lines. Generated here rather than shipped as a file and
/// elevated: anything under Application Support is user-writable, so a script
/// on disk that gets run `with administrator privileges` is a persistent root
/// escalation for every process running as the user, not a power-management
/// feature.
///
/// Three rules this body must keep, each of which has a test:
///
/// 1. The flag is tested for existence, never read. Its contents are attacker-
///    controlled in the only sense that matters — any user process can write it.
/// 2. The app is identified by pid *and* process start time. Pids are recycled.
/// 3. The breadcrumb is written before the first possible hold, so a process
///    that dies abruptly still leaves behind who owned the setting.
fn watchdog_script(watchdog: &crate::platform::Watchdog) -> String {
    // `-` rather than an empty string for "nothing to reclaim": always exactly
    // one character, so there is no case where the test below sees a blank.
    let reclaim = match watchdog.reclaimed_prior {
        Some(1) => "1",
        Some(_) => "0",
        None => "-",
    };
    [
        "set -u".to_string(),
        format!("FLAG='{}'", watchdog.flag.display()),
        format!("CRUMB='{}'", watchdog.breadcrumb.display()),
        format!("PID={}", watchdog.app_pid),
        format!("RECLAIM={reclaim}"),
        // Read here, not handed in: `lstart` is a date string full of spaces and
        // is the one variable-length value that would otherwise be interpolated.
        r#"START=$(ps -o lstart= -p "$PID" 2>/dev/null)"#.to_string(),
        // The app died between the password prompt and here. Nothing to watch.
        r#"[ -n "$START" ] || exit 0"#.to_string(),
        // The reclaim path is unconditional and first: a previous run died
        // holding the setting, so the machine is stuck awake right now and every
        // later rule in this loop would decide to leave it that way.
        r#"if [ "$RECLAIM" = - ]"#.to_string(),
        r#"then PRIOR=$(pmset -g | awk '/SleepDisabled/{print $2}'); [ "$PRIOR" = 1 ] || PRIOR=0"#
            .to_string(),
        r#"else PRIOR="$RECLAIM"; pmset -a disablesleep "$PRIOR""#.to_string(),
        "fi".to_string(),
        r#"printf 'prior=%s\n' "$PRIOR" > "$CRUMB""#.to_string(),
        "HELD=0".to_string(),
        r#"while kill -0 "$PID" 2>/dev/null && [ "$(ps -o lstart= -p "$PID" 2>/dev/null)" = "$START" ]"#
            .to_string(),
        // `do` opens the body directly rather than sitting on its own: every
        // element here is joined with `; `, and `do;` is a syntax error.
        r#"do if [ -e "$FLAG" ]; then WANT=1; else WANT=0; fi"#.to_string(),
        // Edge-triggered: written only on a transition, so a user toggling
        // `pmset` by hand is not fought every three seconds.
        r#"if [ "$WANT" != "$HELD" ]"#.to_string(),
        r#"then if [ "$WANT" = 1 ]; then pmset -a disablesleep 1; else pmset -a disablesleep "$PRIOR"; fi; HELD="$WANT""#
            .to_string(),
        "fi".to_string(),
        format!("sleep {WATCHDOG_POLL_SECONDS}"),
        "done".to_string(),
        r#"[ "$HELD" = 0 ] || pmset -a disablesleep "$PRIOR""#.to_string(),
        r#"rm -f "$CRUMB""#.to_string(),
    ]
    .join("; ")
}

/// Escapes a shell command for an AppleScript string literal.
///
/// Only two characters mean anything inside one. This is safe as the single
/// escaping layer because `paths::unquotable_refusal` has already turned away
/// any data root carrying a quote, a backslash or a newline — so every `"` and
/// `\` left in the string is one this file wrote.
fn as_applescript_string(shell: &str) -> String {
    format!("\"{}\"", shell.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Runs `shell` as root and returns immediately, leaving it running.
///
/// `do shell script` waits for its command's output to close, so the loop is
/// backgrounded with every stream redirected — otherwise `osascript` would
/// block for the entire session and the app would never finish starting.
fn privileged_background(shell: &str) -> String {
    format!(
        "do shell script {} with administrator privileges",
        as_applescript_string(&format!("{{ {shell} ; }} >/dev/null 2>&1 &"))
    )
}

/// Runs `shell` as root and waits for it.
///
/// Deliberately not backgrounded, unlike [`privileged_background`]: this is used
/// for one-shot repairs, where returning before the command has run would report
/// a success nobody has verified — on the one screen whose job is to tell the
/// user the truth about whether their Mac can sleep.
fn privileged_now(shell: &str) -> String {
    format!(
        "do shell script {} with administrator privileges",
        as_applescript_string(shell)
    )
}

/// Runs one AppleScript through `osascript`.
///
/// The script is a single argv element, so no shell of ours ever sees it.
fn run_osascript(applescript: &str) -> Result<()> {
    let out = std::process::Command::new("osascript")
        .arg("-e")
        .arg(applescript)
        .output()?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    // -128 is AppleScript's "user cancelled". Worth its own sentence: it is not
    // a failure, it is an answer, and reporting it as an error the user has to
    // diagnose would be wrong.
    if stderr.contains("-128") {
        anyhow::bail!("the administrator password prompt was cancelled");
    }
    anyhow::bail!("could not start the keep-awake helper: {}", stderr.trim())
}

/// The single shell command that cancels one batch of one-off wakes and arms
/// another, or `None` when there is nothing to do.
///
/// `wake`, not `wakeorpoweron`: the `poweron` half only ever mattered for a Mac
/// that had been fully shut down, and it needs AC to fire at all — Apple Silicon
/// drops it even on AC. `wake` covers a merely-sleeping Mac, which keeps its RTC
/// powered from the battery, so this is what actually fires unplugged. The
/// tradeoff is explicit, not accidental: a Mac that is shut down at the
/// scheduled time now simply stays off, where it previously made an unreliable
/// attempt to power on.
///
/// A free function so the batching and quoting can be asserted without a running
/// `pmset` or a password prompt: every cancel comes first, every arm after, all
/// joined with `; ` so one elevation runs the lot. Each datetime is one of our
/// own formatted strings (`MM/dd/yy HH:mm:ss`) carrying no quote or backslash, so
/// the surrounding double quotes are the whole of the quoting needed — the space
/// is all that has to be protected.
fn pmset_batch_command(cancel: &[String], schedule: &[String]) -> Option<String> {
    if cancel.is_empty() && schedule.is_empty() {
        return None;
    }
    let mut commands = Vec::with_capacity(cancel.len() + schedule.len());
    for dt in cancel {
        commands.push(format!("pmset schedule cancel wake \"{dt}\""));
    }
    for dt in schedule {
        commands.push(format!("pmset schedule wake \"{dt}\""));
    }
    Some(commands.join("; "))
}

/// Must match `identifier` in `tauri.conf.json`. Named here rather than read from
/// the running app so the platform layer stays free of a `tauri::AppHandle`.
const BUNDLE_ID: &str = "com.husniadil.agent-profiles";

/// The LaunchAgent label is the plist file stem, i.e. `<bundle-id>.schedule`.
fn plist_label(plist: &Path) -> String {
    plist
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string()
}

/// The launchd domain the user's own agents live in: `gui/<uid>`.
fn launchd_gui_domain() -> Result<String> {
    // getuid never fails and returns the real user's id, which is the domain a
    // per-user LaunchAgent is bootstrapped into.
    let uid = unsafe { libc::getuid() };
    Ok(format!("gui/{uid}"))
}

/// One `launchctl` invocation as the user. argv, never a shell — the plist path
/// is a single element, so no quoting of the home directory is involved.
fn launchctl(args: &[&str]) -> Result<()> {
    let out = std::process::Command::new("launchctl")
        .args(args)
        .output()?;
    if out.status.success() {
        return Ok(());
    }
    anyhow::bail!(
        "launchctl {} failed: {}",
        args.first().copied().unwrap_or_default(),
        String::from_utf8_lossy(&out.stderr).trim()
    )
}

/// The application at `path`'s icon, scaled to a small fixed square and encoded
/// as PNG, or `None` if any step of the AppKit pipeline comes back empty.
///
/// `NSWorkspace::iconForFile` hands back the multi-representation `NSImage`
/// LaunchServices already has cached, so this is a lookup rather than a disk
/// read. It is drawn once into an offscreen bitmap of the target size — the
/// picker shows a thumbnail, not the 512-point original — and re-encoded to PNG
/// for a `data:` URI.
///
/// The whole traversal runs inside an autorelease pool: every `Retained` AppKit
/// object built here is temporary, and without a pool they would pile up until
/// the thread's outermost pool drains.
///
/// Offscreen `NSImage` drawing via `lockFocus`/`unlockFocus` does not require the
/// main thread — it renders into a private bitmap context this call owns and
/// never installs into a view — so this is safe to run on the Tauri command
/// worker `list_applications` calls it from. Backed by
/// `every_installed_app_renders_an_icon_off_the_main_thread`, which renders a
/// whole real `/Applications` scan on a `cargo test` worker thread (never the
/// main thread) rather than trusting the claim on Apple's word alone.
// `lockFocus`/`unlockFocus` are deprecated in favour of resolution-independent
// block drawing, which matters for vector art rendered at unknown scale. This
// draws a system icon into a fixed 36-point thumbnail once, so the concern does
// not apply and the two calls remain the simplest offscreen route; the raster
// output is re-encoded straight to PNG.
#[allow(deprecated)]
fn render_app_icon_png(path: &str) -> Option<Vec<u8>> {
    use objc2::rc::autoreleasepool;
    use objc2::AnyThread;
    use objc2_app_kit::{
        NSBitmapImageFileType, NSBitmapImageRep, NSCompositingOperation, NSImage, NSWorkspace,
    };
    use objc2_foundation::{NSDictionary, NSPoint, NSRect, NSSize, NSString};

    /// The square the icon is scaled into. Matches the picker's row height, so a
    /// Retina display still gets a crisp thumbnail from a modest payload.
    const ICON_POINTS: f64 = 36.0;

    autoreleasepool(|_| {
        let source = NSWorkspace::sharedWorkspace().iconForFile(&NSString::from_str(path));

        let size = NSSize::new(ICON_POINTS, ICON_POINTS);
        let scaled = NSImage::initWithSize(NSImage::alloc(), size);
        let dest = NSRect::new(NSPoint::new(0.0, 0.0), size);
        // A zero source rect means "the whole image", which lets AppKit choose
        // the representation that best fits the destination square.
        let whole = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(0.0, 0.0));

        scaled.lockFocus();
        source.drawInRect_fromRect_operation_fraction(
            dest,
            whole,
            NSCompositingOperation::SourceOver,
            1.0,
        );
        scaled.unlockFocus();

        let tiff = scaled.TIFFRepresentation()?;
        let rep = NSBitmapImageRep::imageRepWithData(&tiff)?;
        // SAFETY: the properties dictionary is empty, so it trivially satisfies
        // the "values of the correct type" requirement the binding documents.
        let png = unsafe {
            rep.representationUsingType_properties(NSBitmapImageFileType::PNG, &NSDictionary::new())
        }?;
        Some(png.to_vec())
    })
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

    fn sample_watchdog(reclaimed_prior: Option<u8>) -> String {
        let flag = PathBuf::from("/data/keep-awake.hold");
        let crumb = PathBuf::from("/data/keep-awake.owned");
        watchdog_script(&crate::platform::Watchdog {
            flag: &flag,
            breadcrumb: &crumb,
            reclaimed_prior,
            app_pid: 4242,
        })
    }

    #[test]
    fn the_loop_tests_the_flag_for_existence_and_never_reads_it() {
        // The flag lives in a user-writable folder and this shell runs as root.
        // Reading it — `$(cat $FLAG)`, `. $FLAG`, `eval` — would turn a power
        // setting into a persistent root shell for anything running as the user.
        let script = sample_watchdog(None);
        assert!(script.contains("[ -e \"$FLAG\" ]"), "got: {script}");
        for forbidden in ["cat ", "eval", "source ", "$(<"] {
            assert!(
                !script.contains(forbidden),
                "the loop must never {forbidden:?} anything: {script}"
            );
        }
    }

    #[test]
    fn the_loop_exits_when_the_app_goes_and_checks_the_start_time_too() {
        // `kill -0` alone is not enough. Pids are recycled, and a loop that
        // outlived its app would keep sleep disabled on behalf of whatever
        // inherited the number.
        let script = sample_watchdog(None);
        assert!(script.contains("kill -0 \"$PID\""), "got: {script}");
        assert!(
            script.contains("ps -o lstart= -p \"$PID\""),
            "got: {script}"
        );
        assert!(
            script.contains("= \"$START\""),
            "the start time must be compared: {script}"
        );
    }

    #[test]
    fn the_start_time_is_captured_by_the_loop_rather_than_handed_to_it() {
        // `lstart` is a date string full of spaces. Interpolating one into the
        // script is the one avoidable variable-length value in it, so the loop
        // reads its own.
        let script = sample_watchdog(None);
        assert!(script.contains("START=$(ps -o lstart="), "got: {script}");
        assert!(
            !script.contains("2026"),
            "no captured date may appear: {script}"
        );
    }

    #[test]
    fn a_run_that_finds_no_breadcrumb_adopts_the_live_setting_as_the_users_own() {
        // "Restore only what it took": a user who ran `sudo pmset -a
        // disablesleep 1` by hand must find it still set after this app quits.
        let script = sample_watchdog(None);
        assert!(script.contains("RECLAIM=-"), "got: {script}");
        assert!(
            script.contains("SleepDisabled"),
            "the live value must be read: {script}"
        );
    }

    #[test]
    fn a_run_that_finds_a_breadcrumb_resets_the_setting_before_anything_else() {
        // The defect the breadcrumb exists for: a panic leaves `disablesleep`
        // on, it survives reboot, and edge-triggered writes plus
        // restore-only-what-you-took would each independently decide to leave it
        // alone. Reclaiming has to be unconditional and first.
        let script = sample_watchdog(Some(0));
        assert!(script.contains("RECLAIM=0"), "got: {script}");
        let reclaim_at = script.find("pmset -a disablesleep \"$PRIOR\"").unwrap();
        let loop_at = script.find("while kill -0").unwrap();
        assert!(
            reclaim_at < loop_at,
            "the reset must happen before the loop: {script}"
        );
    }

    #[test]
    fn the_breadcrumb_is_written_before_the_loop_can_hold_anything() {
        // Written after the first hold, it would not exist during exactly the
        // window it is meant to cover.
        let script = sample_watchdog(None);
        let crumb_at = script.find("> \"$CRUMB\"").unwrap();
        let loop_at = script.find("while kill -0").unwrap();
        assert!(crumb_at < loop_at, "got: {script}");
    }

    #[test]
    fn the_script_is_one_line_with_no_characters_applescript_cannot_hold() {
        // It is embedded in an AppleScript string literal, which cannot span
        // lines. A newline here would truncate the loop mid-statement.
        let script = sample_watchdog(None);
        assert!(!script.contains('\n'), "got: {script}");
    }

    #[test]
    fn a_path_is_single_quoted_so_a_space_cannot_split_it() {
        let flag =
            PathBuf::from("/Users/a b/Library/Application Support/Agent Profiles/keep-awake.hold");
        let crumb =
            PathBuf::from("/Users/a b/Library/Application Support/Agent Profiles/keep-awake.owned");
        let script = watchdog_script(&crate::platform::Watchdog {
            flag: &flag,
            breadcrumb: &crumb,
            reclaimed_prior: None,
            app_pid: 1,
        });
        assert!(script.contains("FLAG='/Users/a b/"), "got: {script}");
    }

    #[test]
    fn the_generated_loop_is_valid_shell() {
        // The one artifact whose correctness the assertions above can only
        // circle: they check that certain substrings are present and ordered,
        // not that the result parses. `sh -n` reads the script and does not
        // execute it, so this is a syntax check and never a running loop —
        // which is also why it can assert against the real generated string
        // rather than a hand-written stand-in that could drift from it.
        for reclaim in [None, Some(0), Some(1)] {
            let script = sample_watchdog(reclaim);
            let checked = std::process::Command::new("sh")
                .arg("-n")
                .arg("-c")
                .arg(&script)
                .output()
                .expect("sh must be available");
            assert!(
                checked.status.success(),
                "reclaim {reclaim:?} generated unparseable shell: {}\n{script}",
                String::from_utf8_lossy(&checked.stderr)
            );
        }
    }

    #[test]
    fn the_backgrounded_wrapper_is_valid_shell_once_unescaped() {
        // The braces-and-ampersand wrapper is assembled separately from the loop
        // body, so it gets its own parse: a stray brace there would not show up
        // in the test above.
        let wrapped = format!("{{ {} ; }} >/dev/null 2>&1 &", sample_watchdog(None));
        let checked = std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&wrapped)
            .output()
            .expect("sh must be available");
        assert!(
            checked.status.success(),
            "the backgrounded form does not parse: {}",
            String::from_utf8_lossy(&checked.stderr)
        );
    }

    #[test]
    fn the_applescript_wrapper_escapes_every_quote_and_backslash() {
        // The two characters that mean anything inside an AppleScript string.
        // An unescaped quote would end the literal early and leave the rest of
        // the loop as AppleScript source.
        let wrapped = as_applescript_string(r#"echo "hi" \ there"#);
        assert_eq!(wrapped, r#""echo \"hi\" \\ there""#);
    }

    #[test]
    fn the_watchdog_is_backgrounded_so_osascript_returns_at_once() {
        // `do shell script` waits for its command's output to close. Without
        // this the password prompt would be followed by a hang lasting the
        // whole session, because the loop only ends when the app does.
        let applescript = privileged_background("true");
        assert!(
            applescript.contains("with administrator privileges"),
            "got: {applescript}"
        );
        assert!(
            applescript.contains(">/dev/null 2>&1 &"),
            "got: {applescript}"
        );
    }

    #[test]
    fn a_one_shot_repair_is_not_backgrounded_so_its_failure_is_reported() {
        // The path a stranded user depends on. Backgrounded, `osascript` would
        // return before `pmset` had run and the window would report success
        // whatever happened — on the one screen whose whole job is to tell them
        // the truth about whether their Mac can sleep.
        let applescript = privileged_now("pmset -a disablesleep 0");
        assert!(
            applescript.contains("with administrator privileges"),
            "got: {applescript}"
        );
        assert!(
            !applescript.contains('&'),
            "a repair must be waited for: {applescript}"
        );
    }

    #[test]
    fn a_laptop_on_battery_reports_its_charge_and_that_it_is_unplugged() {
        // Real output from `pmset -g batt`, captured on the target machine.
        let raw = concat!(
            "Now drawing from 'Battery Power'\n",
            " -InternalBattery-0 (id=21823587)\t89%; discharging; 15:53 remaining present: true\n",
        );
        assert_eq!(
            parse_batt(raw),
            Power {
                percent: Some(89),
                external: false
            }
        );
    }

    #[test]
    fn a_laptop_on_the_charger_reports_external_power() {
        let raw = concat!(
            "Now drawing from 'AC Power'\n",
            " -InternalBattery-0 (id=21823587)\t89%; charging; 0:45 remaining present: true\n",
        );
        assert_eq!(
            parse_batt(raw),
            Power {
                percent: Some(89),
                external: true
            }
        );
    }

    #[test]
    fn a_desktop_reports_no_charge_rather_than_zero() {
        // A Mac with no battery prints the source line and nothing else. Zero
        // here would put every desktop permanently under the battery guard.
        assert_eq!(
            parse_batt("Now drawing from 'AC Power'\n"),
            Power {
                percent: None,
                external: true
            }
        );
    }

    #[test]
    fn unreadable_output_reports_nothing_rather_than_guessing() {
        assert_eq!(
            parse_batt(""),
            Power {
                percent: None,
                external: false
            }
        );
    }

    #[test]
    fn a_percentage_is_read_from_the_battery_line_not_from_anywhere_else() {
        // The remaining-time field also carries digits, and a future macOS may
        // add a line that does. Only the `-InternalBattery` line is trusted.
        let raw = concat!(
            "Now drawing from 'Battery Power'\n",
            " -InternalBattery-0 (id=1)\t7%; discharging; 0:12 remaining present: true\n",
        );
        assert_eq!(parse_batt(raw).percent, Some(7));
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

    #[test]
    fn macos_reports_it_can_schedule_a_wake() {
        assert!(MacOs.can_schedule_wake());
    }

    #[test]
    fn a_wake_batch_cancels_then_arms_in_one_command() {
        // The whole point of batching: cancels first, arms after, joined with
        // `; ` so a single elevation runs the lot and the password is asked once.
        let cancel = vec!["01/07/26 17:30:00".to_string()];
        let schedule = vec![
            "01/12/26 09:00:00".to_string(),
            "01/14/26 17:30:00".to_string(),
        ];
        let command = pmset_batch_command(&cancel, &schedule).unwrap();
        assert_eq!(
            command,
            "pmset schedule cancel wake \"01/07/26 17:30:00\"; \
             pmset schedule wake \"01/12/26 09:00:00\"; \
             pmset schedule wake \"01/14/26 17:30:00\""
        );
        // The cancel must come before either arm, so a re-armed datetime is not
        // cancelled straight back off.
        let cancel_at = command.find("cancel").unwrap();
        let first_arm = command.find("wake \"01/12").unwrap();
        assert!(
            cancel_at < first_arm,
            "cancels must precede arms: {command}"
        );
    }

    #[test]
    fn a_wake_batch_with_nothing_to_do_is_no_command_at_all() {
        // Both slices empty means no privileged step and no prompt.
        assert!(pmset_batch_command(&[], &[]).is_none());
    }

    #[test]
    fn an_installed_app_icon_becomes_a_real_png_data_uri() {
        // The one thing no test that cannot see the picker can otherwise prove:
        // that the AppKit pipeline (iconForFile → offscreen draw → PNG →
        // base64) actually produces decodable image bytes. A mistake anywhere in
        // it would compile, pass every pure test, and silently ship blank icons.
        //
        // Uses whatever real `.app` this machine has, preferring the two that
        // ship on every Mac and falling back to the first the scan finds, so it
        // runs on a developer laptop and on bare CI alike.
        let candidates = [
            "/System/Applications/Utilities/Terminal.app",
            "/System/Applications/System Settings.app",
            "/System/Library/CoreServices/Finder.app",
        ];
        let scanned = crate::schedule::scan_applications(&crate::schedule::application_dirs());
        let path = candidates
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .map(|p| p.to_string())
            .or_else(|| scanned.first().map(|app| app.path.clone()));

        let Some(path) = path else {
            // No application on this machine at all — nothing to render. A bare
            // CI runner is allowed to have none rather than fail here.
            return;
        };

        let uri = MacOs
            .app_icon(&path)
            .unwrap_or_else(|| panic!("no icon produced for {path}"));
        let prefix = "data:image/png;base64,";
        assert!(
            uri.starts_with(prefix),
            "icon must be a PNG data URI, got: {}",
            &uri[..uri.len().min(64)]
        );

        // Decode the payload inline and check the PNG magic bytes, so this pins a
        // real image rather than just a long-enough string.
        let payload = decode_base64(&uri[prefix.len()..]);
        assert!(
            payload.starts_with(&[0x89, b'P', b'N', b'G']),
            "the decoded icon must begin with the PNG signature, got {:?}",
            &payload[..payload.len().min(8)]
        );
    }

    #[test]
    fn every_installed_app_renders_an_icon_off_the_main_thread() {
        // `list_applications` calls `app_icon` once per scanned app, synchronously,
        // from the Tauri command worker — never the main thread. `cargo test`
        // gives every test its own non-main worker thread, which is the same
        // shape, so running the *whole* real `/Applications` scan here (not just
        // one app, as the test above does) is the empirical answer to "does
        // lockFocus/drawInRect/TIFFRepresentation hold up off the main thread at
        // the size a real machine's Applications folder gets to": every machine
        // that runs this test IS that machine.
        //
        // `--nocapture` prints how many apps and how long the batch took, which
        // is the other open question — whether the per-app AppKit round trip is
        // fast enough to do synchronously before the tab draws.
        let scanned = crate::schedule::scan_applications(&crate::schedule::application_dirs());
        if scanned.is_empty() {
            return; // A bare CI runner is allowed to have nothing installed.
        }

        let start = std::time::Instant::now();
        for app in &scanned {
            let Some(uri) = MacOs.app_icon(&app.path) else {
                // Some system bundles (frameworks masquerading as .app, or ones
                // missing an icon resource) legitimately produce none — that is
                // `app_icon`'s own `Option`, not a crash, and not this test's
                // concern. What matters is that rendering N of them in a row
                // never panics and never corrupts a later one.
                continue;
            };
            let payload = decode_base64(&uri["data:image/png;base64,".len()..]);
            assert!(
                payload.starts_with(&[0x89, b'P', b'N', b'G']),
                "{} produced a non-PNG payload",
                app.name
            );
        }
        eprintln!(
            "rendered {} icons in {:?} off the main thread",
            scanned.len(),
            start.elapsed()
        );
    }

    /// A minimal standard-alphabet base64 decoder, local to this test so the
    /// PNG-magic assertion above decodes what `schedule::base64_encode` wrote
    /// without either side borrowing the other's implementation.
    #[cfg(target_os = "macos")]
    fn decode_base64(s: &str) -> Vec<u8> {
        fn val(c: u8) -> Option<u32> {
            match c {
                b'A'..=b'Z' => Some((c - b'A') as u32),
                b'a'..=b'z' => Some((c - b'a' + 26) as u32),
                b'0'..=b'9' => Some((c - b'0' + 52) as u32),
                b'+' => Some(62),
                b'/' => Some(63),
                _ => None,
            }
        }
        let mut out = Vec::new();
        let mut acc = 0u32;
        let mut bits = 0u32;
        for &c in s.as_bytes() {
            let Some(v) = val(c) else { continue }; // skip '=' padding
            acc = (acc << 6) | v;
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((acc >> bits) as u8);
            }
        }
        out
    }
}
