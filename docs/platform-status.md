# Platform status

CI compiles and tests all three platforms on their own runners, but the Windows and Linux tests only exercise parsing and path logic against fixtures — no one has ever launched this app on either. **Compiling is not running, and a passing unit test is not acceptance.** An unchecked box means the behavior has never been observed on real hardware, not that it is known to be broken.

You do not need a Windows or Linux machine to check that this still builds and its tests still pass there. [CONTRIBUTING](../CONTRIBUTING.md) has a recipe for each.

## macOS — verified against real applications

Confirmed by the harness driving real installations of the supported applications:

- [x] All six apps detected as installed, each stock profile resolved at its own kind of path
- [x] Account identity read from both shapes of file — a top-level field for Claude, a nested one for ChatGPT
- [x] A profile launches, and a process scan attributes that pid back to it — for the argument-only app and for the argument-plus-environment one
- [x] The designation takes effect: the launched app writes its state into the profile directory, not the stock one
- [x] **Two apps run side by side, and neither app's process is ever attributed to the other** — the premise the whole design rests on
- [x] A profile path leaves room for the socket an application creates inside it, verified by launching one at the real profile path
- [x] A profile deleted after quitting leaves nothing behind

## macOS — the management window and the tray

Confirmed by a person driving the app with all six applications installed:

- [x] Tray menu opens and lists each app's profiles under its own heading
- [x] Management window opens from the tray; closing it hides the window and the tray survives
- [x] Adding a profile from the management window, including the app picker that appears only with more than one app installed
- [x] Deleting a profile from the management window
- [x] A duplicate label is refused, and the refusal appears beside the form that caused it
- [x] Renaming a profile from the management window
- [x] A blank label is refused
- [x] Deletion is refused while that profile is running, and the confirmation shows the directory size
- [x] The window refuses to be resized below its usable minimum
- [x] Tray liveness marker follows an app being launched and quit

The window follows your system theme, in both light and dark. Colour in it means
exactly one thing each time it appears — green for a running profile, amber for a
shared sign-in, red for a destructive action — so nothing else is coloured.

The redesign added controls that need a person in front of them, and these boxes
are open:

- [ ] The size on each row matches the directory, and the total in the status line matches their sum
- [ ] The running dot follows an app being launched and quit, as the tray marker does
- [ ] Open, rename and delete appear on hover and on keyboard focus, and every one is reachable by Tab
- [ ] Opening a profile from the window launches it, and focuses it rather than launching a second copy when it is already running
- [ ] The socket path budget under the add form shows this machine's real numbers

One box remains from before, and it cannot be closed until a release exists:

- [ ] The Launch at login toggle registers and removes a login item, and survives a reboot

The toggle is deliberately hidden in development builds, because a login item registered from `pnpm start` would point at a `target/debug` binary that moves, gets rebuilt, and disappears on `cargo clean`. Closing this box needs an installed release build and a real reboot.

Automated UI driving was attempted and abandoned: macOS attributes Accessibility to the responsible process, and a headless agent session has no grantable one. These boxes still need a person.

## Windows — compiles in CI, never run

- [x] CSV process parsing, the multi-app process filter, and the MSIX/classic path-picker logic covered by unit tests (run on macOS)
- [x] **Compiles on a real Windows runner**, and passes `clippy -D warnings` and the test suite there. Compiling is not running: everything below is unobserved
- [ ] Real process shape of either installed app
- [ ] MSIX vs classic default-directory selection against a real installation
- [ ] The declared ChatGPT install path — a plausible guess, never checked against a real Windows install
- [ ] Hardlink creation for the shared configuration
- [ ] Parallel instances, focus, quit, end-to-end launch
- [ ] Launch at login writes and removes its registry entry

## Linux — compiles in CI, never run

- [x] Desktop-identity helpers, per-app window classes and filenames, `.desktop` metadata, and Wayland detection covered by unit tests (run on macOS)
- [x] **Compiles on a real Ubuntu runner**, and passes `clippy -D warnings` and the test suite there. Compiling is not running: everything below is unobserved
- [ ] Real `claude-desktop` process shape and default data path
- [ ] The declared ChatGPT command name and install path — a plausible guess, never checked against a real Linux install
- [ ] Per-profile `--class` producing a distinct taskbar identity
- [ ] X11 focus via `xdotool`, and the Wayland limitation path
- [ ] Symlink creation, parallel instances, quit flow
- [ ] Launch at login writes and removes its autostart desktop entry

Contributions running Windows or Linux are especially welcome — checking one of those boxes with a real report is worth more than any further test written on macOS.
