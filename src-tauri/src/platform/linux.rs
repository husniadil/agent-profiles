//! Compiled on every platform so its tests keep running, but the code that
//! calls these helpers is the Linux `Platform` impl, which exists only there.
#![cfg_attr(not(target_os = "linux"), allow(dead_code))]

use crate::platform::{Power, Thermal, DATA_DIR_SLUG};
use std::path::{Path, PathBuf};

pub fn desktop_file_path(applications_dir: &Path, wm_class: &str) -> PathBuf {
    applications_dir.join(format!("{wm_class}.desktop"))
}

pub fn desktop_entry(
    app_label: &str,
    profile_label: &str,
    exec: &str,
    icon: &str,
    wm_class: &str,
) -> String {
    format!(
        "[Desktop Entry]\n\
         Type=Application\n\
         Name={app_label} — {profile_label}\n\
         Comment={app_label}, {profile_label} profile\n\
         Exec={exec}\n\
         Icon={icon}\n\
         Terminal=false\n\
         Categories=Utility;\n\
         NoDisplay=true\n\
         StartupWMClass={wm_class}\n",
    )
}

pub fn is_wayland(session_type: Option<&str>) -> bool {
    matches!(session_type, Some(session) if session.eq_ignore_ascii_case("wayland"))
}

/// One entry under `/sys/class/power_supply`, as the kernel writes it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Supply {
    /// `type`: `Battery`, `Mains`, `USB`, `UPS`, …
    pub kind: String,
    /// `status`, on a battery: `Charging`, `Discharging`, `Full`, `Not charging`.
    pub status: String,
    /// `online`, on everything that is not a battery.
    pub online: bool,
    /// `capacity`, in percent, on a battery.
    pub capacity: Option<u8>,
}

/// Folds every supply the kernel lists into the one reading the guard wants.
///
/// External power is any non-battery supply reporting online, not `Mains`
/// alone: a laptop charging over USB-C is listed as `USB` or `USB_PD`, and
/// insisting on `Mains` would pause a machine that is plugged in and charging.
///
/// The battery's own `status` is the fallback for the machines — mostly ARM
/// laptops and tablets — that list a battery and no charger at all. Without it
/// those read as unplugged for as long as they are charging, which is the one
/// mistake the battery guard must not make.
pub fn fold_supplies(supplies: &[Supply]) -> Power {
    let battery = supplies.iter().find(|supply| supply.kind == "Battery");
    // `Not charging` is a battery held at a charge limit, which means the
    // charger is very much attached.
    let charging = battery.is_some_and(|battery| {
        matches!(
            battery.status.as_str(),
            "Charging" | "Full" | "Not charging"
        )
    });
    Power {
        // The first battery, not an average across two: a machine with more
        // than one is rare enough that picking one beats a figure that
        // describes neither.
        percent: battery.and_then(|battery| battery.capacity),
        external: supplies
            .iter()
            .any(|supply| supply.kind != "Battery" && supply.online)
            || charging,
    }
}

/// The hottest of the kernel's thermal zones, as one of the four levels.
///
/// macOS hands over the system's own judgement; Linux hands over numbers, so
/// these bands are ours. They sit against the trip points laptops actually
/// ship with: passive throttling starts in the eighties and the critical
/// shutdown trip is near 100 °C on both Intel and AMD parts.
///
/// Hottest rather than an average, and every zone rather than the CPU's: the
/// question is whether any part of the machine is too hot to make worse, and a
/// battery or chassis zone in the nineties is exactly the case a lid-closed
/// hold must not ignore.
///
/// ponytail: fixed bands. If a machine turns out to need its own, read each
/// zone's `trip_point_*_temp` and band against those instead of these numbers.
pub fn classify_zones(millicelsius: &[i64]) -> Thermal {
    // Below 1 °C or above 125 °C is not a machine, it is a sensor that is not
    // reading — both ends turn up on real hardware, from zones that report 0
    // when unpopulated to ones that report nonsense when a driver is missing.
    let hottest = millicelsius
        .iter()
        .copied()
        .filter(|temp| (1_000..=125_000).contains(temp))
        .max();
    match hottest {
        None => Thermal::Unknown,
        Some(temp) if temp >= 95_000 => Thermal::Critical,
        Some(temp) if temp >= 85_000 => Thermal::Serious,
        Some(temp) if temp >= 70_000 => Thermal::Fair,
        Some(_) => Thermal::Nominal,
    }
}

/// The command `systemd-inhibit` is given to run, and the entire reason the
/// lock is safe to take.
///
/// It must be something that ends when its stdin does. Rust never kills a child
/// on drop, and a child reparented to init outlives whatever spawned it, so a
/// `sleep infinity` here would go on holding the lid-switch lock after this app
/// died — a `kill -9`, a panic, a machine that lost power and came back — until
/// someone found the process with `ps`. That is the same permanent hold the
/// breadcrumb exists to prevent on macOS, reached by a different road.
///
/// A pipe cannot be leaked. However this process ends, the kernel closes the
/// write end, `cat` reads EOF, and the lock goes with it.
pub const INHIBIT_HOLDER: &str = "cat";

/// Takes the lid-switch and idle locks, as the user, for as long as this
/// process holds the other end of the child's stdin.
///
/// Deliberately not `--what=sleep`: that would also block a suspend the user
/// asked for from their own menu, which is not what "keep the machine awake
/// while an agent works" means. The lid and the idle timer are the two things
/// that put a machine to sleep without being told to, and they are exactly the
/// two this holds.
///
/// `--mode=block` rather than `delay`: a delay inhibitor buys a few seconds and
/// then the machine suspends anyway, which for a hold is no hold at all.
///
/// Built here rather than inside the Linux-only backend so that the one thing
/// no compiler on a developer's Mac would otherwise check — that this spawns a
/// command which dies with its pipe — is checked on every platform's test run.
pub fn inhibit_command() -> std::process::Command {
    let mut command = std::process::Command::new("systemd-inhibit");
    command
        .args([
            "--what=handle-lid-switch:idle",
            "--who=Agent Profiles",
            "--why=An agent session is working",
            "--mode=block",
            INHIBIT_HOLDER,
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    command
}

/// Drops the lock, by the same door a crash would use.
///
/// Closing the pipe first rather than killing outright, so the deliberate
/// release travels the path that has to work anyway — the one exercised on
/// every release is then the one a crash depends on. The kill is the backstop:
/// without it a child that somehow did not read its EOF would block the sweep
/// thread inside `wait` for the life of the app.
pub fn release(child: &mut std::process::Child) {
    drop(child.stdin.take());
    let _ = child.kill();
    // Reaped, not abandoned: an unwaited child is a zombie for the life of the
    // app, and this one is created and dropped every time an agent starts and
    // stops.
    let _ = child.wait();
}

pub fn data_root_from(xdg_config_home: Option<&str>, home: &str) -> PathBuf {
    match xdg_config_home {
        Some(path) if !path.is_empty() => PathBuf::from(path).join(DATA_DIR_SLUG),
        _ => PathBuf::from(home).join(".config").join(DATA_DIR_SLUG),
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use crate::app_spec::{AppSpec, Locations};
    use crate::platform::{unix_ps, FocusHint, FocusOutcome, Platform, RunningProcess, ScanTarget};
    use anyhow::{anyhow, Result};
    use std::process::Command;

    const ICON_NAME: &str = "com.husniadil.agent-profiles";

    /// The lid-closed hold, as one live child process.
    ///
    /// Nothing privileged and nothing persistent, which is the whole reason
    /// Linux needs neither the password prompt macOS asks for nor the crash
    /// recovery Windows needs: logind drops an inhibitor the moment the process
    /// holding it goes, so a kill -9 of this app releases the machine by
    /// itself. The `Mutex` is what lets `&self` own a child at all.
    pub struct Linux {
        inhibitor: std::sync::Mutex<Option<std::process::Child>>,
    }

    impl Linux {
        pub fn new() -> Self {
            Self {
                inhibitor: std::sync::Mutex::new(None),
            }
        }
    }

    impl Default for Linux {
        fn default() -> Self {
            Self::new()
        }
    }

    fn home() -> Result<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| anyhow!("HOME is not set"))
    }

    fn which(tool: &str) -> bool {
        Command::new("which")
            .arg(tool)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn applications_dir() -> Result<PathBuf> {
        Ok(home()?.join(".local").join("share").join("applications"))
    }

    fn here<'a>(
        locations: &'a Locations,
        product: &str,
    ) -> Result<&'a crate::app_spec::LinuxLocation> {
        locations
            .linux
            .as_ref()
            .ok_or_else(|| anyhow!("{product} has not been declared for Linux"))
    }

    impl Platform for Linux {
        fn declared_here(&self, locations: &Locations) -> bool {
            locations.linux.is_some()
        }

        fn data_root(&self) -> Result<PathBuf> {
            let home = home()?;
            let xdg_config_home = std::env::var_os("XDG_CONFIG_HOME");
            Ok(data_root_from(
                xdg_config_home.as_deref().and_then(|path| path.to_str()),
                &home.display().to_string(),
            ))
        }

        fn default_profile_dir(&self, locations: &Locations) -> Result<PathBuf> {
            Ok(home()?.join(here(locations, "this app")?.default_profile))
        }

        fn binary(&self, locations: &Locations, product: &str) -> Result<PathBuf> {
            let declared = here(locations, product)?;
            let command = declared.command;
            let hint = declared.install_hint;
            let output = Command::new("which").arg(command).output()?;
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !output.status.success() || path.is_empty() {
                return Err(anyhow!(
                    "{product} was not found on PATH as `{command}`. {hint}"
                ));
            }
            Ok(PathBuf::from(path))
        }

        fn process_marker(&self, locations: &Locations) -> Result<String> {
            Ok(here(locations, "this app")?.command.to_string())
        }

        fn scan(&self, targets: &[ScanTarget]) -> Result<Vec<RunningProcess>> {
            unix_ps::scan(targets)
        }

        fn link(&self, source: &Path, target: &Path) -> Result<()> {
            std::os::unix::fs::symlink(source, target)?;
            Ok(())
        }

        fn focus(&self, _pid: i32, hint: &FocusHint) -> Result<FocusOutcome> {
            if is_wayland(std::env::var("XDG_SESSION_TYPE").ok().as_deref()) {
                return Ok(FocusOutcome::Unsupported(
                    "Wayland does not let one app raise another's window — use this profile's \
                     own entry in your taskbar or alt-tab"
                        .into(),
                ));
            }
            if !which("xdotool") {
                return Ok(FocusOutcome::Unsupported(
                    "install xdotool to focus from here, or use this profile's taskbar entry"
                        .into(),
                ));
            }
            let status = Command::new("xdotool")
                .args(["search", "--class", hint.wm_class, "windowactivate"])
                .status()?;
            if status.success() {
                Ok(FocusOutcome::Focused)
            } else {
                Ok(FocusOutcome::Unsupported(format!(
                    "xdotool found no window with class {}",
                    hint.wm_class
                )))
            }
        }

        fn quit(&self, pid: i32) -> Result<()> {
            crate::platform::unix_signal_quit(pid)
        }

        fn os_launch_args(&self, wm_class: &str) -> Vec<String> {
            vec![format!("--class={wm_class}")]
        }

        fn register_identity(
            &self,
            spec: &AppSpec,
            profile_label: &str,
            wm_class: &str,
        ) -> Result<()> {
            let binary = self.binary(&spec.locations, spec.product)?;
            let directory = applications_dir()?;
            std::fs::create_dir_all(&directory)?;
            let exec = format!("{} --class={wm_class}", binary.display());
            std::fs::write(
                desktop_file_path(&directory, wm_class),
                desktop_entry(spec.label, profile_label, &exec, ICON_NAME, wm_class),
            )?;
            Ok(())
        }

        fn unregister_identity(&self, wm_class: &str) -> Result<()> {
            let path = desktop_file_path(&applications_dir()?, wm_class);
            match std::fs::remove_file(path) {
                Ok(()) => Ok(()),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            }
        }

        fn power(&self) -> Result<Power> {
            let supplies = read_supplies();
            if supplies.is_empty() {
                return Err(anyhow!("{POWER_SUPPLY_DIR} lists no power supply"));
            }
            Ok(fold_supplies(&supplies))
        }

        fn thermal(&self) -> Thermal {
            classify_zones(&read_zone_temps())
        }

        /// Asked per machine rather than answered for the platform: a Linux box
        /// without logind — a runit or OpenRC install, a container — has
        /// nothing to inhibit, and claiming otherwise would put a switch in the
        /// window that silently did nothing every time it was armed.
        fn can_hold_awake(&self) -> bool {
            which("systemd-inhibit")
        }

        fn hold(&self, _data_root: &Path, on: bool) -> Result<()> {
            let mut held = self
                .inhibitor
                .lock()
                .map_err(|_| anyhow!("the inhibitor lock was poisoned"))?;

            if !on {
                if let Some(mut child) = held.take() {
                    release(&mut child);
                }
                return Ok(());
            }

            // Already holding, and the lock is still alive. `try_wait` is the
            // whole check — if `systemd-inhibit` died on us the hold is gone
            // with it, and the machine would sleep while the window went on
            // saying it was held.
            if let Some(child) = held.as_mut() {
                match child.try_wait() {
                    Ok(None) => return Ok(()),
                    _ => {
                        let _ = child.wait();
                        *held = None;
                    }
                }
            }

            *held = Some(inhibit()?);
            Ok(())
        }
    }

    fn inhibit() -> Result<std::process::Child> {
        inhibit_command()
            .spawn()
            .map_err(|error| anyhow!("could not take a logind inhibitor lock: {error}"))
    }

    /// Every battery and charger the kernel knows about. Plain files, read once
    /// per sweep — no dependency, no daemon, and nothing that needs a session
    /// bus to be up, which matters because the sweep also has to work with the
    /// lid shut and the desktop gone quiet.
    const POWER_SUPPLY_DIR: &str = "/sys/class/power_supply";
    const THERMAL_DIR: &str = "/sys/class/thermal";

    /// A sysfs attribute, trimmed. Missing is `None` and not an error: which
    /// attributes exist varies by driver, and a battery with no `capacity` is a
    /// battery this guard cannot read rather than a failure to report.
    fn attribute(dir: &Path, name: &str) -> Option<String> {
        std::fs::read_to_string(dir.join(name))
            .ok()
            .map(|raw| raw.trim().to_string())
    }

    fn read_supplies() -> Vec<Supply> {
        let Ok(entries) = std::fs::read_dir(POWER_SUPPLY_DIR) else {
            return Vec::new();
        };
        entries
            .flatten()
            .map(|entry| {
                let dir = entry.path();
                Supply {
                    kind: attribute(&dir, "type").unwrap_or_default(),
                    status: attribute(&dir, "status").unwrap_or_default(),
                    online: attribute(&dir, "online").as_deref() == Some("1"),
                    capacity: attribute(&dir, "capacity").and_then(|percent| percent.parse().ok()),
                }
            })
            .collect()
    }

    fn read_zone_temps() -> Vec<i64> {
        let Ok(entries) = std::fs::read_dir(THERMAL_DIR) else {
            return Vec::new();
        };
        entries
            .flatten()
            .filter_map(|entry| attribute(&entry.path(), "temp"))
            .filter_map(|temp| temp.parse().ok())
            .collect()
    }
}

#[cfg(target_os = "linux")]
pub use imp::Linux;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::wm_class;

    #[test]
    fn profiles_that_would_slug_identically_still_get_distinct_identities() {
        let a = wm_class("claude", "id-one");
        let b = wm_class("claude", "id-two");
        assert_ne!(a, b);

        let dir = Path::new("/apps");
        assert_ne!(desktop_file_path(dir, &a), desktop_file_path(dir, &b));
    }

    #[test]
    fn the_same_profile_id_under_two_apps_gets_two_desktop_files() {
        let dir = Path::new("/apps");
        assert_ne!(
            desktop_file_path(dir, &wm_class("claude", "abc")),
            desktop_file_path(dir, &wm_class("codex", "abc"))
        );
    }

    #[test]
    fn the_desktop_entry_declares_a_matching_startup_wm_class() {
        let class = wm_class("claude", "a1b2");
        let entry = desktop_entry(
            "Claude",
            "Kerja",
            "/usr/bin/claude-desktop --class=x",
            "/i/icon.png",
            &class,
        );
        assert!(entry.contains("Name=Claude — Kerja"));
        assert!(entry.contains("StartupWMClass=agent-profiles-claude-a1b2"));
        assert!(entry.contains("NoDisplay=true"));
        assert!(entry.contains("Icon=/i/icon.png"));
        assert!(entry.starts_with("[Desktop Entry]"));
    }

    #[test]
    fn the_entry_is_named_after_whichever_app_it_belongs_to() {
        let entry = desktop_entry("ChatGPT", "Kerja", "/usr/bin/chatgpt", "icon", "c");
        assert!(entry.contains("Name=ChatGPT — Kerja"));
    }

    #[test]
    fn the_desktop_file_lands_in_the_applications_directory() {
        assert_eq!(
            desktop_file_path(
                Path::new("/home/h/.local/share/applications"),
                "agent-profiles-claude-a1b2"
            ),
            PathBuf::from("/home/h/.local/share/applications/agent-profiles-claude-a1b2.desktop")
        );
    }

    #[test]
    fn wayland_is_detected_from_the_session_type() {
        assert!(is_wayland(Some("wayland")));
        assert!(!is_wayland(Some("x11")));
        assert!(!is_wayland(None));
    }

    #[test]
    fn the_config_root_honours_xdg_config_home() {
        assert_eq!(
            data_root_from(Some("/xdg"), "/home/h"),
            PathBuf::from("/xdg/agent-profiles")
        );
        assert_eq!(
            data_root_from(None, "/home/h"),
            PathBuf::from("/home/h/.config/agent-profiles")
        );
    }

    fn battery(status: &str, capacity: u8) -> Supply {
        Supply {
            kind: "Battery".into(),
            status: status.into(),
            online: false,
            capacity: Some(capacity),
        }
    }

    fn charger(kind: &str, online: bool) -> Supply {
        Supply {
            kind: kind.into(),
            status: String::new(),
            online,
            capacity: None,
        }
    }

    #[test]
    fn a_laptop_on_battery_reports_its_charge_and_no_external_power() {
        let power = fold_supplies(&[charger("Mains", false), battery("Discharging", 42)]);
        assert_eq!(power.percent, Some(42));
        assert!(!power.external);
    }

    #[test]
    fn charging_over_usb_c_still_counts_as_external_power() {
        // The mistake this catches is matching on `Mains` alone: a machine on a
        // USB-C charger would read as unplugged and be paused at the floor
        // while its battery was going up.
        let power = fold_supplies(&[charger("USB_PD", true), battery("Charging", 25)]);
        assert!(power.external, "a live USB charger is external power");
    }

    #[test]
    fn a_battery_that_says_it_is_charging_covers_a_machine_with_no_charger_listed() {
        // ARM laptops and tablets often list a battery and nothing else at all.
        let power = fold_supplies(&[battery("Charging", 10)]);
        assert!(power.external);
        assert!(!fold_supplies(&[battery("Discharging", 10)]).external);
    }

    #[test]
    fn a_desktop_with_no_battery_reports_no_charge_rather_than_a_flat_one() {
        // The whole point of `Option`: zero here would pause a machine that
        // cannot run out of power for as long as it was switched on.
        let power = fold_supplies(&[charger("Mains", true)]);
        assert_eq!(power.percent, None);
        assert!(power.external);
    }

    #[test]
    fn the_hottest_zone_decides_the_level() {
        assert_eq!(classify_zones(&[45_000, 88_000, 51_000]), Thermal::Serious);
        assert_eq!(classify_zones(&[45_000, 51_000]), Thermal::Nominal);
        assert_eq!(classify_zones(&[72_000]), Thermal::Fair);
        assert_eq!(classify_zones(&[99_000]), Thermal::Critical);
    }

    #[test]
    fn only_serious_and_above_release_a_hold() {
        // The band boundaries matter more than the names: `Fair` is an ordinary
        // busy laptop, and releasing there would mean a hold that never
        // survives a build.
        assert!(!classify_zones(&[84_999]).is_danger());
        assert!(classify_zones(&[85_000]).is_danger());
    }

    /// How long the holder gets to notice its pipe closed before the test calls
    /// it stuck. Milliseconds is the honest scale — this is a read returning
    /// EOF — and the generous bound is here so a loaded CI box does not fail a
    /// working implementation.
    const EOF_DEADLINE: std::time::Duration = std::time::Duration::from_secs(5);

    #[test]
    fn the_command_holding_the_lock_ends_when_its_pipe_does() {
        // The regression test for the defect that mattered: with `sleep
        // infinity` here — which is what this shipped as — the child ignores
        // its stdin, survives the app that spawned it, and holds the
        // lid-switch lock until someone finds it with `ps`. Nothing else
        // catches that. It compiles, it passes clippy, and the machine simply
        // stops sleeping one day.
        //
        // Spawned directly rather than through `systemd-inhibit`, so this runs
        // on a developer's Mac as well as on Linux. What is under test is the
        // holder's behaviour, and `systemd-inhibit` only passes its stdin
        // through and exits when it exits.
        let mut child = std::process::Command::new(INHIBIT_HOLDER)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the inhibitor's holder command must exist on this machine");

        // Exactly what happens to the write end when this process dies. No
        // kill, deliberately — a holder that only stops when it is killed is
        // the bug.
        drop(child.stdin.take());

        let deadline = std::time::Instant::now() + EOF_DEADLINE;
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(20))
                }
                Ok(None) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!(
                        "`{INHIBIT_HOLDER}` outlived its own stdin. The logind lock would outlive \
                         this app: a crash or a force quit leaves the machine unable to sleep on \
                         the lid until the process is found by hand."
                    );
                }
                Err(error) => panic!("could not wait on the holder: {error}"),
            }
        }
    }

    #[test]
    fn releasing_a_hold_leaves_no_process_behind() {
        let mut child = std::process::Command::new(INHIBIT_HOLDER)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();

        release(&mut child);

        // Reaped, so a second wait has nothing left to find. An unreaped child
        // is a zombie for the life of the app, and this one is spawned and
        // dropped every time an agent starts and stops.
        assert!(
            child.try_wait().is_ok_and(|status| status.is_some()),
            "release must reap the child, not just kill it"
        );
        assert!(
            child.stdin.is_none(),
            "release must give up the write end, which is what a crash would do"
        );
    }

    #[test]
    fn the_inhibitor_blocks_the_lid_and_the_idle_timer_and_nothing_else() {
        let command = inhibit_command();
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        assert_eq!(command.get_program(), "systemd-inhibit");
        // `--mode=delay` would buy a few seconds and then let the machine
        // suspend anyway, which for a hold is no hold at all.
        assert!(args.contains(&"--mode=block".to_string()));
        // Not `sleep`: blocking that would stop a suspend the user asked for
        // from their own menu, which is not what this feature promises.
        assert!(args.contains(&"--what=handle-lid-switch:idle".to_string()));
        assert_eq!(
            args.last().map(String::as_str),
            Some(INHIBIT_HOLDER),
            "the holder must be the last argument, or systemd-inhibit runs something else"
        );
    }

    #[test]
    fn a_machine_with_no_readable_zone_is_unknown_rather_than_cool() {
        assert_eq!(classify_zones(&[]), Thermal::Unknown);
        // Zones that report nothing real are ignored, not banded: a driverless
        // zone reading 0 would otherwise pin every machine to `Nominal` and a
        // broken one reading millions would pin it to `Critical` forever.
        assert_eq!(classify_zones(&[0, 250_000]), Thermal::Unknown);
        assert_eq!(classify_zones(&[0, 88_000]), Thermal::Serious);
    }
}
