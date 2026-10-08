//! Answers the four `AppSpec` admission questions against an application that
//! has not been declared yet, and prints a draft declaration.
//!
//! ```text
//! PROBE_APP=/Applications/Cursor.app cargo test -- --ignored probe --nocapture
//! ```
//!
//! Two of the four questions cannot be answered by looking at a bundle — they
//! are claims about behaviour, so this launches the real application twice and
//! watches what happens. It is therefore `#[ignore]`d, like the rest of the
//! manual harness, and macOS-only: it reads an `.app` bundle.
//!
//! Each question is answered by an independent source — state by the filesystem,
//! liveness by the spawned process, attribution by the shipping scanner — so
//! that two answers contradicting each other is a signal rather than a muddle.
//! Deriving one from another is how an earlier version of this file reported "a
//! single-instance lock survived" when the real story was a bug in our own
//! process parser.
//!
//! What it deliberately does NOT guess is called out as `TODO` in its output.
//! The account identity field needs someone who knows the shape of that app's
//! credential file, and the Windows and Linux locations need those machines.

#![cfg(test)]
#![cfg(target_os = "macos")]

use crate::platform::{unix_ps, unix_signal_quit, ScanTarget, DATA_DIR_NAME};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};

/// Passed on every launch, whether or not the app honours it. Even an app that
/// ignores it entirely still carries it in its command line, which is all the
/// scanner needs to attribute a process back to a profile.
const FLAG: &str = "--user-data-dir=";

/// How many environment variables to try in the discovery launch. Each gets its
/// own directory, so this is a width, not a number of launches.
const MAX_CANDIDATES: usize = 8;

/// How much of the app's own state has to land before a channel counts as
/// moving the profile.
///
/// One matching name is not enough, and OpenCode is why: it keeps a bundled
/// `mise` inside its support directory, so redirecting `XDG_DATA_HOME` moves
/// exactly that one entry and nothing else. Literally it is the app's own file;
/// substantially the profile — cookies, storage, settings — never moved. A
/// genuinely redirected profile brings a crowd: Cursor produced ten entries,
/// Devin twenty-seven.
const MIN_OVERLAP: usize = 3;

struct Bundle {
    path: PathBuf,
    /// What a person calls it, taken from the bundle's own folder name. Visual
    /// Studio Code declares `CFBundleName` and `CFBundleDisplayName` as "Code",
    /// which is what Finder hides and nobody says out loud — a tray row reading
    /// "Code" next to "Cursor" and "ChatGPT" tells a user nothing.
    display: String,
    /// `CFBundleName`, which is what the app names its own support directory
    /// after. Distinct from `display` for exactly the VS Code reason above.
    name: String,
    identifier: String,
    binary: PathBuf,
}

fn plist(bundle: &Path, key: &str) -> Option<String> {
    let out = Command::new("plutil")
        .args(["-extract", key, "raw", "-o", "-"])
        .arg(bundle.join("Contents/Info.plist"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!value.is_empty()).then_some(value)
}

fn read_bundle(path: &Path) -> Bundle {
    let executable = plist(path, "CFBundleExecutable").unwrap_or_else(|| {
        // A bundle with no declared executable still has exactly one file in
        // MacOS/, and that is what shows up in the process table.
        std::fs::read_dir(path.join("Contents/MacOS"))
            .expect("bundle has no Contents/MacOS")
            .filter_map(Result::ok)
            .next()
            .expect("bundle has no executable")
            .file_name()
            .to_string_lossy()
            .into_owned()
    });
    let name = plist(path, "CFBundleName").unwrap_or_else(|| executable.clone());
    let display = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.clone());
    Bundle {
        display,
        name,
        identifier: plist(path, "CFBundleIdentifier").unwrap_or_default(),
        binary: path.join("Contents/MacOS").join(&executable),
        path: path.to_path_buf(),
    }
}

/// A filesystem- and identifier-safe id. Names like "T3 Code (Alpha)" carry
/// spaces and brackets, which produce neither a legal Rust constant nor a
/// directory name worth having.
fn slug(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

/// A sandboxed app cannot be redirected: the system pins its container, and no
/// argument or variable will move it. This is question 2 failing before the app
/// is ever launched, which is the cheapest possible answer.
fn is_sandboxed(bundle: &Path) -> bool {
    let out = Command::new("codesign")
        .args(["-d", "--entitlements", "-"])
        .arg(bundle)
        .output();
    let Ok(out) = out else { return false };
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    match text.find("app-sandbox") {
        // The value follows the key, so only the text after it is evidence.
        Some(at) => text[at..].lines().take(3).any(|l| l.contains("true")),
        None => false,
    }
}

fn asar_paths(bundle: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(bundle.join("Contents/Resources"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|x| x == "asar"))
                .collect()
        })
        .unwrap_or_default()
}

fn is_electron(bundle: &Path) -> bool {
    let has_framework = std::fs::read_dir(bundle.join("Contents/Frameworks"))
        .map(|entries| {
            entries.filter_map(Result::ok).any(|e| {
                e.file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains("electron framework")
            })
        })
        .unwrap_or(false);
    has_framework || !asar_paths(bundle).is_empty()
}

/// Pulls `SHOUTY_SNAKE` tokens that look like they name a configuration
/// location out of a blob of text.
///
/// Deliberately generous: a variable that turns out to do nothing costs one
/// temporary directory in the discovery launch, whereas a variable missed here
/// is an app wrongly declared unsupportable.
fn env_like(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let mut token = String::new();
    for ch in text.chars().chain(std::iter::once('\n')) {
        if ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '_' {
            token.push(ch);
            continue;
        }
        if token.len() >= 6 && interesting(&token) {
            found.push(std::mem::take(&mut token));
        } else {
            token.clear();
        }
    }
    found
}

fn interesting(token: &str) -> bool {
    // TMPDIR ends in DIR and redirects nothing worth learning about; setting it
    // during discovery would move scratch files and prove nothing.
    const NEVER: [&str; 3] = ["TMPDIR", "TEMPDIR", "PATH_DIR"];
    if NEVER.contains(&token) {
        return false;
    }
    token.starts_with("XDG_")
        || token.ends_with("_HOME")
        || token.ends_with("_DIR")
        || token.ends_with("_PATH")
}

/// Where an app that defines no variable of its own still keeps things.
const XDG_CONVENTIONAL: [&str; 4] = [
    "XDG_DATA_HOME",
    "XDG_STATE_HOME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
];

/// Ordering key for a candidate variable.
///
/// Ranking decides what actually gets tested, so a bad order produces a wrong
/// answer rather than a slow one: probing OpenCode the first time pushed
/// `XDG_DATA_HOME` off the end of the list behind two test-only knobs, and the
/// verdict then named a channel that had never been tried.
fn rank(name: &str, count: usize, own: &str) -> (u8, std::cmp::Reverse<usize>, String) {
    let tier = if XDG_CONVENTIONAL.contains(&name) {
        0 // Conventional, and what an app defining no variable of its own uses.
    } else if name.contains("TEST") {
        2 // Test-only knobs move nothing a user would ever see.
    } else if name.starts_with(own) {
        0 // Named after the app itself.
    } else {
        1
    };
    (tier, std::cmp::Reverse(count), name.to_string())
}

/// The variables worth trying, most promising first.
fn env_candidates(bundle: &Bundle) -> Vec<String> {
    let own = slug(&bundle.display).to_uppercase().replace('-', "_");
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut sources = asar_paths(&bundle.path);
    sources.push(bundle.binary.clone());
    for source in sources {
        let Ok(out) = Command::new("strings").arg("-a").arg(&source).output() else {
            continue;
        };
        for token in env_like(&String::from_utf8_lossy(&out.stdout)) {
            *counts.entry(token).or_default() += 1;
        }
    }

    let mut ranked: Vec<(String, usize)> = counts.into_iter().collect();
    ranked.sort_by_key(|(name, count)| rank(name, *count, &own));
    ranked
        .into_iter()
        .map(|(name, _)| name)
        .take(MAX_CANDIDATES)
        .collect()
}

/// How long a real profile path is for this app.
///
/// The probe has to certify under the conditions production imposes. Testing in
/// a short temporary directory once certified VS Code, which then failed at the
/// real profile path because the socket it puts inside a profile no longer fitted
/// inside `sun_path`. A verdict obtained under easier conditions than the ones
/// that will apply is not a verdict.
fn production_length(app_id: &str) -> usize {
    let root = home()
        .join("Library/Application Support")
        .join(DATA_DIR_NAME)
        .join(app_id);
    crate::paths::Paths::new(root)
        .profile_dir("9f3c1a7e")
        .display()
        .to_string()
        .len()
}

/// A temporary directory padded out to the length production would use.
fn padded_root(app_id: &str) -> PathBuf {
    let want = production_length(app_id);
    let mut name = format!("agent-profiles-probe-{app_id}");
    loop {
        let candidate = std::env::temp_dir().join(&name);
        if candidate.join("argv").display().to_string().len() >= want {
            return candidate;
        }
        name.push('x');
    }
}

fn home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").expect("HOME"))
}

/// Where the app already keeps its state, which becomes the stock profile.
fn stock_profile(bundle: &Bundle) -> Option<PathBuf> {
    let support = home().join("Library/Application Support");
    // `CFBundleName` first: it is what an app names its support directory
    // after, and it is not always what the bundle is called.
    [
        bundle.name.clone(),
        bundle.display.clone(),
        bundle.identifier.clone(),
    ]
    .into_iter()
    .map(|candidate| support.join(candidate))
    .find(|candidate| candidate.is_dir())
    .map(|found| {
        found
            .strip_prefix(home())
            .expect("under home")
            .to_path_buf()
    })
}

/// Names what landed, rather than counting it. A bare count is not evidence:
/// one entry might be a real profile taking shape, or a stray lock file.
fn entries(dir: &Path) -> Vec<String> {
    let mut found: Vec<String> = std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    found.sort();
    found
}

/// What the app's own state looks like, taken from the profile it already uses.
fn stock_entries(stock: Option<&PathBuf>) -> Option<HashSet<String>> {
    let set: HashSet<String> = entries(&home().join(stock?)).into_iter().collect();
    (!set.is_empty()).then_some(set)
}

/// Splits what landed into the app's own state and everything else.
///
/// "The directory received writes" is not the same claim as "the app's profile
/// moved here", and probing OpenCode showed the gap: setting `XDG_DATA_HOME`
/// filled the directory with `mise` and `fsh`, because the app spawns a shell
/// that inherits the environment. Names the stock profile also uses are the
/// app's own; the rest is collateral from whatever it started.
fn classify(written: &[String], stock: Option<&HashSet<String>>) -> (Vec<String>, Vec<String>) {
    let Some(stock) = stock else {
        // With no stock profile to compare against there is nothing to tell them
        // apart, so nothing is claimed and the caller says so.
        return (written.to_vec(), Vec::new());
    };
    written
        .iter()
        .cloned()
        .partition(|name| stock.contains(name))
}

fn launch(binary: &Path, argv_dir: &Path, env: &[(String, PathBuf)]) -> Child {
    let mut command = Command::new(binary);
    command.arg(format!("{FLAG}{}", argv_dir.display()));
    for (name, dir) in env {
        command.env(name, dir);
    }
    command
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn")
}

/// Uses the real scanner, not a copy of it: the question is not "is a process
/// running" but "would this application find it", and only the shipping parser
/// can answer that.
fn find(binary: &Path, dir: &Path) -> Option<i32> {
    let target = ScanTarget {
        app_id: "probe",
        marker: binary.display().to_string(),
        flag: FLAG,
    };
    unix_ps::scan(std::slice::from_ref(&target))
        .ok()?
        .into_iter()
        .find(|p| p.profile_dir.as_deref() == Some(dir))
        .map(|p| p.pid)
}

fn wait_for(binary: &Path, dir: &Path, seconds: u64) -> Option<i32> {
    for _ in 0..(seconds * 2) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        if let Some(pid) = find(binary, dir) {
            return Some(pid);
        }
    }
    None
}

/// Polls rather than sampling once. An app appears in the process table within a
/// second of launching but takes several more to write anything, so checking
/// immediately reports "the flag was ignored" for an app that honours it
/// perfectly well — a false negative that would reject a supportable app.
fn wait_for_any_state(dirs: &[PathBuf], seconds: u64) {
    for _ in 0..(seconds * 2) {
        if dirs.iter().any(|dir| !entries(dir).is_empty()) {
            // One more beat, so what is reported is a settled directory rather
            // than whichever file happened to be created first.
            std::thread::sleep(std::time::Duration::from_secs(3));
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
}

fn summarise(written: &[String]) -> String {
    const SHOWN: usize = 8;
    if written.len() <= SHOWN {
        return written.join(", ");
    }
    format!(
        "{}, … (+{} more)",
        written[..SHOWN].join(", "),
        written.len() - SHOWN
    )
}

#[test]
fn the_probe_tests_at_least_as_long_a_path_as_production_uses() {
    // Anything shorter and the probe grants a pass that production revokes.
    let want = production_length("code");
    let got = padded_root("code").join("argv").display().to_string().len();
    assert!(got >= want, "probed at {got} bytes, production uses {want}");
}

#[test]
fn a_bundle_name_becomes_a_legal_id() {
    // Not ignored: it needs no application, and a malformed id produces a draft
    // declaration that does not compile.
    assert_eq!(slug("T3 Code (Alpha)"), "t3-code-alpha");
    assert_eq!(slug("Cursor"), "cursor");
    assert_eq!(slug("Visual Studio Code"), "visual-studio-code");
}

#[test]
fn configuration_variables_are_recognised_and_noise_is_not() {
    let found = env_like("process.env.OPENCODE_CONFIG_DIR ?? XDG_DATA_HOME; TMPDIR; AB_C");
    assert!(found.contains(&"OPENCODE_CONFIG_DIR".to_string()));
    assert!(found.contains(&"XDG_DATA_HOME".to_string()));
    // TMPDIR only moves scratch files, and AB_C is too short to be anything but
    // a fragment of something else.
    assert!(!found.contains(&"TMPDIR".to_string()));
    assert!(!found.contains(&"AB_C".to_string()));
}

#[test]
fn a_test_only_knob_never_crowds_out_a_conventional_location() {
    // Order is the whole answer: anything past MAX_CANDIDATES is never tried,
    // and a channel never tried gets reported as a channel that failed.
    let mut names = ["DEMO_TEST_HOME", "XDG_DATA_HOME", "SOMETHING_ELSE_DIR"];
    // The test knob is given the highest occurrence count on purpose, so only
    // the tier can rescue the conventional one.
    let counts = |name: &str| if name == "DEMO_TEST_HOME" { 99 } else { 1 };
    names.sort_by_key(|name| rank(name, counts(name), "DEMO"));
    assert_eq!(names[0], "XDG_DATA_HOME");
    assert_eq!(names[2], "DEMO_TEST_HOME");
}

#[test]
fn a_variable_named_after_its_app_outranks_an_unrelated_one() {
    let mut names = ["UNRELATED_DIR", "DEMO_CONFIG_DIR"];
    names.sort_by_key(|name| rank(name, 1, "DEMO"));
    assert_eq!(names[0], "DEMO_CONFIG_DIR");
}

#[test]
fn writes_from_a_child_process_are_not_mistaken_for_the_app_moving() {
    // Real case: pointing XDG_DATA_HOME at a fresh directory filled it with
    // `mise`, because OpenCode spawns a shell that inherits the environment.
    // Counting that as success would declare an app whose profile never moved.
    let stock: HashSet<String> = ["Cookies", "Local State", "Preferences"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (own, collateral) = classify(
        &["mise".into(), "fsh".into(), "Cookies".into()],
        Some(&stock),
    );
    assert_eq!(own, vec!["Cookies"]);
    assert_eq!(collateral, vec!["mise", "fsh"]);
}

#[test]
fn one_shared_entry_is_not_a_profile_moving() {
    // OpenCode keeps a bundled `mise` in its support directory. Redirecting
    // XDG_DATA_HOME moves that and nothing else — literally the app's own file,
    // substantially not its profile.
    let stock: HashSet<String> = ["Cookies", "Local State", "Preferences", "mise"]
        .iter()
        .map(|s| s.to_string())
        .collect();
    let (own, _) = classify(&["mise".into()], Some(&stock));
    assert_eq!(own.len(), 1);
    assert!(own.len() < MIN_OVERLAP, "one entry must not count as moved");

    let (many, _) = classify(
        &["Cookies".into(), "Local State".into(), "Preferences".into()],
        Some(&stock),
    );
    assert!(many.len() >= MIN_OVERLAP, "a real profile brings a crowd");
}

#[test]
fn with_no_stock_profile_nothing_is_claimed_to_be_collateral() {
    let (own, collateral) = classify(&["Local State".into()], None);
    assert_eq!(own, vec!["Local State"]);
    assert!(collateral.is_empty());
}

#[test]
#[ignore]
fn probe_an_app_bundle() {
    let path = PathBuf::from(
        std::env::var("PROBE_APP").expect("set PROBE_APP=/Applications/Something.app"),
    );
    assert!(path.is_dir(), "{} is not a bundle", path.display());
    let bundle = read_bundle(&path);

    println!("\n=== {} ===", bundle.path.display());
    println!("identifier   {}", bundle.identifier);
    println!("binary       {}", bundle.binary.display());
    println!("electron     {}", is_electron(&bundle.path));

    // Question 2, cheap half: a sandboxed app is unsupportable, full stop.
    let sandboxed = is_sandboxed(&bundle.path);
    println!("sandboxed    {sandboxed}");
    assert!(
        !sandboxed,
        "FAILS question 2: the system pins a sandboxed app's container, so no \
         argument or variable can move its profile. This app cannot be supported."
    );

    let stock = stock_profile(&bundle);
    println!(
        "stock dir    {}",
        stock
            .as_ref()
            .map(|s| format!("~/{}", s.display()))
            .unwrap_or_else(|| "NOT FOUND — run the app once first".into())
    );

    let candidates = env_candidates(&bundle);
    println!(
        "env vars     {}",
        if candidates.is_empty() {
            "none found".to_string()
        } else {
            candidates.join(", ")
        }
    );

    // The id is ours to choose, and it is charged against the socket budget like
    // everything else: "visual-studio-code" costs fourteen bytes more than
    // "code" and that alone can put an app past the limit. Override it here to
    // try a shorter one.
    let id = std::env::var("PROBE_ID").unwrap_or_else(|_| slug(&bundle.display));
    let root = padded_root(&id);
    let profile_len = root.join("argv").display().to_string().len();
    let budget = match crate::paths::SOCKET_PATH_LIMIT {
        Some(limit) => limit.to_string(),
        // Windows keeps its named pipes outside the profile, so there is no
        // budget to report rather than an unlimited one.
        None => "no limit on this platform".to_string(),
    };
    println!(
        "path length  {profile_len} bytes as id {id:?} (real layout), socket needs {} of {budget}",
        profile_len + "/1.13-main.sock".len()
    );
    if crate::paths::socket_refusal(&root.join("argv")).is_some() {
        println!("             ^ OVER BUDGET — an app putting a socket in its profile will");
        println!("               fail here. Re-run with a shorter id to see if that is all");
        println!("               that stands in the way: PROBE_ID=<short> …");
    }
    let _ = std::fs::remove_dir_all(&root);
    let argv_dir = root.join("argv");
    let env_dirs: Vec<(String, PathBuf)> = candidates
        .iter()
        .enumerate()
        .map(|(i, name)| (name.clone(), root.join(format!("env-{i}"))))
        .collect();
    let mut watched = vec![argv_dir.clone()];
    watched.extend(env_dirs.iter().map(|(_, dir)| dir.clone()));
    for dir in &watched {
        std::fs::create_dir_all(dir).unwrap();
    }

    // One launch answers every channel at once, because each channel is pointed
    // somewhere different: whichever directories fill up name the channels this
    // app actually honours.
    println!("\n--- launch 1: every channel, each pointed somewhere different ---");
    let mut first = launch(&bundle.binary, &argv_dir, &env_dirs);
    let pid_a = wait_for(&bundle.binary, &argv_dir, 20);
    wait_for_any_state(&watched, 25);

    // Question 3. This holds even for an app that ignores the argument: the flag
    // is still in its command line, which is all the scanner reads.
    println!(
        "readback     {}",
        match pid_a {
            Some(pid) => format!("YES — the shipping scanner attributed pid {pid}"),
            None => "NO".into(),
        }
    );

    let stock_state = stock_entries(stock.as_ref());
    if stock_state.is_none() {
        println!("  (no stock profile to compare against — any writes are taken at face value)");
    }
    let report = |label: &str, dir: &Path| -> bool {
        let (own, collateral) = classify(&entries(dir), stock_state.as_ref());
        if own.len() >= MIN_OVERLAP {
            println!("  {label:<22} HONOURED — {}", summarise(&own));
        } else if !own.is_empty() {
            println!(
                "  {label:<22} PARTIAL — only {} moved, which is a side directory rather \
                 than the profile",
                summarise(&own)
            );
        } else if !collateral.is_empty() {
            // Worth printing rather than swallowing: it is the difference
            // between "nothing happened" and "something happened that is not
            // this app", and only the second is worth investigating.
            println!(
                "  {label:<22} ignored (collateral only: {})",
                summarise(&collateral)
            );
        } else {
            println!("  {label:<22} ignored");
        }
        own.len() >= MIN_OVERLAP
    };

    let argv_moves = report("argv", &argv_dir);
    let mut working: Vec<String> = Vec::new();
    for (name, dir) in &env_dirs {
        if report(name, dir) {
            working.push(name.clone());
        }
    }
    let designated = argv_moves || !working.is_empty();

    // Question 4, using the designation this app actually responds to, and with
    // every honoured channel pointed at ONE directory — which is what question 1
    // demands and what production would do.
    println!("\n--- launch 2: a second profile, using only the channels that worked ---");
    let b = root.join("second");
    std::fs::create_dir_all(&b).unwrap();
    let combined: Vec<(String, PathBuf)> = working.iter().map(|n| (n.clone(), b.clone())).collect();
    let mut second = launch(&bundle.binary, &b, &combined);
    let _ = wait_for(&bundle.binary, &b, 20);
    wait_for_any_state(std::slice::from_ref(&b), 20);

    // Liveness is asked of the spawned processes directly, never derived from
    // the scanner: those are two independent questions, and conflating them once
    // hid a parser bug behind a false "single-instance lock" verdict.
    let alive = |child: &mut Child| matches!(child.try_wait(), Ok(None));
    let both_alive = alive(&mut first) && alive(&mut second);
    println!(
        "side by side {}",
        if both_alive {
            "YES — both instances stayed alive"
        } else {
            "NO — a single-instance lock survived the split"
        }
    );
    let (second_state, _) = classify(&entries(&b), stock_state.as_ref());
    let second_moved = second_state.len() >= MIN_OVERLAP;
    println!(
        "one dir      {}",
        if !second_moved {
            "NO — the app's own state did not land when every channel shared one directory"
                .to_string()
        } else {
            format!("YES — {}", summarise(&second_state))
        }
    );

    if let Some(pid) = pid_a {
        let _ = unix_signal_quit(pid);
    }
    for child in [&mut first, &mut second] {
        let _ = child.kill();
        let _ = child.wait();
    }
    let _ = std::fs::remove_dir_all(&root);

    let verdict = pid_a.is_some() && designated && both_alive && second_moved;
    println!(
        "\n=== {} ===",
        if verdict {
            "SUPPORTABLE"
        } else {
            "NOT SUPPORTABLE"
        }
    );
    if !verdict {
        if !designated {
            println!("No channel moved this app's state. It keeps its profile somewhere this");
            println!("probe cannot redirect, so a profile cannot be expressed as a directory.");
        } else if pid_a.is_none() {
            println!("Nothing could attribute a running process back to its profile, and the");
            println!("guard against two processes sharing one directory depends on that.");
        } else {
            println!("Do not declare this app: one of the four questions answered no above.");
        }
        return;
    }

    println!("\nDraft declaration for src-tauri/src/app_spec.rs:\n");
    println!(
        "pub static {}: AppSpec = AppSpec {{",
        id.to_uppercase().replace('-', "_")
    );
    println!("    id: {id:?},");
    println!("    label: {:?},", bundle.display);
    println!("    product: {:?},", bundle.display);
    println!("    locations: Locations {{");
    println!("        macos: MacLocation {{");
    println!(
        "            binary: {:?},",
        bundle.binary.display().to_string()
    );
    println!(
        "            default_profile: {:?},",
        stock
            .map(|s| s.display().to_string())
            .unwrap_or_else(|| "TODO".into())
    );
    println!("        }},");
    println!("        linux: LinuxLocation {{ command: \"TODO\", default_profile: \"TODO\", install_hint: \"TODO\" }},");
    println!("        windows: WindowsLocation {{ binaries: &[/* TODO */], default_profiles: &[/* TODO */], process_name: \"TODO\", package_family: None }},");
    println!("    }},");
    println!("    designation: Designation {{");
    println!("        writes: &[");
    println!("            Designator::Arg(\"--user-data-dir={{}}\"),");
    for name in &working {
        println!("            Designator::Env({name:?}),");
    }
    println!("        ],");
    println!("        read_from: Readback::Arg(\"--user-data-dir=\"),");
    println!("    }},");
    println!("    shared_config: None, // TODO: a file every profile should share, if any");
    println!("    identity: None, // TODO: needs the shape of this app's credential file");
    println!("    capabilities: Capabilities {{ focus: true, desktop_identity: true }},");
    println!("}};");

    if !argv_moves {
        println!("\nNOTE: this app ignores --user-data-dir; the variables above are what");
        println!("actually move its profile. The argument is still written, as the tag the");
        println!("scanner reads a process back by — the same split ChatGPT already uses.");
        println!("It is inert today, and were a future version to honour it, it would point");
        println!("at the very directory the variables already do.");
    }
    println!("\nThe TODOs are the parts a probe cannot know: Windows and Linux need those");
    println!("machines, and the identity field needs someone who knows that app's files.");
}
