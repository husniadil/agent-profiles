# Design

Why Agent Profiles is shaped the way it is. [The README](../README.md) covers
installing it and [the manual](manual.md) covers what each part does and where.
This file is for anyone reading the code or deciding whether a change is safe.
[Platform status](platform-status.md) records what has run on real hardware.

## The Default profile is used in place

The Default profile is the installation already on the machine. Agent Profiles
never moves or copies it. It launches it with no designation at all: no
argument and no environment variable.

### Why

The Default profile is defined as the directory an app uses when nothing
designates one. Designating it would make it a different profile and orphan
everything the user already has. An app designated by environment would still
be moved off its stock directory by a stray variable, so both channels stay
empty.

## A profile is pinned through two channels

Each app answers two questions:

- Writing. How a launching process is told which profile to use. An argument,
  an environment variable, or both.
- Reading back. How a running process is later matched to its profile.

An app may write through as many channels as it needs, as long as one of them
can be read back. ChatGPT writes `--user-data-dir` and `CODEX_HOME`, both
pointing at the same directory. The argument moves Chromium's data and its
single-instance lock. `CODEX_HOME` moves the credentials and configuration. A
profile stays one folder.

### Why

Writing is cheap on any channel. Reading is not. An argument comes back from
the process table. An environment variable needs `KERN_PROCARGS2` on macOS,
`/proc/pid/environ` on Linux and `NtQueryInformationProcess` on Windows. So
the scan reads back the argument alone.

## Every launch rescans and fails closed

Right before a spawn the app scans processes again. A profile that is already
running is refused with a message to focus it. The tray offers Focus in place
of Launch for a live profile. A scan that fails refuses the launch.

### Why

Claude Desktop holds no single-instance lock on its user-data directory. Two
processes on one profile both stay alive and corrupt its databases. ChatGPT
does hold a lock, but a duplicate exits in silence, which looks like a launch
that did nothing. The tray menu may be seconds old, so the check runs again
closest to the spawn.

A failed scan cannot tell "nothing is running" from "I cannot tell". Treating
it as nothing running would launch straight into the corruption the check
exists to prevent. Refusing costs one retry. Guessing can cost a profile.

## Account identifiers are compared within one app, and emails are never read

Labels are typed by the user. The app reads one account identifier per app:
`lastKnownAccountUuid` for Claude and `tokens.account_id` for ChatGPT. It uses
it only to warn that two profiles of one app seem signed in to the same
account. Any failure to read it gives no warning.

### Why

A Claude account uuid and a ChatGPT account id share no namespace. A
comparison across apps could only produce a false warning. The warning is
cosmetic, so an unreadable or oddly shaped file means "cannot tell" and
nothing more. Email addresses are not needed for the warning, so they are not
read or shown.

## Profile paths are short

A profile directory is `<data root>/<app id>/p/<8 characters>`. App ids are
terse for the same reason. Creating a profile whose path leaves no room for a
socket is refused, with the numbers in the message. Windows has no such limit
and refuses nothing.

### Why

Several apps create a Unix domain socket inside the profile directory. VS Code
writes `<version>-main.sock` and ChatGPT writes `ipc/ipc.sock`. macOS caps a
socket path at 104 bytes and Linux at 108. That budget is shared by the data
root, the app id, the profile id and the user's home directory, which is not
ours to choose. The check budgets for `/1.13-main.sock`, the longest socket
name seen.

The numbers were measured. At a 94-byte socket path VS Code started with nine
processes and created its socket. At 109 bytes one process survived and no
socket appeared. ChatGPT loses its socket in silence, which is harder to
diagnose.

A profile that half-works is harder to diagnose than one that was never
created. This is the same fail-closed choice as the process scan.

Windows named pipes live under `\\.\pipe\`, outside the profile directory. A
cap there would refuse a profile with a number that means nothing on that
machine.

### Tried and dropped

The first layout was `profiles/<uuid>`. It put an ordinary installation 17
bytes over the limit before the app had written anything.

## Shared configuration is linked to one copy

Each app with a shared file keeps one copy under `<data root>/<app id>/shared/`. Before
a launch the profile's file is linked to it. macOS and Linux use symbolic
links. Windows uses hardlinks, and both paths must be on the same drive.

A profile's own regular file is adopted as the shared copy when none exists
yet. When a shared copy exists, the profile's file is renamed to
`<filename>.replaced`. The suffix is appended, so `config.toml` becomes
`config.toml.replaced`.

The Default profile is linked too. Nothing is shared for Cursor, Devin,
T3 Code or VS Code.

### Why

Once a shared copy exists it is the source of truth for every profile. A new
profile's file must not overwrite it, or every other profile would lose its
settings in silence. Renaming keeps that file on disk.

Hardlinks need neither Developer Mode nor elevation on Windows. Link identity
is checked by volume serial and file index, since two fresh `{}` files can
match on size and creation time.

For the VS Code family `User/settings.json` is a plausible shared file. That is
a product decision a probe cannot settle, so nothing is shared until someone
decides.

## An uninstalled app is greyed, an unchecked platform is absent

An app declared for this platform but not installed is listed greyed, with
its reason. It offers no profiles and nothing to click. It becomes usable the
moment it is installed, with no relaunch. An app not declared for this
platform does not appear at all.

### Why

Someone who knows they have an app needs to tell "not installed" from "this
tool forgot about it". A user cannot know a build was never tried on their
platform, and a row that can only fail is worse than no row.

### Tried and dropped

On Windows an uninstalled app used to vanish, because the Default directory
had to exist before the app got a runtime. Installing it later did nothing
until a relaunch. Windows now falls back to the first candidate directory, as
macOS already did (#22).

## Task-switcher icons are shared on macOS and Windows

On macOS and Windows every profile of one app shows that app's icon in the
task switcher. The tray is how a user tells profiles apart there. On Linux each
profile gets its own desktop entry and window class,
`agent-profiles-<app id>-<profile id>`, passed as `--class`.

### Why

A per-profile app bundle would add code-signing and update maintenance for
every profile. Linux gets an identity from a desktop file and a launch
argument at no such cost.

The identity is keyed on the app id and the profile's immutable id. Renaming a
label rewrites the same entry and leaves no stale one. The same profile id
under two apps never collides.

## Focus on Linux depends on the session

On Wayland, Focus reports that it cannot raise the window and points to the
profile's taskbar entry or Alt-Tab. On X11 it uses `xdotool` when installed
and says to install it when not.

### Why

Wayland does not let one app raise another app's window. Saying so beats a
button that does nothing.

## Claude Desktop on Windows has two data paths

The official Windows install of Claude Desktop may be an MSIX package, which
Windows can virtualize. Its data directory may be
`%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude` or
`%APPDATA%\Claude`. The binary may be `%LOCALAPPDATA%\AnthropicClaude\claude.exe`
or the `WindowsApps` execution alias.

The app checks both data paths and takes the first that exists, with the MSIX
path listed first. With neither present it uses the MSIX path.

### Why

Nothing has confirmed which path a real install writes to. Both are declared
until the Windows acceptance run settles it. See
[platform status](platform-status.md).

## Start at login belongs to the OS

The toggle is off by default and starts only the tray. The OS holds the
setting: a LaunchAgent on macOS, a registry entry on Windows, an autostart
entry on Linux. The app keeps no copy and reads the real value each time. The
switch is shown disabled in debug builds.

### Why

A copy could disagree with what the user set in system settings. Reading the
OS each time means turning it off there shows here.

A login item registered from `pnpm tauri dev` would point at a `target/debug`
binary. That binary moves, gets rebuilt and disappears on `cargo clean`,
leaving an entry that fails silently at every boot.

## Keep awake holds a system flag while an agent works

Keep Awake is off by default and asks for nothing until turned on. Each
platform holds the machine a different way.

- macOS sets `pmset -a disablesleep 1` through a root loop.
- Windows sets the lid-close action in the user's power scheme to "do nothing"
  and records the prior action in `keep-awake.lid`.
- Linux holds a `systemd-inhibit --what=handle-lid-switch:idle` lock for as
  long as a child process lives.

Only macOS asks for a password.

### Why

macOS forces sleep on lid close unless the machine has external power and an
external display. `caffeinate` prevents idle sleep only. `disablesleep` is the
one setting that holds with the lid shut, and it is root's.

On Windows the lid action lives in a scheme the user already owns. On Linux
logind grants the inhibitor to the user and drops it when the process dies.

## The macOS root loop is inline and exits with the app

On Authorize the app runs one shell loop as root through `osascript`, once per
run of the app. The loop is built in memory and passed as a single argument.
Every 3 seconds it checks two things: whether `keep-awake.hold` exists, and
whether the app's pid still runs with the same start time. Flag present means
sleep is disabled. Flag gone means the prior value is restored. When the app
is gone the loop restores the prior value and exits. The loop writes only on a
change.

The app decides whether to hold every 15 seconds and creates or deletes the
flag.

### Why

A script under Application Support is writable by anything running as the
user. Running such a file as root would be a standing root escalation. An
inline script leaves nothing on disk.

The flag is tested for existence and never read, so its contents never reach a
root shell. The pid is paired with its start time because pids are recycled. A
loop tied to the app means a crash or force-quit cannot leave the Mac awake
forever. The 3-second poll bounds how long that takes, and anything shorter
buys nothing against a 15-second decision.

Writing only on a change means a user who runs `pmset` by hand is not fought
every poll.

## A breadcrumb records who owned the setting

Before it can hold anything, the loop writes the prior `SleepDisabled` value to
`keep-awake.owned`. It deletes the file on a clean exit. At launch the app
deletes any leftover flag. If it finds a breadcrumb, it hands the prior value
to the next loop and shows a Restore sleep banner when that value was 0.

### Why

`disablesleep` persists across a reboot. A kernel panic or power cut while
holding would otherwise leave a Mac that never sleeps, for a reason the user
cannot trace to this app. An unreadable breadcrumb is treated as prior 0.
Guessing "ours" costs a setting the user can reapply in one command. Guessing
"theirs" costs a machine that never sleeps.

Restore sleep disarms the trigger and drops the flag before it resets the
setting. A second copy of the app, such as a dev build beside the installed
one, can have its own loop watching the same flag. Dropping the flag is the
only way to make that loop let go.

## The flag file is not a privilege boundary

`keep-awake.hold` sits in the user's own data folder. Any process running as
the user can create it and keep the machine awake.

### Why

Agent Profiles runs as the user, so a boundary here would protect nothing the
user's processes cannot already reach. The worst outcome is a flat battery.
Root is never exposed, because the flag is never read.

## Agent activity comes from transcripts

The "when an agent is working" trigger watches session transcripts under
`~/.claude/projects` and `~/.codex/sessions`. For Claude Code it also reads
whether the last turn ended. A finished turn releases at once. A turn that
writes nothing for the idle window, 10 minutes by default and 1 to 60 allowed,
is taken as dead. Codex is judged by modification time alone. The "always"
trigger holds for as long as the app runs, for agents inside a desktop app.

The hold pauses when the machine is too hot, when on battery below the floor
(30% by default, 0 to 95 allowed), and ends when the app quits. The tab lists
the watched folders and how fresh each is.

### Why

A transcript is appended on every message and tool result. It moves during a
long network wait, when the process looks idle and a CPU check would read
"done".

Freshness alone is wrong at both ends. A transcript is written when a turn
ends, so it is freshest exactly when work stops. Resuming a session touches it
too: one session idle for 83 minutes read as working for five minutes after it
was reopened. The turn state in the transcript answers that, and the idle
window bounds the one case it cannot: a session killed mid-turn.

The idle window defaults to 10 minutes. Two minutes would release during a
five-minute build, and a released live run costs more than a held dead one.
The battery floor already bounds that cost.

Codex's session layout has not been verified against a real install, so it is
not parsed.

### Tried and dropped

Holds used to end at a duration cap. The cap stood in for heat, and a clock is
a poor proxy for temperature. It ended holds that were fine and missed a
machine overheating in a bag after ten minutes. The heat guard replaced it
(84d5553). It is on by default and can be turned off.

## The schedule arms a rolling batch of one-off wakes

The Schedule tab is macOS only. It wakes a sleeping Mac at each chosen
weekday's own time and opens a chosen app with `/usr/bin/open`. The wakes are
`pmset schedule wake` events, one per occurrence, for the next 56 days. A
per-user LaunchAgent, `~/Library/LaunchAgents/com.husniadil.agent-profiles.schedule.plist`,
opens the app with `StartCalendarInterval`. At launch the app re-arms when the
furthest wake is 14 days away or less. The tab shows how many days remain.

It opens any app. It does not launch a profile.

### Why

`pmset repeat` holds one wake time for all its days, so per-day times cannot
be a repeat. Absolute one-off events can name each day.

A 56-day horizon keeps the password prompt rare and keeps a machine left off
for a while from piling up stale events. Re-arming below 14 days puts a prompt
about every six weeks. The re-arm runs after the tray is up, so a prompt never
delays the tray icon. If the app is not opened for long enough, the count
reaches zero and wakes stop.

The LaunchAgent is the user's own and needs no root. Only the `pmset` batch
does, and all cancels and arms run under one prompt.

### Tried and dropped

The wakes were `wakeorpoweron`. Power-on from shutdown needs AC and is
unreliable on Apple Silicon even then. `wake` works on battery, because a
sleeping Mac keeps its clock powered. A Mac shut down at the scheduled time
now stays off.

The tab and README once said the Mac wakes about a minute before the set time.
The code sets the wake and the launch to the same minute, so the claim was
removed (e752aee).

## Updates are silent, and off means no request

Auto-update is on by default. Once per launch the app fetches `latest.json`
from this repository's latest GitHub release. If a newer version exists it
downloads, verifies the minisign signature, installs and relaunches with no
dialog. Check now runs the same check. With the switch off, no request is
made, and Check now is disabled.

Download and install are separate steps. Keep Awake releases its hold between
them and pauses its sweep for up to 10 minutes.

### Why

A tray app people forget about is the case auto-update exists for.

A check whose result is thrown away still tells GitHub the app ran. Off has to
mean no network. The guard lives in the hook as well as on the button's
disabled state, so no path can check while off.

Checking once per launch, and not on each toggle, avoids hammering GitHub.

The hold is handed back before install because on Windows the installer ends
the process. A lid action left on "do nothing" would outlive it. The sweep
pause stops a 15-second sweep re-taking the hold in that gap. Ten minutes
clears install and relaunch by an order of magnitude and still ends a pause
whose install never reports back.

The minisign signature proves an update came from this project's release
process. It is separate from OS code signing, and the bundles are unsigned.
The release policy lives in [CONTRIBUTING](../CONTRIBUTING.md#releasing).

## Where it lives

| File | What it holds |
|---|---|
| `src-tauri/src/app_spec.rs` | Each app's channels, locations, shared file and account field |
| `src-tauri/src/instance_manager.rs` | Launch arguments and environment, the fail-closed rescan |
| `src-tauri/src/account.rs` | Reading the account id and finding duplicates within one app |
| `src-tauri/src/paths.rs` | The `p` layout, the 104 and 108 byte limits, the socket refusal, every data file path |
| `src-tauri/src/profile_store.rs` | The 8-character profile id |
| `src-tauri/src/shared_config.rs` | Adopt, displace to `.replaced`, link |
| `src-tauri/src/runtime.rs` | Greyed uninstalled apps and absent undeclared ones |
| `src-tauri/src/platform/mod.rs` | The `Platform` trait and `wm_class` |
| `src-tauri/src/platform/macos.rs` | The root loop, `pmset` batches, the LaunchAgent |
| `src-tauri/src/platform/windows.rs` | Hardlinks, MSIX candidate choice, the lid-action hold |
| `src-tauri/src/platform/linux.rs` | Desktop identities, Wayland and X11 focus, the logind inhibitor |
| `src-tauri/src/keep_awake.rs` | Settings and guards, the 15 s sweep, breadcrumb recovery, restore |
| `src-tauri/src/agent_activity.rs` | Watched transcript roots and turn state |
| `src-tauri/src/schedule.rs` | The 56-day horizon, the 14-day re-arm, the LaunchAgent plist |
| `src-tauri/src/general.rs` | The auto-update and language settings |
| `src-tauri/src/lib.rs` | Autostart hidden in debug builds, startup recovery, the re-arm thread |
| `src-tauri/tauri.conf.json` | The updater endpoint and public key |
| `src/hooks/useUpdater.ts` | Once-per-launch check, the off guard, the keep-awake handoff |
