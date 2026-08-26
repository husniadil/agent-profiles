# Changelog

Notable changes, newest first. This project follows [Semantic Versioning](https://semver.org).

## [Unreleased]

### Fixed

- **The security notes now say what the update signature does not cover.** They described a bundle verified against the key baked into the app, which is true, and left the impression that a verified update is therefore the update this project intended. The manifest that names which bundle to fetch is trusted on TLS alone, so the note now says so, and says what someone able to serve that response could and could not do with it.
- **An app that is not installed is now shown greyed with the reason, rather than vanishing.** This is the half of the 0.6.0 note that never shipped, and the correction printed under 0.6.1 no longer applies. Beside a working app, an uninstalled one was filtered out of both the tray and the window, so it was indistinguishable from an app this tool had never heard of. Both surfaces now keep it, at the length each can afford: the tray gives it one disabled row naming the product and saying it is not installed, and the window — which is not as wide as its widest row the way a menu is — says the same thing with the path that was looked at. The screen shown when nothing at all is installed lists those reasons too. Nothing about the row is clickable — there is still nothing to launch — and it returns to a normal list the moment the app is installed, with no relaunch.

## [0.6.1] — 2026-08-21

Six fixes. Two of them stop the app telling you something it never checked, and one hands your Mac back on the way out.

### Fixed

- **A folder the app could not fully measure is no longer reported as if it had been.** A profile directory rewrites and deletes files while its app runs, and 0.6.0 already skipped an entry it could not reach rather than withholding the whole total. What it did not do was say so, and a subdirectory that vanished mid-walk was absorbed silently — a total short by an arbitrary amount, presented as the answer. The walk now counts what it had to skip, and a figure that is known to be short is marked `≥` on the row and in the status line, reads "at least X on disk" to a screen reader, and says so in the delete confirmation, which is the last sentence anyone reads before a folder stops existing.
- **A delete that fails no longer removes the profile from the registry anyway.** The registry was written before the directory was removed, which is the right order, but a failed removal left the entry gone while the directory — credentials and all — stayed on disk, owned by nothing and cleaned up by nobody. The registry is rolled back now, so what you see and what is on disk agree again. The error also stops claiming the data is intact: `remove_dir_all` deletes as it walks and stops at the first thing it cannot delete, so a failure often means part of the profile is already gone, and the message says that instead of guessing.
- **The registry is written atomically.** It was a plain truncate-and-write, which meant a disk that filled up mid-write left `profiles.json` empty or half-finished, and the next launch read that as corruption and fell back to a single profile. It is written aside and renamed into place now, so the live file only ever moves from one complete registry to another.
- **Quitting hands the machine back.** Nothing ran on the way out, so quitting while Keep Awake held the machine never released the hold. macOS and Linux survived that by accident of how they hold; Windows did not, because its hold is a power-scheme write that outlives the process — so a quit left the lid-close action on "do nothing" until the next launch happened to notice. The release now runs on quit, on a Windows session end (a restart or an update reboot, which never reaches the ordinary exit path), and before an update installs itself. The trigger you set is left alone: quitting is not switching Keep Awake off.
- **The "held" clock stops counting while a hold is paused.** A pause on low battery or heat released the machine but kept adding to the figure beside it, so the window could report "held 1h 10m" for a machine that had been free to sleep for most of it. The figure freezes instead, and resumes from where it stopped rather than restarting.
- **The security notes no longer promise something the app stopped doing in 0.5.0.** `SECURITY.md` said this app makes no network requests of its own. That has been false since auto-update shipped: it is on by default and checks GitHub once per launch, and if a release is advertised it downloads, installs and relaunches without asking. The note now says so, and says where the update's signature actually comes from.

### Corrections to the 0.6.0 notes

- The 0.6.0 entry said an app that is not installed **"appears greyed with the reason, rather than vanishing"**. Half of that shipped and half did not. An uninstalled app is now resolvable — it is listed and counted when nothing at all is installed, and it becomes usable the moment you install it, with no relaunch. But no greyed row with a reason is rendered anywhere: alongside a working app, an uninstalled one is still filtered out of both the tray and the window. Whether to build that row or to keep the current behaviour and say so plainly is open in [#36](https://github.com/husniadil/agent-profiles/issues/36).

## [0.6.0] — 2026-08-20

Five fixes, two of which stop a readable registry being set aside and the profiles it named going quiet.

### Fixed

- **A profile registry that cannot be read is no longer mistaken for a corrupt one.** A `profiles.json` that was merely locked, on a disk that hiccuped, behind a descriptor limit, or restored with the wrong owner was moved aside and replaced with a registry holding one profile — silently, while the real profile directories stayed on disk with no way back to them through the app. Only a file whose contents cannot be parsed is treated as corrupt now. A registry that could not be read leaves the file untouched, and the app it belongs to says so instead of offering a fresh start nobody asked for.
- **A preserved registry is no longer destroyed by the next one.** `profiles.json.corrupt` was a single fixed name, so a second corruption overwrote the first copy — which is the one worth keeping, since by then the app has already rewritten the registry down to whatever it could still see. Later copies are numbered instead.
- **The "on disk" total no longer disappears because one file did.** A live profile directory rewrites and deletes files while its app runs, and a single entry vanishing mid-measurement withheld the total for every profile for the whole visit. An entry that cannot be reached is skipped now, the way `du` has always done it.
- **Windows: an app that is not installed appears greyed with the reason, rather than vanishing.** It also becomes usable the moment it is installed, with no relaunch — the behaviour macOS already had.
- **Translated labels are no longer clipped mid-word.** Two controls were sized in pixels to their English label, and five of the six shipped languages had at least one label that did not fit; the overflow wrapped inside a fixed height and was cut off. In Spanish the delete confirmation lost its verb. Both controls now size themselves to the longest label in whatever language is loaded.

### Security

- **Every GitHub Actions workflow step is pinned to a commit SHA**, and Dependabot opens the version bumps weekly so the pins move deliberately instead of freezing. The release job holds the updater's signing key, and a mutable tag means the code in that job can change without anything in this repository changing — which is how a compromised action would sign an update the app then installs on its own. Pinning is what closes that door.

## [0.5.0] — 2026-08-19

A General tab, carrying a silent updater and six languages for the window and the tray.

### Added

- **Silent auto-update from this project's own GitHub releases.** Once per launch, and on demand from a **Check now** button, the app checks, downloads, installs and relaunches with no dialog in between — the setting this project needs, since a tray app nobody opens is exactly the app that stops getting fixes. Off is a real off: no request to GitHub is made at any point, not a check whose result is ignored. The downloaded bundle is verified against the minisign signature the update manifest carries for it, entirely separate from OS code signing — the release bundles stay unsigned, and installing one still needs the same right-click-Open or SmartScreen bypass described under **Installing a release build**.
- **A six-language interface** — English, Bahasa Indonesia, 日本語, Deutsch, Español and Português — covering the window and the tray menu together, since a picker that only translates one of them would leave the other looking like it forgot. The default follows the operating system's own language and falls back to English for anything else; an explicit choice overrides it and is remembered across restarts. Switching takes effect immediately, with no restart, in both surfaces at once.

### Changed

- **Start at login moved into the General tab**, alongside the new update and language rows, rather than sitting on its own beside the profile list. All three are settings about how the app itself behaves rather than about a profile, and now live together.
- Every visible string in the window now comes from a typed dictionary rather than being written inline in JSX; a locale file missing a key fails `pnpm build`, so a translation cannot ship incomplete.

### Fixed

- **Windows: one uninstalled app no longer stops the whole app from starting.** A declared app whose data directory was absent aborted startup and took every other app down with it; it is now skipped. It is skipped entirely rather than listed as unavailable, which is tracked in [#11](https://github.com/husniadil/agent-profiles/issues/11).

## [0.4.0] — 2026-08-17

An agent can now finish its work with the lid shut. Off by default, and it asks for nothing until you turn it on.

### Added

- **Keep awake with the lid closed, on all three platforms.** A long-running agent used to die the moment the lid shut. Each system hides the lever somewhere different, and none of the portable ones work: `caffeinate` and the `IOPMAssertion` APIs behind it prevent *idle* sleep only, and `SetThreadExecutionState` is the same story on Windows. Agent Profiles now holds the lever that does work on each — the system-wide `pmset disablesleep` flag on macOS, a logind inhibitor on Linux, the power scheme's lid-close action on Windows — for as long as an agent is working, and gives it back when it stops. Off by default; it asks for nothing until you turn it on.
- **A *Keep Awake* tab in the management window**, alongside the profile list, carrying the trigger, the limits, and an honest readout of what is being held and why it is not.
- **Detection reads agent session transcripts rather than CPU.** Claude Code and Codex append to a session file on every message and every tool result, so a transcript that moved recently is proof an agent is working — including through a long network wait, which is exactly when the process looks idle and a CPU heuristic reports "finished". Measured against a live session: the transcript was two seconds stale mid-task. The tab lists the folders being watched and how fresh each one is, so a trigger that did not fire is something you can look at.
- **Two guards.** A battery floor, configurable, that drops the hold even with an agent still working. And a thermal guard, on by default, that releases while the machine reports itself overheating — the case a closed laptop in a bag actually runs into, where nothing can be reported to you and the charge level is not the problem. The battery reading comes from `pmset`, `/sys/class/power_supply` and `GetSystemPowerStatus`; the thermal reading is Apple's own four-level `thermalState` on macOS and the kernel's thermal zones, banded to match, on Linux.
- **Recovery from a run that died holding the setting.** `disablesleep` survives a reboot, so a panic could otherwise leave a Mac that never sleeps again — and both of the obvious safeguards, writing only on a change and restoring only what you took, independently decide to leave it alone in exactly that case. The privileged helper now writes down who owned the setting *before* it can hold anything, and the next launch reclaims it and offers a one-click repair. Windows records the lid action the same way and puts it back by itself at the next launch; Linux needs neither, because logind drops an inhibitor when the process holding it goes.

### Changed

- `AppSpec` gained `session_trace`, naming where an app's agent sessions land inside a profile directory. Only Codex has one, because only Codex has its state root relocated into the profile by `CODEX_HOME`.
- `Platform` gained `can_hold_awake`, `needs_authorization`, `can_read_thermal`, `power`, `thermal`, `hold`, `recover_hold`, `start_awake_watchdog` and `restore_sleep`, all defaulted, so `commands.rs` carries no new `cfg` branches. Every one of them is answered per machine rather than per operating system: a Linux box with no logind cannot hold, and one with no thermal zones cannot read a temperature, and the window is told which rather than being told about the platform.
- **Only macOS asks for a password.** Linux takes its inhibitor as the signed-in user and Windows writes a power scheme that user already owns, so both skip the authorization step entirely rather than offering a button that grants something never withheld.
- **A guard that cannot fire is left out of the window, not greyed out.** Windows publishes no thermal state to user space — the ACPI class needs administrator rights and most consumer firmware does not implement it — so the switch is absent there. A disabled switch reads as *turn something on first*; one that says "this never applies" is still a switch someone can leave on and believe they are protected.
- **The app icon no longer draws its own rounded square.** macOS 26 masks every app icon to the system squircle and paints a light plate behind it, so our smaller rounded square on transparency showed the plate through as a white rim around the orange. The background is now a full-bleed square and the platform does the rounding. This is an improvement everywhere rather than a macOS fix with a cost elsewhere: the old artwork's corners were opaque white, not transparent, so every platform was already shipping a white box behind the rounded square. Windows and Linux mask nothing, so they now get a hard-cornered orange square — the honest shape of the drawing, and the trade we are taking for a correct icon on the platform that shapes it for us.
- The app now runs one background thread. It is the first timer in the codebase, and it is what makes the guards reachable: every other scan here is demand-driven, and the lid-closed case is precisely the case where nobody opens the tray and nobody renders the window.

### Security

- The elevated loop is passed inline to `osascript` and never written to disk. Anything under Application Support is user-writable, so an elevated script on disk would be a persistent root escalation for every process running as you rather than a power-management feature.
- The loop tests the flag file for *existence* only and never reads it, and a data root containing a quote, a backslash or a newline is refused outright rather than escaped through three layers of quoting.
- The helper identifies the app by pid *and* process start time. Pids are recycled, and a loop that outlived its app would otherwise keep sleep disabled on behalf of a stranger.
- Windows records the lid action it displaced in a file in your own folder, and refuses to restore any value above 3 out of it. What comes off that file is written straight into a power scheme, and putting the setting back must not be a way to arm a lid that shuts the machine down.
- The Linux inhibitor is held by a command reading a pipe, not by a sleeping process. Rust never kills a child on drop and a process reparented to init outlives whatever spawned it, so a `kill -9` or a panic would otherwise leave the lid-switch lock held until someone found the process by hand. A pipe cannot be leaked: however this app ends, the kernel closes the write end and the lock goes with it.

## [0.3.1] — 2026-08-16

The app icon redrawn. Artwork only; no behaviour changes.

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
