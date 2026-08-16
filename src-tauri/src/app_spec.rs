//! Everything that differs between the apps we can hold profiles for, expressed
//! as data rather than behaviour.
//!
//! The rule that keeps this file cheap to extend: a new app is a new `AppSpec`
//! constant and nothing else. If adding one would require editing a `Platform`
//! backend, something app-specific has leaked into the OS axis and belongs back
//! here instead.
//!
//! The dependency runs one way only — `platform` reads these declarations, and
//! nothing here knows a `Platform` exists.

/// A profile-switchable application.
///
/// Admission test, all four of which must hold before an app can be described
/// here at all:
///
/// 1. a profile can be expressed as one directory;
/// 2. that directory can be selected at launch, through argv or the environment;
/// 3. the selection can be read back off a running process;
/// 4. no global lock survives the directories being separated.
///
/// An app that fails any of them cannot be supported by this design — a
/// sandboxed app fails (2), and an app keeping its credentials in the system
/// keychain fails (1). Those are limits, not bugs to be patched around.
pub struct AppSpec {
    /// Stable, filesystem-safe key. Names the app's data directory and prefixes
    /// every tray row id, so changing it orphans a user's profiles.
    pub id: &'static str,
    /// What the tray calls it.
    pub label: &'static str,
    /// What error messages call it, in full.
    pub product: &'static str,
    pub locations: Locations,
    pub designation: Designation,
    /// A file every profile shares, linked into each profile directory. `None`
    /// means each profile keeps its own copy of everything.
    pub shared_config: Option<&'static str>,
    /// How to tell which account a profile is signed into. `None` means we
    /// cannot tell, and the "same account" warning is simply never shown.
    pub identity: Option<Identity>,
    /// Where this app's agent sessions leave a trace inside a profile
    /// directory, relative to it. `None` means this app writes no observable
    /// session trace and the keep-awake trigger simply never fires for it.
    ///
    /// Only Codex qualifies today, and only because this app already points
    /// `CODEX_HOME` at the profile directory — so a profile-launched Codex
    /// writes its sessions inside the profile rather than under the home
    /// directory. The Electron apps write continuously whether or not anyone is
    /// working, which is a signal that is always on and therefore no signal.
    pub session_trace: Option<&'static str>,
    pub capabilities: Capabilities,
}

/// Where the app lives on each OS. The one genuinely irreducible cell of the
/// app × OS matrix: only the app knows the names, only the OS knows the roots.
/// Keeping it as a table means the OS backends interpret it without ever
/// naming a particular app.
///
/// Each row is optional, and `None` means "nobody has checked this platform" —
/// not "unsupported". An app declared only where someone has actually run it
/// simply does not appear elsewhere, which is the honest alternative to
/// inventing a path and shipping a guess.
/// Only one row of this table is read on any given build, so every field looks
/// unused to the compiler on two platforms out of three. The allow is declared
/// once, here at the source of the data, rather than as a scattering of
/// per-backend suppressions.
#[allow(dead_code)]
pub struct Locations {
    pub macos: Option<MacLocation>,
    pub linux: Option<LinuxLocation>,
    pub windows: Option<WindowsLocation>,
}

#[allow(dead_code)]
pub struct MacLocation {
    /// Absolute path to the real executable inside the bundle — not the `.app`
    /// directory, because that is what shows up in `ps` output.
    pub binary: &'static str,
    /// The stock profile, relative to the home directory. Home-relative rather
    /// than rooted at Application Support because not every app puts it there:
    /// Claude does, Codex uses a dotfile directory.
    pub default_profile: &'static str,
}

#[allow(dead_code)]
pub struct LinuxLocation {
    /// Looked up on `PATH`, and matched as a substring of `ps` output.
    pub command: &'static str,
    /// The stock profile, relative to the home directory.
    pub default_profile: &'static str,
    /// Shown verbatim when the command is missing, so the message can tell the
    /// user how to install this particular app.
    pub install_hint: &'static str,
}

#[allow(dead_code)]
pub struct WindowsLocation {
    /// Tried in order; the first that is a file wins.
    pub binaries: &'static [WinPath],
    /// Tried in order; the first that is a directory wins.
    pub default_profiles: &'static [WinPath],
    /// The `Win32_Process` name used to narrow the process query.
    pub process_name: &'static str,
}

#[allow(dead_code)]
pub struct WinPath {
    pub root: WinRoot,
    pub rest: &'static str,
}

#[allow(dead_code)]
pub enum WinRoot {
    /// `%LOCALAPPDATA%`
    Local,
    /// `%APPDATA%`
    Roaming,
}

/// How a profile directory is handed to a launching process, and how it is
/// recovered from one that is already running.
///
/// These are deliberately two separate fields. Writing is cheap on any channel;
/// reading is not. Recovering an argument means parsing `ps` output, which we
/// already do — recovering an environment variable means `KERN_PROCARGS2` on
/// macOS, `/proc/pid/environ` on Linux and `NtQueryInformationProcess` on
/// Windows, three implementations of varying fragility.
///
/// So an app may write through as many channels as it needs, as long as at
/// least one of them is readable. Codex is exactly this case: it needs
/// `--user-data-dir` for Electron's single-instance lock *and* `CODEX_HOME` for
/// its own state, but one readable argument is enough to identify it later.
pub struct Designation {
    pub writes: &'static [Designator],
    pub read_from: Readback,
}

pub enum Designator {
    /// A template containing `{}`, replaced by the profile directory.
    Arg(&'static str),
    /// An environment variable set to the profile directory.
    Env(&'static str),
}

/// Currently one variant by design, not by omission: every app we support is
/// readable from argv. `Env` is the extension point, and adding it is a change
/// to the `Platform` backends — paid once, then available to every app.
pub enum Readback {
    /// An argument prefix, including its `=`.
    Arg(&'static str),
}

/// Where a profile's account identity is recorded, so two profiles signed into
/// the same account can be flagged.
pub struct Identity {
    /// JSON file, relative to the profile directory.
    pub file: &'static str,
    /// Path of object keys down to a string value.
    pub pointer: &'static [&'static str],
}

/// What this app can actually do, so the tray offers only what will work.
pub struct Capabilities {
    /// Whether a running instance can be raised. An app with no window cannot.
    pub focus: bool,
    /// Whether each profile gets its own desktop entry. Linux-only in effect,
    /// and only meaningful for apps that show a window.
    pub desktop_identity: bool,
}

impl AppSpec {
    /// The prefix a profile directory is announced with, for readback.
    pub fn readback_flag(&self) -> &'static str {
        match self.designation.read_from {
            Readback::Arg(flag) => flag,
        }
    }

    /// The arguments that pin this launch to `profile_dir`.
    pub fn launch_args(&self, profile_dir: &std::path::Path) -> Vec<String> {
        self.designation
            .writes
            .iter()
            .filter_map(|designator| match designator {
                Designator::Arg(template) => {
                    Some(template.replace("{}", &profile_dir.display().to_string()))
                }
                Designator::Env(_) => None,
            })
            .collect()
    }

    /// The environment overrides that pin this launch to `profile_dir`.
    pub fn launch_env(&self, profile_dir: &std::path::Path) -> Vec<(&'static str, String)> {
        self.designation
            .writes
            .iter()
            .filter_map(|designator| match designator {
                Designator::Env(name) => Some((*name, profile_dir.display().to_string())),
                Designator::Arg(_) => None,
            })
            .collect()
    }
}

pub static CLAUDE: AppSpec = AppSpec {
    id: "claude",
    label: "Claude",
    product: "Claude Desktop",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/Claude.app/Contents/MacOS/Claude",
            default_profile: "Library/Application Support/Claude",
        }),
        linux: Some(LinuxLocation {
            command: "claude-desktop",
            default_profile: ".config/Claude",
            install_hint: "Install it with: sudo apt install claude-desktop",
        }),
        windows: Some(WindowsLocation {
            binaries: &[
                WinPath {
                    root: WinRoot::Local,
                    rest: r"AnthropicClaude\claude.exe",
                },
                WinPath {
                    root: WinRoot::Local,
                    rest: r"Microsoft\WindowsApps\claude.exe",
                },
            ],
            default_profiles: &[
                WinPath {
                    root: WinRoot::Local,
                    rest: r"Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude",
                },
                WinPath {
                    root: WinRoot::Roaming,
                    rest: "Claude",
                },
            ],
            process_name: "claude.exe",
        }),
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: Some("claude_desktop_config.json"),
    identity: Some(Identity {
        file: "config.json",
        pointer: &["lastKnownAccountUuid"],
    }),
    // Claude Desktop keeps its conversations in Chromium storage it rewrites
    // whether or not anyone is typing, so there is nothing here to observe.
    // Claude *Code* is a separate CLI and is watched through the home
    // directory instead — see `agent_activity::CLI_ROOTS`.
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// ChatGPT and Codex ship as one bundle — `/Applications/ChatGPT.app` carries
/// the identifier `com.openai.codex`. It is Electron and unsandboxed, so it
/// takes `--user-data-dir` like any other, and its own state root is whatever
/// `CODEX_HOME` says (verified against the bundle: `process.env.CODEX_HOME ??
/// join(home, ".codex")`).
///
/// Both channels are required, for different reasons. `--user-data-dir` moves
/// Chromium's data *and* the single-instance lock, without which a second
/// profile exits immediately; `CODEX_HOME` moves the credentials and config the
/// user actually wants separated. Pointing both at one directory keeps a
/// profile a single folder.
///
/// The `codex` CLI reads the same `CODEX_HOME`, so a profile launched here is
/// the same profile on the command line.
pub static CODEX: AppSpec = AppSpec {
    id: "codex",
    label: "ChatGPT",
    product: "the ChatGPT desktop app",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
            default_profile: ".codex",
        }),
        linux: Some(LinuxLocation {
            command: "chatgpt",
            default_profile: ".codex",
            install_hint: "Install the ChatGPT desktop app, or run `codex app` to fetch it.",
        }),
        windows: Some(WindowsLocation {
            binaries: &[WinPath {
                root: WinRoot::Local,
                rest: r"Programs\ChatGPT\ChatGPT.exe",
            }],
            default_profiles: &[WinPath {
                root: WinRoot::Roaming,
                rest: ".codex",
            }],
            process_name: "ChatGPT.exe",
        }),
    },
    designation: Designation {
        writes: &[
            Designator::Arg("--user-data-dir={}"),
            Designator::Env("CODEX_HOME"),
        ],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: Some("config.toml"),
    identity: Some(Identity {
        file: "auth.json",
        pointer: &["tokens", "account_id"],
    }),
    // The one app whose sessions land inside the profile, because `CODEX_HOME`
    // above moves its whole state root there. A profile-launched Codex is
    // therefore observable per profile rather than only through the home
    // directory.
    session_trace: Some("sessions"),
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// The VS Code family below shares one shape: Electron, unsandboxed, a single
/// `--user-data-dir` that moves the whole profile, and no lock that survives
/// the split. Each was confirmed against a real installation by the probe in
/// `probe.rs` — never declared from inspection alone.
///
/// Their ids are deliberately terse. An id is a directory name charged against
/// the socket budget in `paths`, and these apps put a socket inside a profile:
/// declaring VS Code as "visual-studio-code" rather than "code" costs fourteen
/// bytes and is on its own enough to stop the application starting.
pub static CURSOR: AppSpec = AppSpec {
    id: "cursor",
    label: "Cursor",
    product: "Cursor",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/Cursor.app/Contents/MacOS/Cursor",
            default_profile: "Library/Application Support/Cursor",
        }),
        linux: None,
        windows: None,
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: None,
    identity: None,
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// Ships as `Devin.app` but identifies itself as `com.exafunction.windsurf`:
/// the desktop app is Windsurf rebranded, which is why it behaves exactly as
/// Cursor does.
pub static DEVIN: AppSpec = AppSpec {
    id: "devin",
    label: "Devin",
    product: "Devin",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/Devin.app/Contents/MacOS/Devin",
            default_profile: "Library/Application Support/Devin",
        }),
        linux: None,
        windows: None,
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: None,
    identity: None,
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// The app `DEVIN` above is a rebrand of: same fork, same `--user-data-dir`, and
/// the two can be installed side by side, which is the only reason both are
/// declared rather than one.
///
/// NOT probed against a real installation — the machine this was added on had no
/// Windsurf. The shape is inherited from Devin, which is this app under another
/// name, but the bundle path is the one thing a rebrand does change: if it is
/// wrong, Windsurf reads as "not installed" and nothing worse. Run
/// `PROBE_APP=/Applications/Windsurf.app cargo test -- --ignored probe` on a
/// machine that has it and correct the path from the draft it prints.
///
/// Declared as `surf`, not `windsurf`, for the family's terseness reason above
/// and not merely to match it: the full name is eight bytes, which puts the
/// socket path one byte over the macOS limit. `every_app_id_leaves_room_for_a_socket`
/// is what said so.
pub static WINDSURF: AppSpec = AppSpec {
    id: "surf",
    label: "Windsurf",
    product: "Windsurf",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/Windsurf.app/Contents/MacOS/Windsurf",
            default_profile: "Library/Application Support/Windsurf",
        }),
        linux: None,
        windows: None,
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: None,
    identity: None,
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// The bundle name carries a space and brackets, which is what exposed the
/// process parser's assumption that a command is one whitespace-delimited token.
pub static T3_CODE: AppSpec = AppSpec {
    id: "t3code",
    label: "T3 Code",
    product: "T3 Code (Alpha)",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/T3 Code (Alpha).app/Contents/MacOS/T3 Code (Alpha)",
            default_profile: "Library/Application Support/t3code",
        }),
        linux: None,
        windows: None,
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: None,
    identity: None,
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// Declared as `code`, not `visual-studio-code`: see the note above the family.
/// It also names its executable `Code` while calling the bundle Visual Studio
/// Code, so the two are kept apart on purpose.
pub static VS_CODE: AppSpec = AppSpec {
    id: "code",
    label: "VS Code",
    product: "Visual Studio Code",
    locations: Locations {
        macos: Some(MacLocation {
            binary: "/Applications/Visual Studio Code.app/Contents/MacOS/Code",
            default_profile: "Library/Application Support/Code",
        }),
        linux: None,
        windows: None,
    },
    designation: Designation {
        writes: &[Designator::Arg("--user-data-dir={}")],
        read_from: Readback::Arg("--user-data-dir="),
    },
    shared_config: None,
    identity: None,
    session_trace: None,
    capabilities: Capabilities {
        focus: true,
        desktop_identity: true,
    },
};

/// The registry. Adding an app is adding a constant above and a line here.
static ALL: &[&AppSpec] = &[
    &CLAUDE, &CODEX, &CURSOR, &DEVIN, &WINDSURF, &T3_CODE, &VS_CODE,
];

pub fn all() -> &'static [&'static AppSpec] {
    ALL
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn an_app_designated_by_argument_alone_sets_no_environment() {
        let dir = Path::new("/p/work");
        assert_eq!(CLAUDE.launch_args(dir), vec!["--user-data-dir=/p/work"]);
        assert!(CLAUDE.launch_env(dir).is_empty());
    }

    #[test]
    fn an_app_designated_by_both_channels_writes_to_both() {
        let dir = Path::new("/p/work");
        // Chromium's lock moves with the argument...
        assert_eq!(CODEX.launch_args(dir), vec!["--user-data-dir=/p/work"]);
        // ...and the credentials move with the environment.
        assert_eq!(
            CODEX.launch_env(dir),
            vec![("CODEX_HOME", "/p/work".into())]
        );
    }

    #[test]
    fn every_app_is_readable_from_a_channel_it_actually_writes() {
        // The invariant that keeps process scanning possible: an app may write
        // through any number of channels, but the one it is read back from must
        // be among them. A spec that violates this launches fine and is then
        // invisible to every scan — profiles would show as stopped forever.
        for spec in all() {
            let flag = spec.readback_flag();
            let written = spec
                .designation
                .writes
                .iter()
                .any(|designator| match designator {
                    Designator::Arg(template) => template.starts_with(flag),
                    Designator::Env(_) => false,
                });
            assert!(
                written,
                "{} is read back from a flag it never writes",
                spec.id
            );
        }
    }

    #[test]
    fn every_app_id_leaves_room_for_a_socket() {
        // An id is a directory name, and its length is charged against the
        // socket budget in `paths`. Declaring VS Code as "visual-studio-code"
        // rather than "code" costs fourteen bytes and was on its own enough to
        // stop the application starting. Measured against a home directory
        // longer than the author's, since that is not ours to choose.
        for spec in all() {
            let root = std::path::Path::new("/Users/christopher.anderson")
                .join("Library/Application Support/Agent Profiles")
                .join(spec.id);
            let profile = crate::paths::Paths::new(root).profile_dir("9f3c1a7e");
            // Against the macOS number explicitly, not whichever limit this
            // build happens to have: on Windows there is none, and the check
            // would pass by having nothing to check.
            assert!(
                crate::paths::fits_within(&profile, crate::paths::MACOS_SOCKET_PATH_LIMIT),
                "id {:?} leaves no room for a socket: {}",
                spec.id,
                profile.display()
            );
        }
    }

    #[test]
    fn app_ids_are_unique_and_filesystem_safe() {
        // Ids name directories and are parsed out of tray row ids, which are
        // colon-separated — a colon or a slash here would corrupt both.
        let mut seen = std::collections::HashSet::new();
        for spec in all() {
            assert!(seen.insert(spec.id), "duplicate app id {}", spec.id);
            assert!(
                !spec.id.contains(':') && !spec.id.contains('/') && !spec.id.is_empty(),
                "unsafe app id {}",
                spec.id
            );
        }
    }
}
