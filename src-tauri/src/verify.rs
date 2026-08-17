//! Manual verification harness, run against the real machine: the real
//! `Platform`, the real installed apps, the real process table. Every check here
//! launches and quits actual applications, so all of them are `#[ignore]`d and
//! none run as part of the normal suite.
//!
//! This is the admission test for a new `AppSpec`, made executable — the four
//! questions in `app_spec` answered against a real install rather than argued
//! about. Run it after declaring an app:
//!
//! ```text
//! cargo test -- --ignored --nocapture                    # all of them
//! VERIFY_APP=cursor cargo test -- --ignored launch_detect # just the new app
//! ```

#![cfg(test)]

use crate::instance_manager;
use crate::platform;
use crate::runtime;
use crate::tray::{self, AppSection};

/// Read-only: reports exactly what the Watching list would render right now,
/// from this machine's own `~/.claude/projects`.
///
/// The detector cannot be argued about from a stub. This runs the shipping
/// `cli_roots` + `scan` against the real home directory and prints the verdict
/// per row, so "it says idle while my agent is working" becomes a thing that can
/// be checked in one command rather than reported and guessed at.
///
/// ```text
/// cargo test -- --ignored --nocapture report_what_the_watch_list_would_show
/// ```
#[test]
#[ignore]
fn report_what_the_watch_list_would_show() {
    use crate::agent_activity::{cli_roots, scan};

    let home = std::path::PathBuf::from(std::env::var("HOME").expect("HOME"));
    let roots = cli_roots(&home);
    let seen = scan(&roots, std::time::SystemTime::now());

    // The window's own default. Printed rather than assumed, because the
    // verdict is only meaningful next to the number that produced it.
    let window = std::time::Duration::from_secs(2 * 60);
    println!("\nwindow: {}s\n", window.as_secs());
    println!("{:<9} {:>9} {:>9}  label", "verdict", "age", "mid_turn");
    for root in &seen {
        let fresh = root.seconds_ago.is_some_and(|ago| ago <= window.as_secs());
        let verdict = match (root.mid_turn, fresh, root.seconds_ago) {
            (_, _, None) => "never",
            (true, true, _) => "WORKING",
            (true, false, _) => "stalled",
            (false, _, _) => "idle",
        };
        println!(
            "{:<9} {:>9} {:>9}  {}",
            verdict,
            root.seconds_ago
                .map_or("-".to_string(), |ago| format!("{ago}s")),
            root.mid_turn,
            root.label,
        );
    }
    println!(
        "\nany_working = {}\n",
        crate::agent_activity::any_working(&seen, window)
    );
}

/// Read-only: reports exactly what the tray would render right now.
#[test]
#[ignore]
fn report_what_the_tray_would_show() {
    let platform = platform::current();
    let apps = runtime::build(&*platform).expect("runtimes");

    let mut sections = Vec::new();
    let mut targets = Vec::new();
    for rt in &apps {
        let unavailable = platform
            .binary(&rt.spec.locations, rt.spec.product)
            .err()
            .map(|e| e.to_string());
        println!(
            "app {:<8} available={:<5} stock={}",
            rt.spec.id,
            unavailable.is_none(),
            platform
                .default_profile_dir(&rt.spec.locations)
                .unwrap()
                .display()
        );
        if unavailable.is_none() {
            targets.push(instance_manager::scan_target(&*platform, rt.spec).expect("marker"));
        }
        sections.push(AppSection {
            spec: rt.spec,
            profiles: rt.store.lock().unwrap().list().to_vec(),
            unavailable,
        });
    }

    let processes = platform.scan(&targets).expect("scan");
    println!("\nlive processes: {}", processes.len());
    for p in &processes {
        println!(
            "  {:<8} pid={:<7} profile={:?}",
            p.app_id, p.pid, p.profile_dir
        );
    }

    println!("\ntray menu:");
    for row in tray::menu_rows(&sections, &processes, None, crate::general::Locale::En) {
        println!(
            "  [{}] {:<40} id={}",
            if row.enabled { "x" } else { " " },
            row.text,
            row.id
        );
    }
}

/// The full loop against a real app: add a profile, launch it, find it in the
/// process table by the designation we gave it, quit it, remove it.
///
/// Defaults to ChatGPT, which writes both an argument and an environment
/// variable and so exercises the harder designation path. Set `VERIFY_APP` to
/// any other declared app id to point it elsewhere.
#[test]
#[ignore]
fn launch_detect_and_quit_a_real_profile() {
    let platform = platform::current();
    let apps = runtime::build(&*platform).expect("runtimes");
    let wanted = std::env::var("VERIFY_APP").unwrap_or_else(|_| "codex".into());
    let rt = apps
        .iter()
        .find(|r| r.spec.id == wanted)
        .unwrap_or_else(|| panic!("no app {wanted}"));
    println!("verifying {} ({})", rt.spec.label, rt.spec.product);
    platform
        .binary(&rt.spec.locations, rt.spec.product)
        .unwrap_or_else(|e| panic!("{wanted} must be installed for this check: {e}"));

    let profile = {
        let mut store = rt.store.lock().unwrap();
        let created = store.add("Verify Harness", &rt.paths).expect("add");
        store.save(&rt.paths).expect("save");
        created
    };
    println!("created profile at {}", profile.path.display());

    let pid = instance_manager::launch(&*platform, rt.spec, &profile, &rt.paths).expect("launch");
    println!("launched pid {pid}");

    // Give Electron a moment to become visible in the process table.
    let target = instance_manager::scan_target(&*platform, rt.spec).expect("marker");
    let mut found = None;
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let processes = platform.scan(std::slice::from_ref(&target)).expect("scan");
        if let Some(p) = platform::find_for(&processes, rt.spec.id, &profile.path, false) {
            found = Some(p);
            break;
        }
    }
    let live = found.expect("the launched profile must be found by a scan");
    println!("scan attributed pid {live} to this profile");

    // Proof the designation actually took effect: the app wrote its own state
    // into this profile directory rather than into the user's stock one.
    match rt.spec.shared_config {
        Some(shared) => println!(
            "profile received its own {shared}: {}",
            profile.path.join(shared).exists()
        ),
        // Not every app has a file worth sharing between its profiles, and an
        // app that has none is not a failure — it simply keeps everything in
        // the profile directory.
        None => println!("this app shares no file between profiles"),
    }

    platform.quit(live).expect("quit");
    let after = platform
        .scan(std::slice::from_ref(&target))
        .expect("rescan");
    assert!(
        platform::find_for(&after, rt.spec.id, &profile.path, false).is_none(),
        "the profile must be gone from the process table after quitting"
    );
    println!("quit confirmed");

    let mut store = rt.store.lock().unwrap();
    store.remove(&profile.id, &rt.paths).expect("remove");
    store.save(&rt.paths).expect("save");
    assert!(!profile.path.exists(), "the profile directory must be gone");
    println!("cleaned up");
}

/// Both apps live at once, which is the case the app-id on every process exists
/// to get right. A scan that attributed one app's pid to the other would light
/// up the wrong tray row and offer Quit for a process the user never launched.
#[test]
#[ignore]
fn both_apps_run_side_by_side_without_being_confused() {
    let platform = platform::current();
    let apps = runtime::build(&*platform).expect("runtimes");

    let mut made = Vec::new();
    for rt in &apps {
        platform
            .binary(&rt.spec.locations, rt.spec.product)
            .expect("both apps must be installed for this check");
        let profile = {
            let mut store = rt.store.lock().unwrap();
            let p = store.add("Side By Side", &rt.paths).expect("add");
            store.save(&rt.paths).expect("save");
            p
        };
        let pid =
            instance_manager::launch(&*platform, rt.spec, &profile, &rt.paths).expect("launch");
        println!("launched {} as pid {pid}", rt.spec.label);
        made.push((rt, profile));
    }

    let targets: Vec<_> = apps
        .iter()
        .map(|rt| instance_manager::scan_target(&*platform, rt.spec).expect("marker"))
        .collect();

    // One sweep has to find both, each under its own app id.
    let mut processes = Vec::new();
    for _ in 0..30 {
        std::thread::sleep(std::time::Duration::from_millis(500));
        processes = platform.scan(&targets).expect("scan");
        let all_found = made.iter().all(|(rt, profile)| {
            platform::find_for(&processes, rt.spec.id, &profile.path, false).is_some()
        });
        if all_found {
            break;
        }
    }

    for (rt, profile) in &made {
        let pid = platform::find_for(&processes, rt.spec.id, &profile.path, false)
            .unwrap_or_else(|| panic!("{} was not found in the scan", rt.spec.label));
        println!("{:<8} -> pid {pid}", rt.spec.id);
        // The decisive assertion: asking under the OTHER app's id must miss.
        for (other, _) in &made {
            if other.spec.id != rt.spec.id {
                assert!(
                    platform::find_for(&processes, other.spec.id, &profile.path, false).is_none(),
                    "{}'s profile was attributed to {}",
                    rt.spec.id,
                    other.spec.id
                );
            }
        }
    }

    let sections: Vec<_> = apps
        .iter()
        .map(|rt| AppSection {
            spec: rt.spec,
            profiles: rt.store.lock().unwrap().list().to_vec(),
            unavailable: None,
        })
        .collect();
    println!("\ntray menu with both live:");
    for row in tray::menu_rows(&sections, &processes, None, crate::general::Locale::En) {
        println!("  {:<44} id={}", row.text, row.id);
    }

    for (rt, profile) in &made {
        if let Some(pid) = platform::find_for(&processes, rt.spec.id, &profile.path, false) {
            platform.quit(pid).expect("quit");
        }
        let mut store = rt.store.lock().unwrap();
        store.remove(&profile.id, &rt.paths).expect("remove");
        store.save(&rt.paths).expect("save");
    }
    println!("\nboth quit and cleaned up");
}
