# Contributing

## The most useful contribution

This project was built and verified entirely on macOS. **Windows and Linux have never been compiled or run on real hardware** — every test for them executes on macOS against fixtures. See the platform checklists in [docs/platform-status.md](docs/platform-status.md).

If you run Windows or Linux, checking one of those boxes with a real report is worth more than any further test written on macOS. A bug report saying "the process list looks like this instead" is a genuine contribution, even without a patch.

## Setup

The toolchain is Rust plus `pnpm`. Install them however you like — nothing here depends on a particular version manager.

This repository happens to be developed with [mise](https://mise.jdx.dev/), a tool that installs and pins language runtimes per project. If you use it and its shims are not already on `PATH`, add them:

```bash
export PATH="$HOME/.local/share/mise/shims:$PATH"
```

If you installed Rust with [rustup](https://rustup.rs/) instead, or by any other means, skip that line entirely: `cargo` is already where the commands below expect it.

```bash
pnpm install
pnpm start
```

Start the app with `pnpm start`, not by running the binary from `target/debug`. A development build loads its interface from the Vite dev server, so a bare binary opens a blank management window.

Create an unsigned local bundle for the current platform:

```bash
pnpm tauri build
```

Why the app is shaped the way it is — process pinning, path budgets, shared configuration, and the platform caveats — is in [docs/design.md](docs/design.md). Cutting a release is [docs/releasing.md](docs/releasing.md).

## Before opening a pull request

Run what CI runs:

```bash
pnpm check
```

That covers the platform you are on. The two recipes below cover the other ones from a Mac, and they are worth the trouble: a `-D warnings` failure on a platform you cannot build for is invisible until CI says so, and each of these finds one in minutes.

### Checking the Windows build from macOS

Tauri's build script compiles a Windows resource file, so it needs `llvm-rc`:

```bash
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"
rustup target add x86_64-pc-windows-msvc

cd src-tauri
cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
```

This type-checks and lints everything, tests included, but it cannot **run** them: there is no linking and no Windows to run on. It catches a Windows-only compile error in seconds instead of after a push.

### Running the Linux gate in a container

Cross-compiling to Linux needs a whole sysroot — GTK, dbus, webkit — so use a container instead and get the real thing: compiled, linted, and the tests actually executed.

```bash
docker run --rm -v "$PWD:/src:ro" ubuntu:22.04 bash -c '
  apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    build-essential curl wget file libssl-dev libgtk-3-dev libxdo-dev \
    pkg-config ca-certificates >/dev/null
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
    --default-toolchain stable --component rustfmt,clippy >/dev/null
  . "$HOME/.cargo/env"
  # The build script writes inside the source tree, so work on a copy and
  # leave the host checkout alone. It needs ../dist, so run `pnpm build` first.
  mkdir -p /build && cp -a /src/src-tauri /src/dist /build/ && cd /build/src-tauri
  export CARGO_TARGET_DIR=/tmp/target
  cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
'
```

Ubuntu 22.04 is the distribution CI pins, and the container is architecture-native, so on Apple Silicon this is an arm64 Linux rather than the amd64 one CI uses. That difference has never mattered for this code, which contains nothing architecture-specific — but it is a difference, and a container is still not a desktop. It proves the code builds and its tests pass on Linux. It proves nothing about the tray, the window, or `xdotool`.

One command, so there is no chance of running a weaker check than CI does: it is the frontend build, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`, stopping at the first failure. The build is expected to be warning-free. If your change adds a warning, resolve it rather than leaving it for someone else to wonder about.


## Adding another app

An app is a data declaration in `src-tauri/src/app_spec.rs` — one `AppSpec` constant and one line in the registry. No OS backend is touched, which is what keeps the third app cheaper than the second.

Before declaring one, answer four questions. All four must be yes:

1. Can a profile be expressed as **one directory**?
2. Can that directory be **selected at launch**, through an argument or the environment?
3. Can the selection be **read back** off a running process?
4. Does **no global lock** survive the directories being separated?

These are limits, not obstacles to work around. A sandboxed app fails (2) because the system pins its container. An app keeping its credentials in the system keychain fails (1) because its profile is not a directory at all. Better to find that out at the declaration than three days into an implementation.

The four questions have an executable form. After declaring an app, run the manual harness against a real installation:

```bash
cd src-tauri
PROBE_APP=/Applications/Something.app cargo test -- --ignored probe --nocapture
```

The probe launches the application twice, works out which channels move its
profile and which are ignored, checks whether a second profile can live
alongside the first, and prints a draft declaration with the parts it cannot
know marked `TODO`. It runs at a path as long as the real profile layout, and
reports the socket budget every time — an id that is too long fails here rather
than in a user's tray. Try a shorter one with `PROBE_ID=<id>`.

Once declared, the same harness exercises it end to end:

```bash
cargo test -- --ignored --nocapture                          # every check
VERIFY_APP=<app id> cargo test -- --ignored launch_detect    # just the new app
```

It creates a profile, launches the real application, confirms a process scan attributes it back to that profile, confirms the app wrote its state into the profile directory rather than the stock one, quits it, and cleans up. These checks launch real applications, so they are `#[ignore]`d and never run in CI or in a normal `cargo test`.

Declare an app only for the platforms someone has actually checked. Leaving a platform's row out is honest; filling it with a plausible-looking path is a guess that ships.

## Adding a UI string

A visible string starts in `src/lib/i18n/en.ts`, which is the type every other locale is checked against — add the key there first. Then add the same key to the other five files (`id.ts`, `ja.ts`, `de.ts`, `es.ts`, `pt.ts`): a locale missing a key, or inventing one the English dictionary does not have, fails `pnpm build` rather than shipping a blank label. If the string also appears in the tray menu — which draws from its own small table rather than the frontend's dictionary — add it to `tray_strings` in `src-tauri/src/general.rs` as well; a Rust test there enforces that all six locales carry every tray string, so a locale that forgot one fails `cargo test` instead of shipping silence.

## Tests

Changes to behavior come with a test. Prefer testing the decision over the mechanism: much of this codebase is deliberately shaped so the interesting judgement lives in a small pure function that a test can call without a running app or a window server.

## What to be careful about

Claude Desktop has **no single-instance lock**. Two processes pointed at one user-data directory will both stay alive and corrupt its databases. Anything touching launch, process scanning, or profile deletion is guarding against real data loss, so those paths deliberately **fail closed**: when the code cannot tell whether a profile is running, it refuses rather than guesses. Please keep it that way — an `unwrap_or_default()` on a process scan turns "I cannot tell" into "nothing is running", which is precisely the wrong answer.

## Commit messages

Explain *why*, not *what*. The diff already says what changed.
