<p align="center">
  <img src="assets/banner.png" alt="Agent Profiles — run your coding agents side by side, one profile each" width="100%">
</p>

<p align="center">
  <a href="https://github.com/husniadil/agent-profiles/actions/workflows/ci.yml"><img src="https://github.com/husniadil/agent-profiles/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT licence"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg" alt="Platforms">
</p>

# Agent Profiles

> **Unofficial.** This is a third-party tool with no affiliation to, endorsement by, or support from Anthropic or OpenAI. "Claude" and "Claude Desktop" are trademarks of Anthropic; "ChatGPT" and "Codex" are trademarks of OpenAI. This project only launches the applications you already installed, pointed at a different profile directory.

Agent Profiles is a menu bar and system tray app for running several accounts of a coding-agent desktop app in parallel, one profile each. Every profile gets its own permanently separate directory, so using one account never requires signing out of another — and profiles of different apps can run at the same time.

## Supported apps

| App | Profile is selected by | Shared file | Platforms | Notes |
| --- | --- | --- | --- | --- |
| **Claude** (Claude Desktop) | `--user-data-dir` | `claude_desktop_config.json` | macOS, Windows, Linux | |
| **ChatGPT** (bundle id `com.openai.codex`) | `--user-data-dir` **and** `CODEX_HOME` | `config.toml` | macOS, Windows, Linux | the `codex` CLI reads the same `CODEX_HOME` |
| **Cursor** | `--user-data-dir` | — | macOS | |
| **Devin** | `--user-data-dir` | — | macOS | ships as Devin, identifies as `com.exafunction.windsurf` |
| **T3 Code** | `--user-data-dir` | — | macOS | |
| **VS Code** | `--user-data-dir` | — | macOS | |

Every one of these was confirmed against a real installation by the probe described in [CONTRIBUTING](CONTRIBUTING.md#adding-another-app), never declared from inspection alone.

**Platforms** lists where an app has actually been checked, and the ledger behind that column is [docs/platform-status.md](docs/platform-status.md) — Windows and Linux compile in CI and have never been run on real hardware. Where an app has not been checked on a platform it is simply absent — no tray section, no directory, no error row — because a user has no way of knowing this build was never tried on their system, and a row that can only fail is worse than no row. An app that is checked here but not installed is the opposite case: it is named, greyed, and says it is not installed, because someone who knows they have it needs to be able to tell "not installed" from "this tool forgot about it". It contributes no profiles and nothing to click.

`Shared file` is empty where no file has an obvious claim to being shared between an app's profiles. For the VS Code family `User/settings.json` is a plausible candidate, but that is a product decision rather than something a probe can establish, so nothing is shared until someone decides.

## The Default profile

The **Default** profile is the installation that already exists on the machine — `~/Library/Application Support/Claude` for Claude, `~/.codex` for ChatGPT. Agent Profiles uses it in place: it never moves or copies the directory, and launches it with **no designation at all**, neither argument nor environment variable. Anything else would make it a different profile and orphan everything already there. Additional profiles live below the Agent Profiles data root and get their own directories.

## Important safety behavior

Claude Desktop provides no single-instance lock for a user-data directory, and starting two processes against the same directory can corrupt its databases. ChatGPT does hold one, but a duplicate exits silently, which to a user looks like a launch that did nothing.

Agent Profiles therefore rescans processes immediately before every launch. A profile that is already running gets a Focus action instead of a second launch, and an unreadable process scan **fails closed** — refusing costs one retry, guessing costs a profile.

Profile labels are manual. Account email addresses are never read from disk or displayed. Each app's account identifier — `lastKnownAccountUuid` for Claude, `tokens.account_id` for ChatGPT — is used only to warn that two profiles appear to be signed in to the same account, and only ever compared **within one app**: those two values share no namespace, so comparing across apps could produce nothing but a false warning.

## Shared configuration

Each app's shared configuration file — `claude_desktop_config.json` for Claude, `config.toml` for ChatGPT — is shared across that app's profiles, so editing it in one profile changes it for all of them. If a profile already has its own copy and a shared one exists, that copy is moved aside to `<filename>.replaced` rather than silently overwriting the configuration every other profile is using. [How the linking works](docs/design.md#shared-configuration).

## Launch at login

The management window offers an opt-in **Launch at login** toggle. It is off until you turn it on, and it starts only the tray: no profile is opened for you.

The operating system owns this setting — a login item on macOS, a registry entry on Windows, an autostart desktop entry on Linux. Agent Profiles keeps no copy of it and reads the real value each time the window opens, so turning it off in your system settings is reflected here rather than contradicted.

The toggle is hidden in development builds. A login item registered from `pnpm tauri dev` would point at a `target/debug` binary that moves, gets rebuilt, and disappears on `cargo clean`, leaving an entry that fails silently at every boot.

## Keep awake with the lid closed

A long-running agent dies when you close the lid: macOS forces sleep on lid close unless the machine has both external power and an external display, and `caffeinate` cannot override that — it prevents *idle* sleep only. The one lever that works is the system-wide `pmset disablesleep` flag, which needs root.

Agent Profiles holds that flag while an agent is working and gives it back when it stops. It is **off by default** and asks for nothing until you turn it on, in the management window's **Keep Awake** tab.

**How it works.** When you click Authorize, the app asks for an administrator password **once on this Mac** — not once per launch. What that password buys is a `sudoers` drop-in at `/etc/sudoers.d/agent-profiles`, granting exactly two commands and nothing else: `pmset -a disablesleep 1` and `pmset -a disablesleep 0`. Every later launch finds the grant and holds the flag with no prompt at all.

The helper itself is an ordinary user process, not root. It watches two things: a flag file, and this app's process. Flag there, sleep is disabled; flag gone, sleep is restored. The loop exits when Agent Profiles does, so a crash or a force-quit cannot leave your Mac permanently unable to sleep.

**Why a `sudoers` grant and not a helper tool.** Apple's supported route for "authorize once" — `SMAppService`, or the older `SMJobBless` — validates the app's code signature at registration, and these builds are deliberately unsigned. A grant pinned to two exact commands is the narrower alternative: `sudoers` matches the full argument vector, so it cannot be spent on anything but turning that one setting on and off, and `/usr/bin/pmset` is `root:wheel` and SIP-protected, so it cannot be replaced with something else to run. The grant outlives the app — **deleting Agent Profiles does not remove it**. The Keep Awake tab has a **Give it back** button, and prints the one command that does the same thing: `sudo rm /etc/sudoers.d/agent-profiles`.

**What arms it.** Either *when an agent is working* — Claude Code and Codex append to a session transcript on every message and every tool result, so a transcript that moved recently means an agent is working, even while it sits at zero CPU waiting on the network — or *always while Agent Profiles runs*, which is the option for agents inside a desktop app, where there is nothing to observe. The tab lists the session folders being watched and how fresh each one is, so a trigger that did not fire is something you can look at rather than guess about.

**What releases it.** The agent finishing, the battery falling below your threshold, the hold running past your time limit, or the app quitting. With the lid shut nothing can be reported to you, so the time limit is deliberately conservative rather than generous.

**If a run ends badly.** `disablesleep` is persistent and survives a reboot, so a kernel panic or a power cut could otherwise leave a Mac that never sleeps again. The helper writes down who owned the setting before it can hold anything, and the next launch reads that note, reclaims the setting and offers a one-click **Restore sleep** — which costs nothing once the grant is in place. If the grant was never given, or you took it back, the tab also prints the command: `sudo pmset -a disablesleep 0`.

**What it does not do.** It disables *all* sleep while held, not only lid-close sleep — the Sleep menu item and low-power auto-sleep are blocked too. It does nothing on Windows or Linux, where the lid action is a system power-plan setting with no user-space override.

**One thing worth knowing.** The flag file lives in your own Application Support folder and is writable by anything running as you, so any process of yours could pin the machine awake. This is not a privilege boundary — Agent Profiles runs as you — and the worst it costs is a flat battery, not root.

## Wake and open an app on a schedule

The **Schedule** tab wakes a sleeping Mac and opens a chosen application at each day's own time — Slack, a browser, anything in `/Applications`. It is not scoped to agent profiles: this is a general "wake the machine and open something" tool, not a way to launch a profile specifically (profiles already have their own launch path elsewhere in the app).

Turn on the master switch, pick which weekdays are active and what time each one wakes at, and choose the app to open. It works whether the Mac is on AC power or battery, and needs you logged in — a locked screen still counts, only actually being logged out doesn't. A fully shut-down Mac stays off; a closed lid needs an external display for the app to actually open.

Per-day times mean this cannot use macOS's built-in repeating wake, which only holds one time for all its days — instead it schedules a rolling batch of one-off wakes a few weeks ahead and tops the batch up automatically. The tab shows how many days of wakes are currently armed, and if you go long enough without opening Agent Profiles, that number reaches zero and wakes stop until you open it again.

## Updating

Agent Profiles updates itself silently by default: once per launch it checks this repository's own GitHub releases, and if a newer one exists it downloads, installs and relaunches with no dialog in between. The switch — **Update automatically** — lives in the General tab, alongside the version currently installed, a **Check now** button for running the same check on demand, and a status line reporting exactly what it is doing. Turning it off is a real off: no request to GitHub is made at any point, not a check whose result is discarded.

The manifest it reads (`latest.json`) is attached to each GitHub release next to the installers, and each artifact carries a minisign signature the plugin verifies before installing. That signature is **separate from OS code signing** — it proves the update file came from this project's release process, not that the binary is signed for macOS Gatekeeper or Windows SmartScreen. The bundles are still unsigned, so the "damaged"/SmartScreen warnings described under [Installing a release build](#installing-a-release-build) are unchanged by any of this; they apply to the first install and to any manual download the same as before.

## Languages

The window and the tray menu are both translated into six languages: **English**, **Bahasa Indonesia**, **日本語**, **Deutsch**, **Español** and **Português**. The picker is the **Language** row in the General tab, and its detail line says so directly — the choice is not scoped to the window alone.

The default is **Same as system**, which reads the operating system's language once at startup and falls back to English for anything not in the six above. Picking a language explicitly overrides that and is remembered across restarts; picking **Same as system** again hands the decision back to the OS. Switching takes effect immediately, in both the window and the tray menu, with no restart.

## Installing a release build

Releases are **unsigned**, because code-signing certificates cost money this project does not have. The operating system will therefore object, and the objection is misleading in both cases:

- **macOS** claims the app "is damaged and can't be opened". It is not damaged; it is merely unsigned. Right-click the app and choose **Open**, then confirm. If macOS still refuses, clear the quarantine flag: `xattr -d com.apple.quarantine "/Applications/Agent Profiles.app"`
- **Windows** shows a SmartScreen warning about an unknown publisher. Choose **More info → Run anyway**.

Only do this for a build you obtained from this project's Releases page. If either warning appears for a download from anywhere else, it deserves your suspicion.

## Contributing

[CONTRIBUTING](CONTRIBUTING.md) covers building the app, adding another supported app, and adding a UI string. [docs/design.md](docs/design.md) explains why the app is shaped the way it is, [docs/platform-status.md](docs/platform-status.md) is the acceptance ledger, and [docs/releasing.md](docs/releasing.md) is the release policy.

## License

MIT — see [LICENSE](LICENSE).
