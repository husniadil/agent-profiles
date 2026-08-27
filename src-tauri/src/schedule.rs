//! A per-weekday wake-and-launch: the pure model, the `pmset`/launchd
//! generators, and the persisted settings. Nothing here touches the system —
//! the macOS platform backend executes the [`WakePlan`] this produces, so every
//! rule below is unit-testable on any OS.
//!
//! The launch target is an ordinary macOS application (Slack, a browser, …),
//! opened at the scheduled time with `/usr/bin/open`. This deliberately does not
//! go through the app's own profile-launch path — it is a general "wake the Mac
//! and open this app" schedule, not an agent-profile launcher.
//!
//! Why one-off wakes rather than a single repeat: `pmset repeat` holds exactly
//! one wake time, so a distinct time per weekday cannot be expressed as a repeat.
//! Instead each upcoming occurrence in the next [`WAKE_HORIZON_DAYS`] is armed as
//! its own `pmset schedule wakeorpoweron` event, and [`coverage_is_low`] decides
//! when the buffer has run low enough to re-arm — which is the only thing that
//! costs the administrator password, and then only every several weeks.

use anyhow::Result;
use chrono::{DateTime, Datelike, Duration, Local, Weekday};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// How far ahead one re-arm schedules wakes: eight weeks. Long enough that the
/// password prompt is rare, short enough that a machine left off for a while does
/// not accumulate a huge backlog of stale one-off events.
pub const WAKE_HORIZON_DAYS: i64 = 56;

/// Re-arm once the furthest-out installed wake is within two weeks. The buffer
/// between this and [`WAKE_HORIZON_DAYS`] is what keeps re-arms — and their
/// password prompt — down to roughly one every six weeks.
pub const REARM_BELOW_DAYS: i64 = 14;

/// launchd's own weekday numbering (Sunday = 0, Monday = 1 … Saturday = 6) for a
/// Monday-first index (0 = Monday … 6 = Sunday), or `None` for an out-of-range
/// index. `StartCalendarInterval` speaks these integers, so this is the one place
/// the two numbering schemes are reconciled.
fn launchd_weekday(weekday: u8) -> Option<u8> {
    (weekday <= 6).then(|| (weekday + 1) % 7)
}

/// The chrono [`Weekday`] for a Monday-first index (0 = Monday … 6 = Sunday), or
/// `None` for an out-of-range index.
fn chrono_weekday(weekday: u8) -> Option<Weekday> {
    match weekday {
        0 => Some(Weekday::Mon),
        1 => Some(Weekday::Tue),
        2 => Some(Weekday::Wed),
        3 => Some(Weekday::Thu),
        4 => Some(Weekday::Fri),
        5 => Some(Weekday::Sat),
        6 => Some(Weekday::Sun),
        _ => None,
    }
}

/// One weekday's wake time, persisted verbatim inside [`Settings::days`].
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
pub struct DayWake {
    /// Monday-first: 0 = Monday … 6 = Sunday.
    pub weekday: u8,
    /// Local wall-clock time, `"HH:MM"`.
    pub time: String,
}

/// The schedule, persisted verbatim to `schedule.json`.
///
/// One entry per weekday the user set, each with its own time, so waking at a
/// different time on different days is expressible where a single `pmset repeat`
/// could not.
// Plain snake_case field names on the wire, matching `keep_awake::Settings` (not
// `general::Settings`, which is camelCase): the TypeScript `ScheduleSettings`
// mirror in `src/lib/api.ts` uses `app_path`/`days`/`weekday`/`time`, and a
// `camelCase` rename here would silently drop them on every JS↔Rust round-trip —
// the schedule would wake the Mac and launch nothing.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug, Default)]
pub struct Settings {
    /// Whether the wake and launch are installed. Off is the default: this
    /// touches a system power schedule and asks for an administrator password.
    #[serde(default)]
    pub enabled: bool,
    /// The days the user set, each with its own time. Only the days chosen appear
    /// here; an empty list means nothing is armed even when enabled.
    #[serde(default)]
    pub days: Vec<DayWake>,
    /// The application bundle to open at the scheduled time, as an absolute
    /// `.app` path (e.g. `/Applications/Slack.app`). Empty means nothing chosen
    /// yet, which leaves the schedule unarmed even when enabled.
    #[serde(default)]
    pub app_path: String,
}

impl Settings {
    /// Falls back to the defaults for anything unreadable, like
    /// [`crate::general::Settings::load`]: the default is disabled and holds
    /// nothing, so a lost preference never costs the user a stuck power schedule.
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

/// One installed application the schedule can open, as the picker shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct InstalledApp {
    /// The bundle name without `.app`, e.g. `Slack`.
    pub name: String,
    /// The absolute `.app` path, stored in [`Settings::app_path`] and handed to
    /// `open`.
    pub path: String,
    /// The app's icon as a `data:image/png;base64,…` URI, or `None` where one
    /// could not be produced (any non-macOS build, or an icon that failed to
    /// render). The scan itself always leaves this `None` — it stays pure and
    /// cross-platform — and the platform layer fills it in from
    /// [`crate::platform::Platform::app_icon`].
    #[serde(default)]
    pub icon: Option<String>,
}

/// The directories a macOS user's applications live in, in priority order. Off
/// macOS these simply do not exist, so the scan comes back empty — which is fine,
/// the tab is unsupported there anyway.
pub fn application_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs.push(PathBuf::from("/System/Applications"));
    dirs
}

/// Every `.app` bundle directly inside `dirs`, de-duplicated by name (the first
/// directory to carry a name wins) and sorted case-insensitively for the picker.
pub fn scan_applications(dirs: &[PathBuf]) -> Vec<InstalledApp> {
    let mut apps = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("app") {
                continue;
            }
            let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            if seen.insert(name.to_string()) {
                apps.push(InstalledApp {
                    name: name.to_string(),
                    path: path.display().to_string(),
                    // The scan is cross-platform and pure; icons are attached by
                    // the platform layer in `list_applications`.
                    icon: None,
                });
            }
        }
    }
    apps.sort_by_key(|app| app.name.to_lowercase());
    apps
}

/// Standard-alphabet base64, hand-rolled so a PNG icon can become a `data:` URI
/// without pulling in a dependency for the one place this app needs it.
///
/// Every three input bytes become four output characters; a final group of one
/// or two bytes is padded with `=` in the usual way, so the output length is
/// always a multiple of four. No line wrapping — a `data:` URI is a single run.
///
/// Only the macOS backend renders an icon to encode, so elsewhere this is unused
/// by design rather than by mistake — hence the narrowed allow. It is still
/// compiled and still tested on every platform, which is the point: the encoder
/// is the half a mistake would hide in.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub(crate) fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(ALPHABET[((n >> 18) & 0x3f) as usize] as char);
        out.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
        // The last two characters are real only when the input group carried the
        // bytes that feed them; otherwise they are padding.
        out.push(if chunk.len() > 1 {
            ALPHABET[((n >> 6) & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            ALPHABET[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// What the window needs to render the tab in one call.
#[derive(Serialize, Clone)]
pub struct Status {
    /// Whether this machine can schedule a wake at all. False off macOS, and the
    /// window draws the disabled band with [`Status::refusal`] filled in.
    pub supported: bool,
    /// Why the feature cannot be offered here, if it cannot.
    pub refusal: Option<String>,
    pub settings: Settings,
}

/// Everything the macOS backend needs to install the schedule, built by
/// [`build_wake_plan`] from pure inputs so the backend stays a thin executor.
pub struct WakePlan {
    /// Every upcoming wake to arm, formatted for `pmset schedule` as
    /// `"MM/dd/yy HH:mm:ss"`. Each carries no user-supplied path — they are our
    /// own formatted datetimes — so they need no shell quoting beyond the double
    /// quotes the batch wraps them in.
    pub wake_datetimes: Vec<String>,
    /// The LaunchAgent this plan installs. Only the macOS backend writes one —
    /// everywhere else `refresh_launch_agent` is the defaulted refusal and never
    /// reaches these, which is correct rather than an oversight, so the allow is
    /// narrowed to the platforms where going unread is the honest outcome.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub plist_path: PathBuf,
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub plist_xml: String,
}

fn parse_hhmm(s: &str) -> Option<(u8, u8)> {
    let (h, m) = s.split_once(':')?;
    let h: u8 = h.parse().ok()?;
    let m: u8 = m.parse().ok()?;
    (h < 24 && m < 60).then_some((h, m))
}

/// Every wake datetime in `(now, now + horizon]` for the given per-day times,
/// sorted ascending and de-duplicated. `weekday` is Monday-first 0..=6.
///
/// Each occurrence is materialised as a real local datetime rather than left as a
/// weekday-and-time, because `pmset schedule` takes absolute datetimes: the
/// whole point of one-off events is that the exact calendar days are named. Times
/// that do not parse, and weekday indices out of range, are skipped — a bad entry
/// costs its own wakes, never the whole schedule.
pub fn upcoming_wakes(
    days: &[DayWake],
    now: DateTime<Local>,
    horizon_days: i64,
) -> Vec<DateTime<Local>> {
    let end = now + Duration::days(horizon_days);
    let start_date = now.date_naive();
    let end_date = end.date_naive();

    let mut out = Vec::new();
    for day in days {
        let (Some(target), Some((h, m))) = (chrono_weekday(day.weekday), parse_hhmm(&day.time))
        else {
            continue;
        };
        let mut date = start_date;
        while date <= end_date {
            if date.weekday() == target {
                if let Some(naive) = date.and_hms_opt(u32::from(h), u32::from(m), 0) {
                    // A spring-forward gap makes the wall-clock time nonexistent
                    // (`.earliest()` is `None`); a fall-back overlap makes it
                    // ambiguous, and the earlier of the two instants is the right
                    // one to wake at. Either way this is safe — the wake is a
                    // little early at worst, and launchd fires on wall-clock time.
                    if let Some(dt) = naive.and_local_timezone(Local).earliest() {
                        if dt > now && dt <= end {
                            out.push(dt);
                        }
                    }
                }
            }
            let Some(next) = date.succ_opt() else { break };
            date = next;
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Formats a local datetime the way `pmset schedule` wants it: `"MM/dd/yy
/// HH:mm:ss"`.
pub fn pmset_datetime(dt: DateTime<Local>) -> String {
    dt.format("%m/%d/%y %H:%M:%S").to_string()
}

/// Parses a `pmset` datetime string (`"MM/dd/yy HH:mm:ss"`) back into a local
/// datetime, or `None` if it does not parse — used only to read our own
/// breadcrumb, so a malformed line is simply dropped from coverage.
fn parse_pmset_datetime(s: &str) -> Option<DateTime<Local>> {
    let naive = chrono::NaiveDateTime::parse_from_str(s.trim(), "%m/%d/%y %H:%M:%S").ok()?;
    naive.and_local_timezone(Local).earliest()
}

fn xml_escape(raw: &str) -> String {
    raw.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// One `StartCalendarInterval` dict per selected weekday, each at that day's own
/// time. launchd's weekday numbering (Sunday = 0), not a Monday-first index.
///
/// The launch is `/usr/bin/open <app_path>`, which hands the request to
/// LaunchServices exactly as a double-click would — so a login session that is
/// already up gets the app in the foreground, and nothing here has to know how
/// any particular app starts.
fn plist_xml(label: &str, app_path: &str, days: &[DayWake]) -> String {
    let mut intervals = String::new();
    for day in days {
        let (Some(launchd), Some((h, m))) = (launchd_weekday(day.weekday), parse_hhmm(&day.time))
        else {
            continue;
        };
        intervals.push_str(&format!(
            "    <dict><key>Weekday</key><integer>{launchd}</integer>\
             <key>Hour</key><integer>{h}</integer>\
             <key>Minute</key><integer>{m}</integer></dict>\n"
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \x20 <key>Label</key>\n  <string>{label}</string>\n\
         \x20 <key>ProgramArguments</key>\n  <array>\n\
         \x20   <string>/usr/bin/open</string>\n\
         \x20   <string>{app}</string>\n\
         \x20 </array>\n\
         \x20 <key>StartCalendarInterval</key>\n  <array>\n{intervals}  </array>\n\
         </dict>\n</plist>\n",
        label = xml_escape(label),
        app = xml_escape(app_path),
    )
}

/// Assembles the OS work from pure inputs, or `None` when there is nothing to
/// install (disabled, no app chosen, or no days chosen).
pub fn build_wake_plan(
    settings: &Settings,
    now: DateTime<Local>,
    home: &Path,
    bundle_id: &str,
    horizon_days: i64,
) -> Option<WakePlan> {
    if !settings.enabled || settings.days.is_empty() || settings.app_path.is_empty() {
        return None;
    }
    let wake_datetimes = upcoming_wakes(&settings.days, now, horizon_days)
        .iter()
        .map(|dt| pmset_datetime(*dt))
        .collect();
    let label = format!("{bundle_id}.schedule");
    Some(WakePlan {
        wake_datetimes,
        plist_path: crate::paths::launch_agent_plist(home, bundle_id),
        plist_xml: plist_xml(&label, &settings.app_path, &settings.days),
    })
}

/// Whether the installed wakes have run low enough to re-arm.
///
/// Given the wake datetimes we last installed and `now`, finds the furthest-out
/// one still in the future and returns true when none remain, or the latest is
/// within `min_days` of now. This is what keeps the password prompt rare: a
/// re-arm schedules [`WAKE_HORIZON_DAYS`] of wakes, and this only fires again once
/// that buffer has drained to `min_days`.
pub fn coverage_is_low(installed: &[String], now: DateTime<Local>, min_days: i64) -> bool {
    let threshold = now + Duration::days(min_days);
    match installed
        .iter()
        .filter_map(|s| parse_pmset_datetime(s))
        .filter(|dt| *dt > now)
        .max()
    {
        Some(latest) => latest <= threshold,
        None => true,
    }
}

/// The live settings, mirroring [`crate::general::Handle`]: the file is the
/// record and the mutex is the copy the commands read.
pub struct Handle {
    data_root: PathBuf,
    settings: Mutex<Settings>,
}

impl Handle {
    pub fn new(data_root: PathBuf) -> Self {
        let settings = Settings::load(&crate::paths::schedule_settings(&data_root));
        Self {
            data_root,
            settings: Mutex::new(settings),
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    pub fn set_settings(&self, next: Settings) -> Result<()> {
        next.save(&crate::paths::schedule_settings(&self.data_root))?;
        if let Ok(mut held) = self.settings.lock() {
            *held = next;
        }
        Ok(())
    }

    /// The one-off wake datetimes currently installed, one per line, in the
    /// `pmset` format. Empty file or no file means nothing is installed.
    pub fn installed_wakes(&self) -> Vec<String> {
        std::fs::read_to_string(crate::paths::schedule_applied(&self.data_root))
            .map(|raw| {
                raw.lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn set_installed_wakes(&self, dts: &[String]) -> Result<()> {
        std::fs::write(
            crate::paths::schedule_applied(&self.data_root),
            dts.join("\n"),
        )?;
        Ok(())
    }

    pub fn clear_installed_wakes(&self) {
        let _ = std::fs::remove_file(crate::paths::schedule_applied(&self.data_root));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::path::PathBuf;

    /// A fixed local datetime, so the occurrence math is asserted against exact
    /// expected values rather than against the clock.
    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    #[test]
    fn upcoming_wakes_lists_each_days_occurrences_in_the_horizon() {
        // now is Monday 2026-01-05 12:00. Monday@09:00 and Wednesday@17:30 over a
        // 14-day horizon (ending Monday 2026-01-19 12:00).
        let now = at(2026, 1, 5, 12, 0);
        let days = vec![
            DayWake {
                weekday: 0, // Monday
                time: "09:00".into(),
            },
            DayWake {
                weekday: 2, // Wednesday
                time: "17:30".into(),
            },
        ];
        let got = upcoming_wakes(&days, now, 14);
        assert_eq!(
            got,
            vec![
                // This Monday 09:00 is already past (now is 12:00), so it is not
                // listed; the two following Mondays are, including the one landing
                // exactly on the horizon.
                at(2026, 1, 7, 17, 30),  // Wed
                at(2026, 1, 12, 9, 0),   // Mon
                at(2026, 1, 14, 17, 30), // Wed
                at(2026, 1, 19, 9, 0),   // Mon, exactly on the horizon end
            ]
        );
    }

    #[test]
    fn upcoming_wakes_with_no_days_is_empty() {
        let now = at(2026, 1, 5, 12, 0);
        assert!(upcoming_wakes(&[], now, 14).is_empty());
    }

    #[test]
    fn upcoming_wakes_skips_a_malformed_time_but_keeps_the_rest() {
        let now = at(2026, 1, 5, 12, 0);
        let days = vec![
            DayWake {
                weekday: 1, // Tuesday
                time: "not-a-time".into(),
            },
            DayWake {
                weekday: 1, // Tuesday, valid
                time: "08:00".into(),
            },
        ];
        // Only the valid entry contributes; the first Tuesday after now is Jan 6.
        assert_eq!(upcoming_wakes(&days, now, 7), vec![at(2026, 1, 6, 8, 0)]);
    }

    #[test]
    fn pmset_datetime_formats_month_day_two_digit_year_and_seconds() {
        assert_eq!(pmset_datetime(at(2026, 1, 7, 17, 30)), "01/07/26 17:30:00");
        assert_eq!(pmset_datetime(at(2026, 12, 31, 9, 5)), "12/31/26 09:05:00");
    }

    #[test]
    fn coverage_is_low_only_when_the_buffer_has_actually_drained() {
        let now = at(2026, 1, 5, 12, 0);

        // Full coverage: the furthest wake is 40 days out, well past the 14-day
        // floor — no re-arm.
        let full = vec![pmset_datetime(at(2026, 2, 14, 9, 0))];
        assert!(!coverage_is_low(&full, now, 14));

        // Nearly expired: the furthest wake is only 5 days out — re-arm.
        let nearly = vec![pmset_datetime(at(2026, 1, 10, 9, 0))];
        assert!(coverage_is_low(&nearly, now, 14));

        // Nothing installed — re-arm.
        assert!(coverage_is_low(&[], now, 14));

        // Only past wakes remain (none still in the future) — re-arm.
        let past = vec![pmset_datetime(at(2026, 1, 3, 9, 0))];
        assert!(coverage_is_low(&past, now, 14));
    }

    #[test]
    fn the_plist_names_each_selected_day_at_its_own_time() {
        // launchd fires at the exact time, one dict per selected day using
        // launchd's own weekday numbering (Sun = 0).
        let days = vec![
            DayWake {
                weekday: 0, // Monday -> launchd 1
                time: "09:00".into(),
            },
            DayWake {
                weekday: 6, // Sunday -> launchd 0
                time: "22:15".into(),
            },
        ];
        let xml = plist_xml("com.example.app.schedule", "/Applications/Slack.app", &days);
        // Monday at 09:00.
        assert!(xml.contains(
            "<key>Weekday</key><integer>1</integer><key>Hour</key><integer>9</integer><key>Minute</key><integer>0</integer>"
        ));
        // Sunday at 22:15.
        assert!(xml.contains(
            "<key>Weekday</key><integer>0</integer><key>Hour</key><integer>22</integer><key>Minute</key><integer>15</integer>"
        ));
        // The launch is `/usr/bin/open <app>`, not the agent-profiles binary.
        assert!(xml.contains("<string>/usr/bin/open</string>"));
        assert!(xml.contains("<string>/Applications/Slack.app</string>"));
        assert!(!xml.contains("--launch-profile"));
    }

    #[test]
    fn the_plist_escapes_xml_metacharacters_in_the_path() {
        // A username with an ampersand is legal on macOS and would otherwise break
        // the plist.
        let days = vec![DayWake {
            weekday: 0,
            time: "08:30".into(),
        }];
        let xml = plist_xml(
            "com.example.app.schedule",
            "/Users/a&b/Applications/My App.app",
            &days,
        );
        assert!(xml.contains("/Users/a&amp;b/Applications/My App.app"));
        assert!(!xml.contains("/Users/a&b/"));
    }

    #[test]
    fn a_wake_plan_is_only_built_when_the_schedule_is_armed() {
        let now = at(2026, 1, 5, 12, 0);
        let home = PathBuf::from("/Users/h");
        let bundle = "com.example.app";

        let armed = Settings {
            enabled: true,
            days: vec![DayWake {
                weekday: 0,
                time: "09:00".into(),
            }],
            app_path: "/Applications/Slack.app".into(),
        };
        let plan = build_wake_plan(&armed, now, &home, bundle, WAKE_HORIZON_DAYS).unwrap();
        assert_eq!(
            plan.plist_path,
            PathBuf::from("/Users/h/Library/LaunchAgents/com.example.app.schedule.plist")
        );
        assert!(plan.plist_xml.contains("/Applications/Slack.app"));
        // Eight weeks of Mondays gives eight one-off wakes, each a real datetime.
        assert!(!plan.wake_datetimes.is_empty());
        assert!(plan
            .wake_datetimes
            .iter()
            .all(|dt| dt.contains(" 09:00:00")));

        // Disabled, no app chosen, or no days chosen — nothing to install.
        assert!(build_wake_plan(
            &Settings {
                enabled: false,
                ..armed.clone()
            },
            now,
            &home,
            bundle,
            WAKE_HORIZON_DAYS
        )
        .is_none());
        assert!(build_wake_plan(
            &Settings {
                app_path: String::new(),
                ..armed.clone()
            },
            now,
            &home,
            bundle,
            WAKE_HORIZON_DAYS
        )
        .is_none());
        assert!(build_wake_plan(
            &Settings {
                days: Vec::new(),
                ..armed.clone()
            },
            now,
            &home,
            bundle,
            WAKE_HORIZON_DAYS
        )
        .is_none());
    }

    #[test]
    fn settings_survive_a_round_trip_and_old_files_take_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("schedule.json");
        let written = Settings {
            enabled: true,
            days: vec![
                DayWake {
                    weekday: 0,
                    time: "07:30".into(),
                },
                DayWake {
                    weekday: 4,
                    time: "10:00".into(),
                },
            ],
            app_path: "/Applications/Slack.app".into(),
        };
        written.save(&file).unwrap();
        assert_eq!(Settings::load(&file), written);

        // A missing file, and a partial one written before `days` existed, both
        // land on the defaults for what they do not carry.
        assert_eq!(
            Settings::load(&dir.path().join("nope.json")),
            Settings::default()
        );
        let partial = dir.path().join("partial.json");
        std::fs::write(&partial, br#"{"enabled":true}"#).unwrap();
        let loaded = Settings::load(&partial);
        assert!(loaded.enabled);
        assert!(
            loaded.days.is_empty(),
            "an absent day list defaults to nothing armed"
        );
        assert!(loaded.app_path.is_empty());
    }

    #[test]
    fn the_wire_field_names_are_the_snake_case_ones_the_frontend_sends() {
        // The TypeScript `ScheduleSettings` in `src/lib/api.ts` uses `days`,
        // `weekday`, `time` and `app_path`. A `#[serde(rename_all = "camelCase")]`
        // anywhere here would emit `appPath` and drop it on every JS↔Rust
        // round-trip, so the schedule would wake the Mac but launch nothing.
        let json = serde_json::to_string(&Settings {
            enabled: true,
            days: vec![DayWake {
                weekday: 1,
                time: "09:00".into(),
            }],
            app_path: "/Applications/Slack.app".into(),
        })
        .unwrap();
        assert!(json.contains("\"days\""), "got: {json}");
        assert!(json.contains("\"weekday\""), "got: {json}");
        assert!(json.contains("\"time\""), "got: {json}");
        assert!(json.contains("\"app_path\""), "got: {json}");
        assert!(!json.contains("appPath"), "must not camelCase: {json}");
    }

    #[test]
    fn the_app_scan_finds_dot_app_bundles_dedupes_and_sorts_them() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("apps-a");
        let b = dir.path().join("apps-b");
        std::fs::create_dir_all(a.join("Slack.app")).unwrap();
        std::fs::create_dir_all(a.join("zed.app")).unwrap();
        std::fs::create_dir_all(a.join("notes.txt")).unwrap(); // not an app
                                                               // A second directory carrying a name already seen must not double it.
        std::fs::create_dir_all(b.join("Slack.app")).unwrap();
        std::fs::create_dir_all(b.join("Arc.app")).unwrap();

        let apps = scan_applications(&[a.clone(), b]);
        let names: Vec<&str> = apps.iter().map(|app| app.name.as_str()).collect();
        // Case-insensitive sort, "notes.txt" excluded, "Slack" not duplicated.
        assert_eq!(names, vec!["Arc", "Slack", "zed"]);
        // The path is the bundle from the first directory that carried the name.
        let slack = apps.iter().find(|app| app.name == "Slack").unwrap();
        assert_eq!(slack.path, a.join("Slack.app").display().to_string());
    }

    #[test]
    fn base64_matches_the_rfc_test_vectors() {
        // The canonical RFC 4648 §10 vectors, which exercise every padding case:
        // no padding, one `=`, and two `=`.
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // A byte with the high bit set must still map into the alphabet, not
        // panic or sign-extend: 0xFF 0xFF 0xFF is all ones, i.e. `////`.
        assert_eq!(base64_encode(&[0xff, 0xff, 0xff]), "////");
    }

    #[test]
    fn a_handle_reads_the_installed_wakes_it_was_given() {
        let dir = tempfile::tempdir().unwrap();
        let handle = Handle::new(dir.path().to_path_buf());
        assert!(handle.installed_wakes().is_empty());

        let wakes = vec![
            "01/07/26 17:30:00".to_string(),
            "01/12/26 09:00:00".to_string(),
        ];
        handle.set_installed_wakes(&wakes).unwrap();
        assert_eq!(handle.installed_wakes(), wakes);

        // On disk, not only in memory: a second handle reads the same list.
        let reopened = Handle::new(dir.path().to_path_buf());
        assert_eq!(reopened.installed_wakes(), wakes);

        handle.clear_installed_wakes();
        assert!(handle.installed_wakes().is_empty());
    }

    #[test]
    fn a_handle_reads_what_was_written_through_it() {
        let dir = tempfile::tempdir().unwrap();
        let handle = Handle::new(dir.path().to_path_buf());
        handle
            .set_settings(Settings {
                enabled: true,
                days: vec![DayWake {
                    weekday: 0,
                    time: "10:00".into(),
                }],
                app_path: "/Applications/Slack.app".into(),
            })
            .unwrap();
        assert!(handle.settings().enabled);
        // On disk, not only in the mutex: a second handle is the next launch.
        let reopened = Handle::new(dir.path().to_path_buf());
        assert_eq!(reopened.settings().days[0].time, "10:00");
    }
}
