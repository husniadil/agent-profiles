# Agent Profiles manual

What each feature does and where to find it. UI labels are the English ones. For the reasons behind a behaviour, read [design.md](design.md). For what has been checked on Windows and Linux, read [platform-status.md](platform-status.md).

## Where things are

Agent Profiles lives in the menu bar on macOS and the system tray on Windows and Linux.

- **The tray menu** lists each installed app under its own heading, with that app's profiles below it. Click a profile to open it. **Settings…** opens the management window. **Quit** closes Agent Profiles.
- **The management window** has four tabs: **Agent Profiles**, **Keep Awake**, **Schedule** and **General**. Closing the window hides it. The tray keeps running.

## Apps in the list

The app list is in the [README](../README.md#supported-apps).

- An app that is declared for your platform and installed gets a section with its profiles.
- An app that is declared for your platform but not installed is shown greyed, with the reason beside it. It has no profiles and nothing to click.
- An app that is not declared for your platform does not appear at all.

## The Default profile

**Default** is the installation that already exists on the machine. For example, `~/Library/Application Support/Claude` for Claude on macOS, and `~/.codex` for ChatGPT.

Agent Profiles uses that directory where it is. It never moves or copies it. Default launches with no profile argument and no environment variable, exactly as the app starts on its own. Default cannot be deleted.

Every other profile gets its own directory under the Agent Profiles data folder.

## Managing profiles

All of this is in the **Agent Profiles** tab.

- **Add a profile.** Type a name in the **New profile** card and press **Add**. When more than one supported app is installed, pick the app first. A blank name or a name already in use is refused.
- **Open a profile.** Use the open button on its row. A profile that is already running is focused instead of started again.
- **Rename a profile.** Use the rename button on its row, type the new name, then **Save name**.
- **Delete a profile.** Use the delete button on its row, then press and hold **Hold to delete**. The confirmation shows how much space the folder uses. A running profile cannot be deleted. Deletion cannot be undone.
- **See the totals.** The status line shows how many profiles exist, how many are running, and their size on disk. Click the folder path beside it to open the profiles folder in your file manager.
- **Socket path budget.** The meter under the add card shows how long a profile path can get on this system. If the data folder is too deep, no profile can be added there.

## Running profiles safely

Before every launch, Agent Profiles checks which profiles are already running.

- A profile that is running is focused instead of launched a second time.
- If the process list cannot be read, the launch is refused. Try again.
- A running profile shows **Running** in the window and a dot in the tray.

## Shared sign-in warning

Profile names are the ones you type. Agent Profiles never reads or shows account email addresses.

For Claude and ChatGPT, it compares each profile's account identifier with the other profiles of the same app. When two profiles appear to use the same account, the row shows **Shared sign-in** and the tray adds **(same account)** to the name. Profiles of different apps are never compared.

## Shared configuration

Claude profiles share `claude_desktop_config.json`. ChatGPT profiles share `config.toml`. Editing the file in one profile changes it for every profile of that app. The other apps share no file.

If a profile already has its own copy when a shared one exists, that copy is renamed to `<filename>.replaced`. For example, `config.toml.replaced`. Nothing is overwritten.

## Start at login

The switch is **Start at login** in the **General** tab. It is off until you turn it on.

When on, Agent Profiles starts at login with only the tray. No profile is opened.

The setting is stored by the operating system: a login item on macOS, a registry entry on Windows, an autostart desktop entry on Linux. The switch reads the real value each time, so a change made in system settings shows here.

In a development build the switch is disabled and reads "available once Agent Profiles is installed".

## Keep Awake

The **Keep Awake** tab keeps the machine awake with the lid closed while an agent works, and lets it sleep again when the work stops. It is off until you choose a trigger.

### Hold the machine awake

Choose one trigger:

- **Off.** The machine sleeps as usual.
- **When an agent is working.** A Claude Code or Codex session that is being written to holds the machine awake.
- **Always while Agent Profiles runs.** For agents inside a desktop app, where there is nothing to detect.

### Authorizing on macOS

macOS needs an administrator password once per run of Agent Profiles. Press **Authorize…** and enter it. A helper then turns the setting on while an agent works and off when it stops. The helper shuts down when Agent Profiles quits.

Windows and Linux ask for no password.

### Limits

- **Pause on low battery.** On battery, the hold is dropped below the charge you set, even mid-task. The default is 30%. It is ignored while plugged in. A machine with no battery never uses it.
- **Give up on a silent agent after.** An agent that finishes its turn releases the machine at once. This setting only covers an agent that stopped part-way: after this many minutes with nothing written, it counts as gone. The default is 10 minutes.
- **Thermal guard.** Releases the hold when the machine reports it is overheating. On by default. The switch is shown only where the machine reports its temperature, so it is absent on Windows.

### The status card

The card at the top shows the current state:

- **Off**, **Watching** (nothing working, nothing held), or **Keeping this Mac awake**.
- A paused state when the battery is low or the machine is too hot. The hold resumes on its own once plugged in or cooled.
- **Not yet authorized** on macOS before you press **Authorize…**.
- A failed-hold message, with the error, when the hold could not be set.
- **Not available here** when the system cannot hold the lid closed. On Linux this means `systemd-inhibit` was not found.

### Watching

The **Watching** list shows the Claude Code and Codex session folders being watched and how recently each was written to. Sessions appear once Claude Code or Codex has written one.

### Restore sleep

If Agent Profiles ended while holding the machine awake, the setting may still be in place.

- **macOS:** the next launch shows **Your Mac may not be able to sleep** with a **Restore sleep** button. To restore it by hand instead, run `sudo pmset -a disablesleep 0`.
- **Windows:** the next launch puts the lid-close action back without asking.
- **Linux:** the lock ends with the Agent Profiles process.

### What a hold covers on each platform

- **macOS:** all sleep is disabled while held. That includes lid-close sleep, the Sleep menu item and automatic sleep.
- **Windows:** the lid-close action of the current power plan is set to do nothing, then put back.
- **Linux:** a `systemd-inhibit` lock on the lid switch and idle sleep.

Windows and Linux have not been run on real hardware yet. See [platform-status.md](platform-status.md).

## Schedule

The **Schedule** tab wakes a sleeping Mac and opens an app at a set time on chosen days. It opens any app, not only a profile. It is available on macOS only. On Windows and Linux the tab shows **Not available here**.

### Setting it up

1. Turn on **Wake up the computer**.
2. Under **Days & times**, turn on each day you want and set its **Time**. Each day has its own time. **Copy times** copies one day's time to other days or to **Every day**.
3. Pick the app under **App to launch**. The list covers `/Applications`, `~/Applications` and `/System/Applications`, one folder level deep.

Changes save as you make them. Arming the wakes asks for an administrator password.

### When it works

- It wakes a Mac on AC power or on battery.
- You must be logged in. A locked screen counts. Logged out does not.
- A Mac that is fully shut down stays off.
- With the lid closed, the app only opens when an external display is connected.

### How far ahead it is armed

Wakes are scheduled up to 56 days ahead. The tab shows "Wakes are armed for the next N days". When fewer than 14 days are left, Agent Profiles arms the next batch the next time it starts, which asks for the password again. If Agent Profiles is not opened for long enough, the count reaches zero and wakes stop until you open it.

## Updating

Update controls are in the **General** tab.

- **Update automatically** is on by default. Once per launch, Agent Profiles checks this project's GitHub releases. If a newer release exists, it downloads it, installs it and restarts, with no dialog.
- **Check now** runs the same check on demand. It is disabled while updates are turned off.
- The card shows the installed version and a status line, such as "Up to date." or "Downloading… 40%".
- With **Update automatically** off, Agent Profiles makes no request to GitHub.

Each update is checked against a minisign signature before it installs. That signature is separate from operating system code signing. The builds are still unsigned, so the first install still shows the warnings in [Installing a release build](../README.md#installing-a-release-build).

## Languages

The window and the tray menu come in six languages: English, Bahasa Indonesia, 日本語, Deutsch, Español and Português.

Choose one in the **Language** row of the **General** tab. The change applies to the window and the tray menu at once, with no restart.

**Same as system** is the default. It reads the system language at startup and uses English when that language is not one of the six. Picking a language is remembered across restarts. Picking **Same as system** again hands the choice back to the system.
