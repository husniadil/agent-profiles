# Changelog

Notable changes, newest first. This project follows [Semantic Versioning](https://semver.org).

## [Unreleased]

### Changed

- **The app icon is the mark the menu bar already draws** — two stacked profile cards carrying the AI sparkle — in place of the `ap` lettermark. The dock, the taskbar and the menu bar showed two different marks for the same app, and the letters were initials of a name the product no longer leads with. The cards are outlines rather than solids, since a solid card on a solid card is one orange blob with a seam, and the seam is the first thing to go when the icon is scaled down. It reaches every surface the old mark did: the app icons on macOS, Windows, Linux, iOS and Android, the web favicons, the window icon and the README banner.
- **The 16 and 32 pixel renderings are drawn rather than downscaled.** At that size the full icon's stroke lands on eight tenths of a pixel and the two cards blur toward one shape, so those sizes come from a second source that redraws the same mark for the pixel grid — heavier stroke, larger cards, and the gap between them widened past half a stroke so it survives as a gap. Nothing is dropped from the mark. Every consumer of the small sizes is fed from it, including the `16`, `24` and `32` entries inside the Windows `.ico` and the 16 pixel entries in the macOS `.icns`, which the icon generator would otherwise fill from the full drawing.

## [0.3.0] — 2026-08-15

The management window rebuilt. Every existing behaviour is kept; only the presentation changes.

### Added

- **The window is a React app** built with [beUI](https://beui.dev) components, replacing the hand-written DOM builder in `src/main.ts` that had grown to 394 lines of element creation and manual event wiring. The behaviour it drew is unchanged; what it is made of is not.
- **A profile has an identity colour** — a stable hue derived from the profile, shown as a chip carrying its initial — so a profile is recognisable at a glance rather than only by reading its name. The hue follows the profile, not its position in the list.
- **The maximize button is disabled.** This is a fixed-purpose window opened from the tray for a few seconds; a zoomed full-screen state serves nothing. It still resizes freely. On macOS the green title-bar button greys out; on Linux Tauri does not support the setting, so the window can still be maximized there.
- **The socket path budget appears only when the margin is thin** — under eight bytes of headroom, or already over. The refusal does not live in that block: `profile_store::add` turns down a profile that leaves no room whether or not anything was drawn, so on a machine that will never reach the limit the meter was a number nobody could act on, held permanently in a window with no room to spare. It cannot move as you type either — a profile directory is named after a generated id, so for one machine it is a fixed verdict rather than a gauge. Eight bytes is the spread between app ids plus margin: a data root that clears `code` can still refuse `claude`. An ordinary home directory sits about sixteen bytes clear.
- **A profile can be opened from the management window**, not only from the tray. The row's open action launches the profile, or focuses it if it is already running, deciding from a fresh process scan rather than from what the window last drew.
- **A running dot on every row.** Running state used to be visible only in the tray.
- **Windsurf is a supported app** — the Devin rebrand, declared alongside the agents already handled.

### Changed

- **The window sizes itself to its content** instead of holding a fixed height with the list stretched to fill it. A tray window is a popover, not a panel: three profiles no longer sit in a frame built for nine, and the empty band that used to open below a short list is gone. Past the point where the list would run off the screen it scrolls inside the window instead of growing it. Measured from the frontend — the gap between the list's natural height and the height the window currently gives it — and applied through the Tauri window API, so it stays a property of what is on screen rather than a count kept in the backend. macOS/Linux; the resize is a no-op where the platform gives no window to size.
- **The add button is `Add`, without a trailing plus.** The plus sat after the word and only said it twice; the button is narrower for losing it, and its width is now pinned to its widest running state (`Adding`) so the label swapping through Add → Adding → Added → Retry never resizes it under the pointer.
- **A new profile's name is capped at 15 characters.** The name is a label, not a path — the profile directory is named after a generated id — so its length costs nothing but row space, and fifteen sits on one line beside the running badge without truncating.
- The window is achromatic and follows the system theme, with full light and dark palettes. Colour is spent on two things only: **identity** — a profile's own hue, on its chip — and **state**, green for running, amber for a shared sign-in, red for destructive. Nothing else in the window is coloured. It previously painted one fixed warm palette regardless of the system.
- **The window is set in the system face** on a five-step type scale — `title`, `body`, `callout`, `sub`, `caption` — and the vendored Archivo and IBM Plex Mono files are gone from the bundle. A window that belongs to the menu bar should read as part of the system it sits in, and the fonts it was carrying were weight the download did not need to include.
- **The three-line page header is one status line**: profile count, running count, total size on disk, and the data root. The window previously carried an eyebrow, a heading and a lede before the first row, then a second heading below that.
- **Profile rows carry their size on disk** in place of the `01 / 02` index column. The index numbered rows in an order that meant nothing and changed when a profile was deleted; the size is the number the delete confirmation already computes.
- **Open, rename and delete are icons**, shown on hover or keyboard focus in the slot the size occupies at rest, so the row's width never changes. The rename and delete confirmations keep their words — an action that destroys 1.4 GB should be read, not recognised from a picture.
- **Section headings are sentence case at the subheadline size**, the way the Finder sidebar and System Settings set a group heading — `Claude`, `VS Code`, `New profile`. They were small caps on a wide track, which is a dashboard's idea of a section label rather than this platform's. The typeface never changed: it has been the system face throughout.
- **The Default profile shows a disabled delete rather than an empty slot.** An empty slot reads as an icon that failed to load; a greyed one reads as an action this row does not have. The label says the profile is the app's own installation and cannot be deleted, so the greyed control promises no condition under which it could be.
- The `Default` badge is gone: that profile is recognisable from being the one with no delete action. The shared-sign-in badge stays, and gains a border so it is findable on an achromatic surface.
- The total size on disk is published only once every profile has been measured. A total that counts half the profiles is a wrong number stated confidently.
- The helper line under the add form is gone with the rest of the prose. Its assurance — that account details stay inside the app itself — is not dropped: it is stated at more length under **Important safety behavior** in the README, which says that labels are manual and that account email addresses are never read from disk or displayed.
- **A profile's state is the row's own icon rather than a `●` inside its label.** Every profile row now carries an image — a filled green disc while it is running, a grey ring while it is not — in the column the Wi-Fi menu puts a network's icon in. It is drawn from the window's `--live` token, so running is the same green in both surfaces. The old `●` and `○` sat in the text column, aligned against nothing.
- **Profile names are set a step below the menu's own type size** — 12pt against AppKit's 14pt — so a person with nine profiles gets a shorter menu out of it. `Settings…` and `Quit` stay at full size: they are commands rather than data, and the contrast is what makes the smaller rows read as a decision rather than as a menu that came out wrong. macOS only. muda's menu item takes a plain `String` and has nowhere to put a font, so the size is set through AppKit itself, down through the tray's `NSStatusItem` to the `NSMenu` it owns; Windows and Linux offer no equivalent and are unchanged.
- **The path under a profile's name is set at `text-sub`**, the same step the New profile card sets its own path in, one below the name beside it — and in one quiet grey rather than ending in full-strength ink, with the last segment held apart by weight instead. The path is where a profile lives, not what it is called; the name is what the eye should land on. It stays in the monospace face every path and id here is set in.
- **Profiles are no longer padded with three spaces under an app's name.** Their icon indents them, and the app name above sits flush left without one, the way `Known Network` sits above the networks under it. Every profile stays on the top level: this is a menu opened to reach a profile, and a profile behind a submenu is one hover further away than it was.
- A profile that is running but cannot be focused keeps its filled dot while staying unclickable, which reads as *running, nothing to do here* rather than as a row that failed.
- **The row that opens the window is `Settings…` rather than `Manage Profiles…`.** It opens the same window and does the same thing; `Settings…` is what every other menu bar app calls the row that opens its window, and it no longer repeats the word the rows above it are already full of.
- **The last row is `Quit` rather than `Quit Agent Profiles`.** It still ends this app and never one of the agents it manages.
- **The menu bar icon is two stacked profile cards bearing the AI sparkle**, rather than the letters `ap`. The cards carry "profiles" — the app runs several at once — and on the front one sits the concave four-point sparkle that reads as AI across the whole field now (Gemini, Copilot, ChatGPT, Slack), which the old wordmark said nothing of. It is a template image: only the alpha channel survives and the system recolours it, so the cards are outlines rather than fills — a solid card would arrive as a black block among a menu bar full of line-weight icons — and the card behind is cut away around the front so the two do not merge into one shape at menu-bar size.
- Each profile is measured once per visit to the window rather than on every list reload. Renaming or opening a profile cannot change a byte of it, and both reload the list; re-walking every directory each time made a rename cost seconds of I/O to arrive back at the same numbers.

### Fixed

- **`cn()` was deleting this window's type sizes before they reached the DOM.** The window styles with Tailwind and merges class lists through `tailwind-merge`, which only knows Tailwind's own class names, and `text-<name>` is a colour in stock Tailwind, so `text-body`, `text-sub` and `text-caption` were all classified as colours: any class list naming a size *and* a colour lost the size, and the element silently inherited `body`'s 13px instead. `extendTailwindMerge` now names the scale as a `font-size` group. One element was affected — the path under a profile's name, the only `cn()` call combining the two — and it had been rendering three sizes larger than the code said for as long as the class existed.
- Every colour pairing in the window now clears WCAG AA for its role. Several did not: the app name that says whose profiles a group holds read at 2.4:1, and the red that reports an unusable data root read at 4.3:1 against the page.

### Removed

- **The `Quit <profile>` row in the tray menu.** Quitting an application belongs to that application: the row put a destructive action directly beneath the navigational one it looked like, in a menu opened to switch between profiles. Focus and Launch stay, and so does **Quit** (see **Changed**), which ends this app rather than one of the agents it manages. Deletion still refuses while a profile is running, and still says so.

### Note

The tray menu has no stylesheet to apply — it is a native platform menu, and it follows the system theme by being a system menu. Its rounded hover highlight, its state column and its section spacing are all drawn by the operating system; what changed above is what the menu asks for, not how any of it is painted.

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
