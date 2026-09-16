# Platform status

CI compiles and tests all three platforms on their own runners. The Windows and Linux tests only exercise parsing and path logic against fixtures. Nobody has launched the app on either. **Compiling is not running, and a passing unit test is not acceptance.** An unchecked box means nobody has observed the behavior on real hardware. It does not mean the behavior is broken.

You do not need a Windows or Linux machine to check that the code still builds and its tests still pass there. [CONTRIBUTING](../CONTRIBUTING.md#the-gate) has a recipe for each.

## macOS: verified against real applications

Confirmed by the harness in `src-tauri/src/verify.rs` driving real installations. The run covered the six apps declared at the time: Claude, ChatGPT, Cursor, Devin, T3 Code and Visual Studio Code.

- [x] All six apps detected as installed, each stock profile resolved at its own kind of path
- [ ] Windsurf, declared after that run, detected at its declared bundle path. Its path has never been probed against a real installation
- [x] Account identity read from both shapes of file: a top-level field for Claude, a nested one for ChatGPT
- [x] A profile launches, and a process scan attributes that pid back to it, for the argument-only app and for the argument-plus-environment one
- [x] The designation takes effect: the launched app writes its state into the profile directory, not the stock one
- [x] **Two apps run side by side, and neither app's process is attributed to the other.** The whole design rests on this
- [x] A profile path leaves room for the socket an application creates inside it, verified by launching one at the real profile path
- [x] A profile deleted after quitting leaves nothing behind

## macOS: the management window and the tray

Confirmed by a person driving a development build with the same six apps installed:

- [x] Tray menu opens and lists each app's profiles under its own heading
- [x] Management window opens from the tray. Closing it hides the window, and the tray survives
- [x] Adding a profile from the management window, including the app picker that appears only with more than one app installed
- [x] Deleting a profile from the management window
- [x] A duplicate label is refused, and the refusal appears beside the form that caused it
- [x] Renaming a profile from the management window
- [x] A blank label is refused
- [x] Deletion is refused while that profile is running, and the confirmation shows the directory size
- [x] The window refuses to be resized below its usable minimum
- [x] Tray liveness marker follows an app being launched and quit

The window redesign added controls that need a person in front of them. These boxes are open:

- [ ] The size on each row matches the directory, and the total in the status line matches their sum
- [ ] The running dot follows an app being launched and quit, as the tray marker does
- [ ] Open, rename and delete appear on hover and on keyboard focus, and every one is reachable by Tab
- [ ] Opening a profile from the window launches it, and focuses it rather than launching a second copy when it is already running
- [ ] The socket path budget under the add form shows this machine's real numbers

One box needs an installed release build:

- [ ] The Start at login switch registers and removes its LaunchAgent, and survives a reboot

The switch is disabled in development builds, because a login item registered from `pnpm start` would point at a `target/debug` binary that moves, gets rebuilt, and disappears on `cargo clean`. Closing this box needs an installed release build and a real reboot.

Automated UI driving was tried and abandoned. macOS attributes Accessibility to the responsible process, and a headless agent session has none that can be granted. These boxes still need a person.

## macOS: Keep Awake

Built in #7. Its tests ran the root loop against a stubbed `pmset`. The PR left everything past the administrator prompt unexecuted, and no later PR records a run against the real `pmset`.

- [x] The generated root script parses (`sh -n` in the test suite)
- [x] The thermal reading returns a real level on a real Mac (`Nominal`)
- [ ] After authorizing, `pmset -g` shows `SleepDisabled 1` while an agent works, and the value the user had before comes back when it stops
- [ ] Closing the lid on battery with no external display leaves the agent running
- [ ] Quitting, or `kill -9` on the app, restores `SleepDisabled` within one poll
- [ ] The battery floor and a `Serious` thermal state release the hold without the app quitting

## macOS: Schedule

- [x] The Mac woke unattended at a scheduled time and the chosen app (Slack) launched, with the fired event gone from `pmset -g sched` (#52, on a development build)
- [ ] A wake across a daylight-saving transition. Unit-tested only
- [ ] The background top-up re-arms wakes when coverage runs low

## Windows: compiles in CI, never run

- [x] CSV process parsing, the multi-app process filter, and the MSIX/classic path-picker logic covered by unit tests (run on macOS)
- [x] **Compiles on a real Windows runner**, and passes `clippy --all-targets -D warnings` and the test suite there. Everything below is unobserved
- [ ] Real process shape of either installed app
- [ ] MSIX vs classic default-directory selection against a real installation
- [ ] The declared ChatGPT install path. A plausible guess, never checked against a real Windows install
- [ ] Hardlink creation for the shared configuration
- [ ] Parallel instances, focus, quit, end-to-end launch
- [ ] Start at login writes and removes its registry entry
- [ ] Keep Awake sets the power scheme's lid-close action to do nothing, and restores the prior action on release, on quit and on session end

## Linux: compiles in CI, never run

- [x] Desktop-identity helpers, per-app window classes and filenames, `.desktop` metadata, and Wayland detection covered by unit tests (run on macOS)
- [x] **Compiles on a real Ubuntu runner**, and passes `clippy --all-targets -D warnings` and the test suite there. Everything below is unobserved
- [ ] Real `claude-desktop` process shape and default data path
- [ ] The declared ChatGPT command name and install path. A plausible guess, never checked against a real Linux install
- [ ] Per-profile `--class` producing a distinct taskbar identity
- [ ] X11 focus via `xdotool`, and the Wayland limitation path
- [ ] Symlink creation, parallel instances, quit flow
- [ ] Start at login writes and removes its autostart desktop entry
- [ ] Keep Awake holds a `systemd-inhibit` lid-switch inhibitor and drops it on release

The Schedule tab is macOS only. On Windows and Linux it is shown disabled with the reason.

Reports from Windows or Linux are especially welcome. A real report that checks one of those boxes is worth more than another test written on macOS.
