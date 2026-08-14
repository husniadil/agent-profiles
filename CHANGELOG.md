# Changelog

Notable changes, newest first. This project follows [Semantic Versioning](https://semver.org).

## [Unreleased]

The management window redesigned around what this app actually holds: paths, profile ids, running processes and byte counts. Every existing behaviour is kept; only the presentation changes.

### Added

- **The socket path budget is drawn under the add form**: the directory a new profile would get, a meter, and `89 / 104 bytes`. On a home directory too long to host a profile it says so, in red, before anything has been typed — where previously the refusal only arrived after the profile was submitted. It is a property of the data root, not of the name: a profile directory is named after a generated id, so the number cannot move as you type and a long label is never the reason a profile is refused.
- **A profile can be opened from the management window**, not only from the tray. The row's open action launches the profile, or focuses it if it is already running, deciding from a fresh process scan rather than from what the window last drew.
- **A running dot on every row.** Running state used to be visible only in the tray.

### Changed

- The window is achromatic and follows the system theme, with full light and dark palettes. Colour is reserved for state — green for running, amber for a shared sign-in, red for destructive — so nothing else in the window is coloured. The window previously painted one fixed warm palette regardless of the system.
- Paths, ids, sizes and byte counts are set in a monospace face; the wordmark and section labels in a wide grotesque. Both typefaces are vendored into the bundle rather than fetched, so the window renders the same offline.
- **The three-line page header is one status line**: profile count, running count, total size on disk, and the data root. The window previously carried an eyebrow, a heading and a lede before the first row, then a second heading below that.
- **Profile rows carry their size on disk** in place of the `01 / 02` index column. The index numbered rows in an order that meant nothing and changed when a profile was deleted; the size is the number the delete confirmation already computes.
- **Open, rename and delete are icons**, shown on hover or keyboard focus in the slot the size occupies at rest, so the row's width never changes. The rename and delete confirmations keep their words — an action that destroys 1.4 GB should be read, not recognised from a picture.
- The `Default` badge is gone: that profile is recognisable from being the one with no delete action. The shared-sign-in badge stays, and gains a border so it is findable on an achromatic surface.
- The total size on disk is published only once every profile has been measured. A total that counts half the profiles is a wrong number stated confidently.
- The helper line under the add form is gone with the rest of the prose. Its assurance — that account details stay inside the app itself — is not dropped: it is stated at more length under **Important safety behavior** in the README, which says that labels are manual and that account email addresses are never read from disk or displayed.
- Each profile is measured once per visit to the window rather than on every list reload. Renaming or opening a profile cannot change a byte of it, and both reload the list; re-walking every directory each time made a rename cost seconds of I/O to arrive back at the same numbers.

### Fixed

- Every colour pairing in the window now clears WCAG AA for its role. Several did not: the app name that says whose profiles a group holds read at 2.4:1, and the red that reports an unusable data root read at 4.3:1 against the page.

### Removed

- **The `Quit <profile>` row in the tray menu.** Quitting an application belongs to that application: the row put a destructive action directly beneath the navigational one it looked like, in a menu opened to switch between profiles. Focus and Launch stay, and so does **Quit Agent Profiles**, which ends this app rather than one of the agents it manages. Deletion still refuses while a profile is running, and still says so.

### Note

The rest of the tray menu is unchanged. It is a native platform menu built from plain text rows, so there is no stylesheet to apply; it already follows the system theme by being a system menu, and it already carried the running marker the window's new dot matches.

## [0.2.0] — 2026-08-14

Renamed from **Claude Profiles** to **Agent Profiles**, and generalised from one application to several. ChatGPT, Cursor, Devin, T3 Code and VS Code join Claude, and profiles of different apps run side by side.

### Breaking

- **The application identifier changed** from `com.husniadil.claude-profiles` to `com.husniadil.agent-profiles`, and the data root moved from `Claude Profiles` to `Agent Profiles`, now with a per-app directory below it. A 0.1.0 installation's profiles are **not** picked up automatically: they remain where they were, under the old data root, and can be moved into `Agent Profiles/claude/p/` by hand. Nothing is deleted or rewritten by upgrading.
- **A profile directory is now `<app>/p/<8 characters>`**, not `<app>/profiles/<uuid>`. This is not cosmetic: several supported applications create a Unix domain socket inside the profile directory, and the old layout put an ordinary installation past the system's 104-byte socket path limit. Under it, VS Code never finished starting and the ChatGPT desktop app lost its `ipc/ipc.sock` in silence.
- The macOS app bundle is now `Agent Profiles.app`. The 0.1.0 bundle is a separate application and should be removed after migrating.
- A **Launch at login** entry registered by 0.1.0 points at the old bundle. Turn it off before removing that bundle, or turn it on again here afterwards.

### Added

- Support for the **ChatGPT desktop app** (`com.openai.codex`) alongside Claude Desktop, with profiles for both running at the same time.
- Support for **Cursor**, **Devin**, **T3 Code** and **VS Code**, macOS only — each confirmed against a real installation rather than declared from inspection. An app is declared only for platforms someone has actually checked, and is simply absent elsewhere rather than offered as a row that can only fail.
- A **probe** (`PROBE_APP=… cargo test -- --ignored probe`) that answers the four admission questions against an undeclared application and prints a draft declaration. It launches the app, works out which channels move its profile, checks whether two profiles can live side by side, and runs at a path as long as the real layout so it cannot certify under easier conditions than production imposes.
- Creating a profile is refused when its path would leave no room for the socket applications create inside it, with the numbers in the message.
- Apps are declared as data in `app_spec.rs`. Adding another one is a new declaration and a registry line; no OS backend is touched.
- Tray sections per app, shown only once more than one app is installed — with a single app the menu is the flat list it always was.
- An app picker in the management window, likewise shown only when there is more than one app to choose between.
- A manual verification harness (`cargo test -- --ignored`) that drives real applications: it launches a profile, confirms a process scan attributes it back, confirms the app wrote into the profile directory rather than the stock one, quits it and cleans up.

### Changed

- A profile can now be designated by environment variable as well as by argument, and by several channels at once. ChatGPT needs both: `--user-data-dir` moves Chromium's data and its single-instance lock, `CODEX_HOME` moves the credentials and configuration. Reading back stays argument-only, because recovering an environment variable from a running process is a per-OS problem worth avoiding.
- Because ChatGPT profiles are keyed on `CODEX_HOME`, the `codex` CLI reads the same profile as the desktop app launched from the tray.
- The shared configuration file is per app: `claude_desktop_config.json` for Claude, `config.toml` for ChatGPT.
- One process sweep now covers every app instead of one sweep per app.
- The account warning compares identities **within one app only**. A Claude account uuid and a ChatGPT account id share no namespace, and comparing across apps could only produce a false warning.
- Icons and banner redrawn from committed SVG sources, so they can be regenerated rather than replaced. The generated-favicon font attribution was removed along with the artwork it described.

### Safety

- **Quitting an instance now checks whether it is still there before insisting.** The Windows path waited out a fixed grace period and then sent `taskkill /F` regardless, so an application that closed promptly was force-killed ten seconds later at a process id Windows may already have handed to something else. Both platforms now stop the moment the process is gone.
- **A change to the profile registry is committed before anything irreversible happens.** Deleting a profile removed its directory first and saved the registry second, so a failed write left an entry describing data that no longer existed — while reporting an error suggesting nothing had happened. Adding one failed the other way, leaving a directory behind that no profile owned. Both now undo themselves if the registry cannot be written.

### Fixed

- The socket path budget is applied only where such a limit exists. It is macOS's `sun_path` cap, and enforcing it on Windows — which keeps its named pipes outside the profile directory — refused every profile for anyone whose account name was long enough, citing a limit that means nothing on that system.
- The management window no longer offers an add form when no supported app is installed. It kept whichever app the previous render had listed, so submitting it created a profile for an application that was no longer there to launch it.
- A displaced profile configuration is now saved as `<filename>.replaced`, keeping the whole filename. The previous code replaced the extension, which would have turned `config.toml` into `config.json.replaced` — a file the user would go looking for under the wrong name.
- The process scanner treated the first whitespace-delimited token as the command, so any application whose path contains a space — `/Applications/T3 Code (Alpha).app/…` — was invisible to every scan. An invisible app reads as permanently stopped, which means the guard against two processes sharing one profile never fires. Latent until now, because neither of the first two apps had a space in its path.

## [0.1.0] — 2026-08-13

First release, under the name **Claude Profiles**. Verified on macOS; **Windows and Linux have never been compiled or run on real hardware** — see the platform checklists in the README.

### Added

- Menu bar and system tray app for running multiple Claude Desktop instances in parallel, one profile per account.
- The existing Claude Desktop installation is adopted in place as the **Default** profile, never moved or copied.
- Tray menu showing each profile's live state, with Focus for a running profile and Launch for a stopped one, plus a Quit action per running instance.
- Management window to add, rename, and delete profiles, with the directory size shown before deletion.
- Shared MCP configuration linked into every profile: symbolic links on macOS and Linux, hardlinks on Windows.
- A warning when two profiles appear to be signed in to the same account.
- Opt-in **Launch at login**, off by default and hidden in development builds.
- Per-profile desktop identity on Linux so each profile gets its own taskbar entry.

### Safety

- Claude Desktop has no single-instance lock, so a second process on one user-data directory corrupts its databases. Processes are rescanned immediately before every launch, and a scan that fails **refuses to launch** rather than assuming nothing is running.
- Deletion is refused while that profile is running, and the Default profile can never be deleted.
- A corrupt profile registry is preserved as `profiles.json.corrupt` instead of being overwritten.
- Adopting a profile's existing MCP configuration never overwrites an established shared one; the displaced file is kept alongside the profile.

[Unreleased]: https://github.com/husniadil/agent-profiles/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/husniadil/agent-profiles/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/husniadil/agent-profiles/releases/tag/v0.1.0
