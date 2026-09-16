# agent-profiles

Agent Profiles is a tray app that keeps several signed-in copies of one desktop
app apart (Claude Desktop, ChatGPT, VS Code and others). Each profile is its own
data directory, launched and recognised by the app. It can also hold the machine
awake while an agent is working, and wake the Mac on a schedule to open an app.
It is Tauri 2: a React 19 window in `src/` and a Rust backend in
`src-tauri/src/`. macOS is verified on real hardware. Windows and Linux compile
in CI and have not been run.

## The map

Each module's header comment says what it is for. Read that before the code.

### Backend: boot and the command surface

- `main.rs`: calls `lib::run`.
- `lib.rs`: the boot. Registers plugins and commands, builds the tray, keeps
  the window hidden on close, and registers autostart only from a bundled
  release build.
- `runtime.rs`: one runtime per app declared on this platform, installed or
  not. A missing app is greyed with a reason and never fatal. `AppState` lives
  here.
- `commands.rs`: every `#[tauri::command]` the window calls. Profile add,
  rename, delete, launch, focus and quit, sizes, the socket budget, keep-awake
  and schedule settings.
- `tray.rs`: the tray menu. Rebuilt when its rows or the locale change.
- `general.rs`: the app's own settings (self-update and language),
  `Locale`, and `tray_strings`, the tray's translated text.

### Backend: profiles and apps

- `app_spec.rs`: everything that differs between supported apps, as data. A
  new app is a new `AppSpec` constant and nothing else.
- `profile_store.rs`: the profile registry per app. Ids are generated, and an
  unparseable registry is moved aside rather than lost.
- `paths.rs`: the data root and profile paths, kept short for the socket path
  limit (`SOCKET_PATH_LIMIT`, 104 bytes on macOS, 108 on Linux, none on Windows).
- `instance_manager.rs`: launch arguments, scan targets, and the guard against
  a second process on one profile directory.
- `shared_config.rs`: a config file shared across profiles by symlink (macOS,
  Linux) or hardlink (Windows).
- `account.rs`: reads which account a profile is signed in to, only to warn
  when two profiles share one.

### Backend: keep awake and schedule

- `agent_activity.rs`: whether an agent CLI is working now, read from its
  session transcripts on disk.
- `keep_awake.rs`: holding the machine awake with the lid shut, the guards
  (battery floor, caps, thermal), and giving the hold back.
- `schedule.rs`: the per-weekday wake-and-launch. The pure model and the
  `pmset`/launchd generators. The macOS backend executes the `WakePlan`.

### Backend: the OS axis (`platform/`)

- `mod.rs`: the `Platform` trait and `current()`. Nothing here names an app.
  `can_hold_awake` and `can_schedule_wake` default to false and each backend
  opts in. `can_read_thermal` is true wherever `thermal()` is not `Unknown`.
- `macos.rs`, `windows.rs`, `linux.rs`: the three backends. The Windows and
  Linux files compile everywhere so their pure helpers stay tested.
- `unix_ps.rs`, `win_proc.rs`: process table scans (`ps`, `tasklist`/PowerShell)
  with a pure `parse` the tests exercise.

### Backend: manual harness (`#[ignore]`d)

- `probe.rs`: answers the four `AppSpec` admission questions against an app
  not yet declared and prints a draft declaration. macOS only.
- `verify.rs`: launches real installed apps through the real `Platform` to
  check a declared `AppSpec` end to end.

### Frontend (`src/`)

- `App.tsx`: the window shell, the tabs, and the window's copy of
  `general::resolve_locale`.
- `components/ProfilesPanel.tsx`: the Profiles tab, over `ProfileList`,
  `ProfileRow`, `RowPanel` (inline rename and delete), `ComposeCard` (add a
  profile), `StatusStrip`, `Counters`, `BudgetMeter`, `IdentityChip`,
  `StateTag`, `PathText`, `EmptyState` and `ErrorBanner`.
- `components/keepawake/`: the Keep Awake tab, its status card, battery gauge
  and watched-session list.
- `components/schedule/ScheduleTab.tsx`: the Schedule tab.
- `components/general/`: the General tab and the update card.
- `components/motion/`: vendored beUI controls (select, switch, slider,
  combobox, availability scheduler). Size overrides used twice live in
  `lib/controls.ts`.
- `hooks/`: one hook per backend area (`useAppData`, `useKeepAwake`,
  `useSchedule`, `useGeneral`, `useSizes`, `useSocketBudget`, `useAutostart`,
  `useUpdater`).
- `lib/api.ts`: every Tauri command and its types, in one place.
- `lib/i18n/`: `en.ts` is the dictionary and the type the other five locales
  (`de`, `es`, `id`, `ja`, `pt`) are checked against. `index.tsx` is the
  provider.
- `lib/identity.ts` (a profile's hue from its id), `lib/paths.ts`,
  `lib/system.ts`, `lib/color.ts`, `lib/ease.ts`, `lib/touch.ts`,
  `lib/utils.ts`, `lib/hooks/`, and `format.ts`.

### Build and release

- `src-tauri/tauri.conf.json`: identifier `com.husniadil.agent-profiles`,
  bundle, updater.
- `src-tauri/capabilities/default.json`: the window's permissions.
- `.github/workflows/ci.yml`: `pnpm build`, `cargo fmt --check`, clippy with
  `-D warnings`, and `cargo test` on macOS, Windows and Linux.
- `.github/workflows/release.yml`: builds and publishes on a `v*` tag.
  `workflow_dispatch` rehearses the matrix without uploading.

## Commands

| Task | Command |
|---|---|
| Install | `pnpm install` |
| Run the app | `pnpm start` (never the bare `target/debug` binary, which opens a blank window) |
| Gate, what CI runs | `pnpm check` |
| Local unsigned bundle | `pnpm tauri build` |
| Probe an undeclared app (in `src-tauri`) | `PROBE_APP=/Applications/Something.app cargo test -- --ignored probe --nocapture` |
| Verify every declared app | `cargo test -- --ignored --nocapture` |
| Verify one app | `VERIFY_APP=<app id> cargo test -- --ignored launch_detect` |

`pnpm check` runs `pnpm build` (`tsc` and `vite build`), then in `src-tauri`
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
`cargo test`. It covers the platform you are on. CONTRIBUTING.md has the
recipes for checking the Windows build and the Linux gate from a Mac.

The probe and verify tests launch and quit real applications. They never run in
CI or in a plain `cargo test`.

## Docs are part of the change, not a follow-up

### What obliges a doc edit in the same commit

1. **A UI string added, or a label renamed.** `docs/manual.md` names every
   screen and control, so it changes with them.
2. **A new supported app or platform.** The supported apps table in
   `README.md`, `docs/platform-status.md`, and the `AppSpec` in `app_spec.rs`
   move together.
3. **A file named in the map moved or went.** Update this file.
4. **Anything a user can see changed.** Add a line to `CHANGELOG.md`.

A module added needs a line in the map. A reason that took a failure to learn
belongs in `docs/design.md`.

### Where each kind of doc lives

- `README.md`: installing and using the app. It ends with a section for AI
  agents.
- `docs/manual.md`: the user manual. What each thing does and where, never why.
- `docs/design.md`: the reasons. Each rule under its own heading, then why,
  then what was tried and dropped. It ends with a where-it-lives table.
- `docs/platform-status.md`: the acceptance ledger. Never tick an item without
  evidence from real hardware.
- `CONTRIBUTING.md`: setup, the gate, cross-platform recipes, adding an app,
  adding a UI string, releasing.
- `SECURITY.md`: reporting a vulnerability, what the app touches, and release
  integrity.
- `CHANGELOG.md`: newest first. History is not rewritten.

## Conventions worth knowing before editing

### Fail closed on a process scan

Claude Desktop has no single-instance lock. Two processes on one profile
directory both stay alive and corrupt its databases. So when a scan fails,
`instance_manager::launch` refuses to launch. Never turn a scan error into an
empty list (`unwrap_or_default()`). "I cannot tell" must never read as "nothing
is running".

### The socket path budget

Several apps put a Unix socket inside the profile directory, and `sun_path` is
capped at 104 bytes on macOS. That is why the profiles folder is `p` and
profile ids are eight characters. Do not lengthen a path segment without checking
`paths::socket_path_len`. The probe reports the budget for a new app.

### A new app is data

Add an `AppSpec` constant and its registry line in `app_spec.rs`. Answer the
four admission questions first (`probe.rs`, CONTRIBUTING.md). If a new app
needs an edit in a `platform/` backend, something app-specific leaked into the
OS axis. Declare a platform row only where someone has checked it.

### A UI string exists in every locale

Add the key to `src/lib/i18n/en.ts` first, then to `de.ts`, `es.ts`, `id.ts`,
`ja.ts` and `pt.ts`. A missing or extra key fails `pnpm build`. A string the
tray shows also goes in `tray_strings` in `src-tauri/src/general.rs`, where a
test fails if any of the six locales lacks it.

### Capabilities are asked of the platform

A feature that exists on some machines goes behind a `Platform` method that
defaults to false (`can_hold_awake`, `can_schedule_wake`). Commands check it
and the window receives `supported`. An unsupported tab is shown disabled with
the reason, never hidden.

### Test the decision

Put the judgement in a small pure function and test that, without a window
server or a running app. `schedule.rs` builds a `WakePlan` the macOS backend
executes. `unix_ps::parse` and `win_proc` parse text that a shell call returns.
Behaviour changes come with a test.
