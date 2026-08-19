use std::path::PathBuf;

/// The container every profile directory hangs off, kept to one character on
/// purpose. See [`SOCKET_PATH_LIMIT`] — every byte spent here is a byte an
/// application cannot spend on the socket it puts inside a profile.
const PROFILES_DIR: &str = "p";

/// macOS caps a Unix domain socket path at 104 bytes (`sun_path`), and Linux at
/// 108. Several of the applications we hold profiles for put a socket inside the
/// profile directory — VS Code writes `<version>-main.sock`, the ChatGPT desktop
/// app writes `ipc/ipc.sock` — so the length of a profile path is not cosmetic.
///
/// Measured, not assumed: at a 94-byte socket path VS Code came up with nine
/// processes and created its socket; at 109 bytes exactly one process survived
/// and no socket appeared. ChatGPT is less brittle and merely loses its socket
/// silently, which is worse to diagnose.
///
/// This is why a profile directory is `<root>/p/<short id>` rather than
/// `<root>/profiles/<uuid>`: the latter left a real installation 17 bytes over
/// the limit before the application had written a single byte.
pub const MACOS_SOCKET_PATH_LIMIT: usize = 104;
pub const LINUX_SOCKET_PATH_LIMIT: usize = 108;

/// The limit in force on the platform this build runs on.
///
/// `None` is Windows, and it means there is no budget to keep rather than a
/// generous one. Windows named pipes live in their own namespace under
/// `\\.\pipe\`, not inside the profile directory, so applying a `sun_path` cap
/// there would invent a limit — and refuse a profile a Windows user could
/// perfectly well have had, citing a number that means nothing on their machine.
pub const SOCKET_PATH_LIMIT: Option<usize> = if cfg!(target_os = "macos") {
    Some(MACOS_SOCKET_PATH_LIMIT)
} else if cfg!(target_os = "linux") {
    Some(LINUX_SOCKET_PATH_LIMIT)
} else {
    None
};

/// The longest socket name seen inside a profile, plus its separator: VS Code's
/// `1.13-main.sock`. ChatGPT's `ipc/ipc.sock` is shorter, so budgeting for the
/// longer one covers both.
const SOCKET_NAME_BUDGET: usize = "/1.13-main.sock".len();

/// How long the socket path inside this profile directory would be.
///
/// The one place the socket-name budget is added, so the number the window
/// draws and the number `socket_refusal` decides on cannot drift apart.
pub fn socket_path_len(profile_dir: &std::path::Path) -> usize {
    profile_dir.display().to_string().len() + SOCKET_NAME_BUDGET
}

/// Whether a profile at this path leaves room for a socket, against one limit.
///
/// Split out from the platform question so the guard can be tested against every
/// platform's number from any platform. Otherwise the only assertion that runs
/// on Windows CI is the vacuous one.
pub fn fits_within(profile_dir: &std::path::Path, limit: usize) -> bool {
    socket_path_len(profile_dir) <= limit
}

/// Why an application could not create its socket inside this profile, if it
/// could not.
///
/// The layout is short enough that this holds comfortably for any ordinary home
/// directory, but the home directory is not ours to choose. Refusing to create a
/// profile that cannot work is the same fail-closed choice made when a process
/// scan fails: a profile that half-works is far harder to diagnose than one that
/// was never created. Where no limit exists, there is nothing to refuse.
pub fn socket_refusal(profile_dir: &std::path::Path) -> Option<String> {
    let limit = SOCKET_PATH_LIMIT?;
    if fits_within(profile_dir, limit) {
        return None;
    }
    Some(format!(
        "this profile's path would be {} characters, leaving no room for the socket \
         applications create inside a profile (the system limit is {limit}). \
         Applications launched from it would fail to start or silently lose features, \
         so it is not created.",
        profile_dir.display().to_string().len()
    ))
}

/// Every path belonging to one app's profiles. Rooted at `<data root>/<app id>`,
/// so two apps never share a registry, a profile directory, or a shared config.
pub struct Paths {
    root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn profiles_json(&self) -> PathBuf {
        self.root.join("profiles.json")
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join(PROFILES_DIR)
    }

    pub fn profile_dir(&self, id: &str) -> PathBuf {
        self.profiles_dir().join(id)
    }

    /// The one copy of `filename` that every profile of this app links to.
    pub fn shared_config(&self, filename: &str) -> PathBuf {
        self.root.join("shared").join(filename)
    }
}

/// The keep-awake files, which belong to the machine rather than to any one
/// app: there is one system sleep setting, not one per profile. They therefore
/// sit at the data root itself rather than inside a [`Paths`], whose whole job
/// is to keep two apps from sharing anything.
pub fn keep_awake_settings(data_root: &std::path::Path) -> PathBuf {
    data_root.join("keep-awake.json")
}

/// The app's own settings — whether it updates itself, and what language it
/// speaks. Beside `keep-awake.json` at the data root rather than inside a
/// [`Paths`], for the same reason: these are facts about the installation, not
/// about Claude or Codex.
///
/// Called from `general::Handle::new`, which `setup()` and `runtime.rs`
/// construct at startup.
pub fn general_settings(data_root: &std::path::Path) -> PathBuf {
    data_root.join("general.json")
}

/// Existence means "hold the machine awake". Never read, only tested for — its
/// contents would otherwise be interpolated into a root shell.
pub fn keep_awake_flag(data_root: &std::path::Path) -> PathBuf {
    data_root.join("keep-awake.hold")
}

/// Who owns `SleepDisabled`, written before anything can be held so the answer
/// survives a process that dies without running its teardown.
pub fn keep_awake_breadcrumb(data_root: &std::path::Path) -> PathBuf {
    data_root.join("keep-awake.owned")
}

/// What closing the lid did before Windows took the setting over.
///
/// Its own file rather than the breadcrumb above, which records a single
/// `SleepDisabled` bit and is consumed by `recover_at_startup` before anything
/// platform-specific gets a look at it. This one holds two values — mains and
/// battery — and is read back by the Windows backend itself, at the next
/// startup if a run died owning it.
///
/// The one keep-awake path only one platform reads, hence the allow: elsewhere
/// there is no lid setting to own, and going unused there is correct rather
/// than a mistake worth hearing about.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub fn keep_awake_lid_prior(data_root: &std::path::Path) -> PathBuf {
    data_root.join("keep-awake.lid")
}

/// Why this data root cannot be spelled inside the privileged watchdog, if it
/// cannot.
///
/// The path is the only variable part of a string that passes through an
/// AppleScript literal and then a root shell. Escaping both layers correctly is
/// possible and is exactly the kind of thing that is subtly wrong for years, so
/// the four characters that would need it are refused instead. A home directory
/// containing a quote, a backslash or a newline is vanishingly rare; spaces,
/// which are not rare at all, are handled by single-quoting and are fine.
///
/// Fail-closed, the same choice [`socket_refusal`] makes for a path that is
/// merely too long: the feature is withheld with a reason rather than shipped
/// with a hole.
pub fn unquotable_refusal(data_root: &std::path::Path) -> Option<String> {
    let shown = data_root.display().to_string();
    let found = shown
        .chars()
        // `\r` belongs here as much as `\n`: AppleScript ends a statement on a
        // carriage return too, so a path carrying one truncates the elevated
        // script mid-command exactly as a newline would. macOS permits any byte
        // but `/` and NUL in a filename, so this is reachable, not theoretical.
        .find(|c| matches!(c, '\'' | '"' | '\\' | '\n' | '\r'))?;
    Some(format!(
        "keep awake is unavailable because this profile folder's path contains \
         {found:?}: {shown:?}. The privileged helper spells the path into a shell \
         command, and a path carrying a quote, a backslash or a newline cannot be \
         spelled there safely."
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn paths_are_rooted_at_the_given_directory() {
        let p = Paths::new("/root/claude");
        assert_eq!(
            p.profiles_json(),
            PathBuf::from("/root/claude/profiles.json")
        );
        assert_eq!(p.profiles_dir(), PathBuf::from("/root/claude/p"));
        assert_eq!(p.profile_dir("abc"), PathBuf::from("/root/claude/p/abc"));
        assert_eq!(
            p.shared_config("claude_desktop_config.json"),
            PathBuf::from("/root/claude/shared/claude_desktop_config.json")
        );
    }

    #[test]
    fn two_apps_never_collide() {
        // The registries must not overlap: a shared `profiles.json` would make a
        // profile id ambiguous, and the stock-profile entry would be written twice.
        let claude = Paths::new("/root/claude");
        let codex = Paths::new("/root/codex");
        assert_ne!(claude.profiles_json(), codex.profiles_json());
        assert_ne!(claude.profile_dir("abc"), codex.profile_dir("abc"));
        assert_ne!(
            claude.shared_config("config.toml"),
            codex.shared_config("config.toml")
        );
    }

    #[test]
    fn a_profile_directory_can_never_be_mistaken_for_our_own_files() {
        // Profiles keep their own container rather than sitting directly under
        // the root, so no profile id can ever collide with `profiles.json` or
        // `shared` and quietly shadow one of them.
        let p = Paths::new("/root/claude");
        assert_ne!(
            p.profile_dir("shared"),
            p.shared_config("x").parent().unwrap()
        );
        assert_ne!(p.profile_dir("profiles.json"), p.profiles_json());
    }

    /// The real layout on a real machine, against the real limit.
    fn measured_socket_path_len(home: &str, app: &str, id: &str, socket: &str) -> usize {
        let root = Path::new(home)
            .join("Library/Application Support")
            .join("Agent Profiles")
            .join(app);
        Paths::new(root)
            .profile_dir(id)
            .join(socket)
            .display()
            .to_string()
            .len()
    }

    #[test]
    fn the_socket_length_is_the_directory_plus_the_socket_name() {
        // The window draws this number, so it must be the same one `fits_within`
        // decides on — not a second, independently drifting calculation.
        let dir = Path::new("/Users/husni/x/p/9f3c1a7e");
        assert_eq!(
            socket_path_len(dir),
            dir.display().to_string().len() + "/1.13-main.sock".len()
        );
        assert!(fits_within(dir, MACOS_SOCKET_PATH_LIMIT));
    }

    #[test]
    fn fits_within_pins_the_boundary_at_the_limit() {
        // A comparison against `fits_within`'s own `<=` body would be tautological
        // — it can't fail unless the two sides fall out of sync, which is a compile
        // error, not a test failure. Pin the real boundary instead: a path whose
        // socket path lands exactly on the limit must fit, and one byte more must not.
        let prefix = "/Users/x/p/";
        let pad_len = MACOS_SOCKET_PATH_LIMIT - SOCKET_NAME_BUDGET - prefix.len();

        let exactly_at_limit = PathBuf::from(format!("{prefix}{}", "n".repeat(pad_len)));
        assert_eq!(socket_path_len(&exactly_at_limit), MACOS_SOCKET_PATH_LIMIT);
        assert!(fits_within(&exactly_at_limit, MACOS_SOCKET_PATH_LIMIT));

        let one_byte_over = PathBuf::from(format!("{prefix}{}", "n".repeat(pad_len + 1)));
        assert_eq!(socket_path_len(&one_byte_over), MACOS_SOCKET_PATH_LIMIT + 1);
        assert!(!fits_within(&one_byte_over, MACOS_SOCKET_PATH_LIMIT));
    }

    #[test]
    fn a_path_with_no_room_left_is_rejected() {
        let roomy = PathBuf::from(
            "/Users/husni/Library/Application Support/Agent Profiles/code/p/9f3c1a7e",
        );
        assert!(fits_within(&roomy, MACOS_SOCKET_PATH_LIMIT));

        // A home directory long enough to eat the budget: the profile is still a
        // legal path, it simply cannot host the socket an app will want.
        let cramped = PathBuf::from(format!("/Users/{}/x/p/9f3c1a7e", "n".repeat(90)));
        assert!(!fits_within(&cramped, MACOS_SOCKET_PATH_LIMIT));
    }

    #[test]
    fn a_profile_leaves_room_for_the_socket_an_app_puts_inside_it() {
        // VS Code's is the longest of the ones measured.
        let len = measured_socket_path_len("/Users/husni", "code", "9f3c1a7e", "1.13-main.sock");
        assert!(
            len <= MACOS_SOCKET_PATH_LIMIT,
            "a profile path must leave room for a socket, got {len}"
        );
    }

    #[test]
    fn there_is_headroom_for_a_long_user_name() {
        // The home directory is not ours to choose, so the layout has to survive
        // a name considerably longer than the author's.
        let len = measured_socket_path_len(
            "/Users/christopher.anderson",
            "code",
            "9f3c1a7e",
            "1.13-main.sock",
        );
        assert!(
            len <= MACOS_SOCKET_PATH_LIMIT,
            "no headroom left, got {len}"
        );
    }

    #[test]
    fn the_layout_this_replaced_would_not_have_fitted() {
        // Guards the reason for the short names: a uuid under `profiles/` put a
        // real installation over the limit before the app wrote anything.
        let old = Path::new("/Users/husni")
            .join("Library/Application Support/Agent Profiles/code/profiles")
            .join("9f3c1a7e-4b2d-4c8e-9a1f-2e5d7b6c0a83")
            .join("1.13-main.sock")
            .display()
            .to_string()
            .len();
        assert!(
            old > MACOS_SOCKET_PATH_LIMIT,
            "the old layout fitted after all: {old}"
        );
    }

    #[test]
    fn a_platform_with_no_such_limit_refuses_nothing() {
        // Windows puts its named pipes in `\\.\pipe\`, not inside the profile,
        // so there is no budget to keep. Applying the macOS number there would
        // refuse a profile a Windows user could perfectly well have had.
        let absurd = PathBuf::from(format!(r"C:\Users\{}\p\9f3c1a7e", "n".repeat(200)));
        if SOCKET_PATH_LIMIT.is_none() {
            assert!(socket_refusal(&absurd).is_none());
        } else {
            assert!(socket_refusal(&absurd).is_some());
        }
    }

    #[test]
    fn a_refusal_names_the_length_and_the_limit_it_broke() {
        // The message is the only thing a user has to go on, and "too long" on
        // its own tells them neither how long nor how long is allowed.
        let Some(limit) = SOCKET_PATH_LIMIT else {
            return;
        };
        let cramped = PathBuf::from(format!("/Users/{}/x/p/9f3c1a7e", "n".repeat(120)));
        let reason = socket_refusal(&cramped).expect("a path this long must be refused");
        assert!(reason.contains(&limit.to_string()), "got: {reason}");
        assert!(
            reason.contains(&cramped.display().to_string().len().to_string()),
            "got: {reason}"
        );
    }

    #[test]
    fn the_keep_awake_files_sit_beside_the_per_app_directories() {
        // App-wide, not per app: one machine has one sleep setting. They must
        // also be unmistakable for an app id — every id today is a bare word,
        // and none of these are.
        let root = Path::new("/data");
        assert_eq!(
            keep_awake_settings(root),
            PathBuf::from("/data/keep-awake.json")
        );
        assert_eq!(
            keep_awake_flag(root),
            PathBuf::from("/data/keep-awake.hold")
        );
        assert_eq!(
            keep_awake_breadcrumb(root),
            PathBuf::from("/data/keep-awake.owned")
        );
        assert_eq!(
            keep_awake_lid_prior(root),
            PathBuf::from("/data/keep-awake.lid")
        );
        for spec in crate::app_spec::all() {
            assert_ne!(keep_awake_flag(root), root.join(spec.id));
        }
    }

    #[test]
    fn an_ordinary_data_root_is_accepted() {
        // Spaces are the common case — `Application Support` has one — and must
        // not be refused. Single-quoting in the shell handles them.
        let ok =
            PathBuf::from("/Users/christopher anderson/Library/Application Support/Agent Profiles");
        assert!(unquotable_refusal(&ok).is_none());
    }

    #[test]
    fn a_data_root_that_would_need_escaping_is_refused_by_name() {
        // The one place a home directory reaches a root shell. Rather than
        // escape three nested quoting layers correctly, refuse the four
        // characters that would need it.
        for bad in [
            "/Users/o'brien/x",
            "/Users/a\"b/x",
            "/Users/a\\b/x",
            "/Users/a\nb/x",
            // AppleScript ends a statement on a carriage return as well as a
            // newline, so this truncates the elevated script just the same.
            "/Users/a\rb/x",
        ] {
            let reason = unquotable_refusal(Path::new(bad))
                .unwrap_or_else(|| panic!("{bad:?} must be refused"));
            assert!(
                reason.contains("keep awake"),
                "the refusal must name the feature it disables, got: {reason}"
            );
        }
    }
}
