# Design notes

Why Agent Profiles is shaped the way it is. None of this is needed to use the
app — [the README](../README.md) covers that. It is here for anyone reading the
code, or deciding whether a change is safe.

## How a profile is pinned to a process

Every supported app has to answer two separate questions, and they are not the same question:

- **Writing** — how is a launching process told which profile to use? An argument, an environment variable, or several at once.
- **Reading back** — how do we tell, later, which profile a running process belongs to?

Writing is cheap on any channel. Reading is not: recovering an argument means parsing the process table, which is routine, whereas recovering an environment variable means `KERN_PROCARGS2` on macOS, `/proc/pid/environ` on Linux and `NtQueryInformationProcess` on Windows.

So an app may write through as many channels as it needs, provided at least one of them is readable. ChatGPT is exactly that case: it needs `--user-data-dir` to move Chromium's data **and its single-instance lock**, and `CODEX_HOME` to move the credentials and configuration that are actually worth separating. Both point at the same directory, so a profile stays one folder.

## Why profile paths are short

A profile directory is `<data root>/<app id>/p/<8 characters>` rather than something more readable, and the app ids are terse for the same reason.

Several of these applications create a Unix domain socket **inside** the profile directory — VS Code writes `<version>-main.sock`, the ChatGPT desktop app writes `ipc/ipc.sock`. macOS caps a socket path at 104 bytes and Linux at 108, and that budget is shared by every naming decision above it: the product name, the app id, the profile id, and the length of the user's home directory, which is not ours to choose.

The numbers are measured, not assumed. At a 94-byte socket path VS Code started with nine processes and created its socket; at 109 bytes exactly one process survived and no socket appeared. The ChatGPT desktop app is less brittle and merely loses its socket in silence, which is harder to diagnose. An earlier layout of `profiles/<uuid>` put a perfectly ordinary installation 17 bytes over the limit before the application had written a single byte.

Because the home directory can still be long enough to exhaust the budget, creating a profile that would leave no room for a socket is refused outright, with the numbers in the message. It is the same fail-closed choice made when a process scan fails: a profile that half-works is far harder to diagnose than one that was never created.

## Shared configuration

Each app's shared configuration file is shared across that app's profiles. Agent Profiles keeps one source-of-truth copy per app and links each profile's file to it before launch:

- **macOS and Linux:** symbolic links.
- **Windows:** hardlinks, so Developer Mode or elevation is not required. Both paths must be on the same drive.

If a profile has an existing regular file, its contents are adopted into the shared copy when there is no shared file yet. When a shared file already exists, the profile's own copy is moved aside to `<filename>.replaced` rather than silently overwriting the configuration every other profile is using.

## Task-switcher icons

On macOS and Windows, all instances of one app intentionally share that app's icon in the operating system's task switcher. Agent Profiles does not create per-profile app bundles, because doing so would add code-signing and update-maintenance risk. The tray is the navigation surface on those platforms. Linux is designed differently: each profile receives its own desktop identity and taskbar entry, but that behavior still awaits live Linux acceptance.

## Linux and Wayland focus limitation

Native Wayland does not allow one application to raise another application's window. On Wayland, the tray's Focus action reports that limitation and points the user to the profile's taskbar entry or Alt-Tab. On X11, the app can use `xdotool` when it is installed. Each desktop identity is keyed by app id and the profile's immutable id, so renaming a label rewrites the same identity rather than leaving a stale entry, and the same profile id under two apps never collides.

## Windows MSIX caveat

The official Windows installation of Claude Desktop may be an MSIX package. Windows can virtualize its writes, so the effective data directory may be:

```text
%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Roaming\Claude
```

rather than:

```text
%APPDATA%\Claude
```

Agent Profiles probes both and prefers the MSIX package path when both exist. The Windows acceptance run must confirm which path the installed build actually uses. The binary may likewise come from the direct-install location or the WindowsApps execution alias.
