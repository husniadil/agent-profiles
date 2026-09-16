# Contributing

How to work on this repository: run the app from a checkout, run the gate,
check the platforms you do not have, add an app or a string, and cut a
release. The code map and the rules a change has to follow, including which
docs move with which code, are in [CLAUDE.md](CLAUDE.md). They apply to people
as much as to agents.

Why the app is shaped the way it is (process pinning, path budgets, shared
configuration, platform caveats) is in [docs/design.md](docs/design.md). What
has been observed on real hardware is in
[docs/platform-status.md](docs/platform-status.md).

## The most useful contribution

This project was built and verified on macOS. Windows and Linux compile and
pass their tests in CI, but nobody has run the app on either. Their tests run
against fixtures.

If you run Windows or Linux, a real report that checks one box in
[docs/platform-status.md](docs/platform-status.md) is worth more than another
test written on macOS. A bug report that says "the process list looks like
this instead" counts, even without a patch.

## Run the app from a checkout

The toolchain is Rust plus `pnpm`, with Node 26 or newer (`engines` in
`package.json`). Install them however you like. The repository is developed
with [mise](https://mise.jdx.dev/). If you use it and its shims are not on
`PATH`, add them:

```bash
export PATH="$HOME/.local/share/mise/shims:$PATH"
```

With Rust from [rustup](https://rustup.rs/) or anywhere else, skip that line.

```bash
pnpm install
pnpm start
```

Use `pnpm start` (`tauri dev`). A development build loads its interface from
the Vite dev server, so running the binary in `target/debug` on its own opens
a blank window.

Build an unsigned local bundle for the current platform:

```bash
pnpm tauri build
```

## The gate

```bash
pnpm check
```

It runs `pnpm build` (`tsc` and `vite build`), then in `src-tauri`
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and
`cargo test`. It stops at the first failure.

CI (`.github/workflows/ci.yml`) runs the same four steps on every push to
`main` and every pull request. It runs them on `macos-latest`,
`windows-latest` and `ubuntu-22.04`, as separate steps so a failure names
itself. The frontend build comes first because the Rust build embeds
`../dist`. The build is expected to be warning-free. If your change adds a
warning, fix it.

`pnpm check` covers the platform you are on. The two recipes below cover the
others from a Mac. A `-D warnings` failure on a platform you cannot build for
stays invisible until CI reports it, and each recipe finds one in minutes.

### Check the Windows build from macOS

Tauri's build script compiles a Windows resource file, so it needs `llvm-rc`:

```bash
brew install llvm
export PATH="$(brew --prefix llvm)/bin:$PATH"
rustup target add x86_64-pc-windows-msvc

cd src-tauri
cargo clippy --target x86_64-pc-windows-msvc --all-targets -- -D warnings
```

This type-checks and lints everything, tests included. It does not link, and
it cannot run the tests.

### Run the Linux gate in a container

Cross-compiling to Linux needs a sysroot with GTK, dbus and webkit. A
container is simpler, and it runs the tests too.

```bash
pnpm build
docker run --rm -v "$PWD:/src:ro" ubuntu:22.04 bash -c '
  apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf \
    build-essential curl wget file libssl-dev libgtk-3-dev libxdo-dev \
    pkg-config ca-certificates >/dev/null
  curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal \
    --default-toolchain stable --component rustfmt,clippy >/dev/null
  . "$HOME/.cargo/env"
  # The build script writes inside the source tree, so work on a copy and
  # leave the host checkout alone. It needs ../dist from `pnpm build`.
  mkdir -p /build && cp -a /src/src-tauri /src/dist /build/ && cd /build/src-tauri
  export CARGO_TARGET_DIR=/tmp/target
  cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
'
```

Ubuntu 22.04 is the distribution CI pins. The container is
architecture-native, so on Apple Silicon it is arm64 Linux, where CI uses
amd64. The code has nothing architecture-specific in it. A container is still
not a desktop. It proves the code builds and its tests pass on Linux. It says
nothing about the tray, the window, or `xdotool`.

## Add an app

An app is data in `src-tauri/src/app_spec.rs`: one `AppSpec` constant and one
line in the `ALL` registry. No OS backend changes.

Answer four questions first. All four must be yes.

1. Can a profile be expressed as one directory?
2. Can that directory be selected at launch, through an argument or the
   environment?
3. Can the selection be read back off a running process?
4. Does no global lock survive the directories being separated?

These are limits. A sandboxed app fails (2) because the system pins its
container. An app that keeps its credentials in the system keychain fails (1)
because its profile is not a directory.

The questions have an executable form. Run the probe against a real
installation (macOS only, it reads the app bundle):

```bash
cd src-tauri
PROBE_APP=/Applications/Something.app cargo test -- --ignored probe --nocapture
```

The probe (`probe_an_app_bundle` in `src-tauri/src/probe.rs`) launches the
app twice, works out which channels move its profile, checks whether a second
profile can live beside the first, and prints a draft declaration with the
unknowns marked `TODO`. It runs at a path as long as the real profile layout
and reports the socket budget, so an id that is too long fails here. Try a
shorter one with `PROBE_ID=<id>`.

Once declared, the harness in `src-tauri/src/verify.rs` drives it end to end:

```bash
cargo test -- --ignored --nocapture                          # every check
VERIFY_APP=<app id> cargo test -- --ignored launch_detect    # just the new app
```

It creates a profile, launches the real app, confirms a process scan
attributes it to that profile, confirms the app wrote its state into the
profile directory, quits it, and cleans up. These tests launch real apps, so
they are `#[ignore]`d and never run in CI or in a plain `cargo test`.

Declare an app only for the platforms someone has checked. Leaving a
platform's row out is honest. A plausible-looking path is a guess that ships.

## Add a UI string

1. Add the key to `src/lib/i18n/en.ts`. Every other locale is type-checked
   against it.
2. Add the same key to `id.ts`, `ja.ts`, `de.ts`, `es.ts` and `pt.ts`. A
   missing or extra key fails `pnpm build`.
3. If the string also appears in the tray menu, add it to `tray_strings` in
   `src-tauri/src/general.rs`. The tray has its own table.
   `every_locale_translates_the_tray` fails `cargo test` when a locale lacks
   one.

## Tests

A change in behavior comes with a test. Test the decision, not the mechanism.
Much of this code keeps the judgement in a small pure function that a test can
call without a running app or a window server. Keep new code that way.

## What to be careful about

Claude Desktop has no single-instance lock. Two processes pointed at one
user-data directory both stay alive and corrupt its databases. Code that
touches launch, process scanning or profile deletion guards against real data
loss, so it fails closed. When it cannot tell whether a profile is running, it
refuses. Keep it that way. An `unwrap_or_default()` on a process scan turns "I
cannot tell" into "nothing is running", which is the wrong answer.

Keep Awake and Schedule run commands as root on macOS through `osascript ...
with administrator privileges`. The root script is generated in memory and
never written to disk, because anything under Application Support is
user-writable. Keep it that way, and keep `paths::unquotable_refusal` in front
of any path that reaches that script.

## Commit messages

Conventional prefixes, as in the history: `feat(scope):`, `fix(scope):`,
`docs(scope):`, `ci:`, `chore(release):`. The body explains why. The diff
already says what changed.

## Releasing

Users installing a build want
[Installing a release build](README.md#installing-a-release-build) instead.

### Cut a release

1. Set the new version in `package.json`, `src-tauri/Cargo.toml` and
   `src-tauri/tauri.conf.json`, and let `src-tauri/Cargo.lock` follow.
2. Add a `CHANGELOG.md` entry.
3. Commit as `chore(release): <version>`, tag `v<version>`, push the tag.

Nothing checks that the tag matches the version in the files. Check it
yourself.

### What the workflow does

A pushed `v*` tag runs `.github/workflows/release.yml`. It builds on three
runners with `tauri-action`, which runs `pnpm build` through
`beforeBuildCommand`. It does not run `cargo fmt`, `clippy` or `cargo test`,
so tag a commit that already passed CI.

| Runner | Artifacts |
| --- | --- |
| `macos-latest`, `--target universal-apple-darwin` | one universal `.dmg` for Intel and Apple Silicon, plus `.app.tar.gz` for the updater |
| `windows-latest` | `.msi` and NSIS `-setup.exe` |
| `ubuntu-22.04` | `.AppImage`, `.deb` and `.rpm` |

Evidence: the assets on `v0.7.0` (`gh release view v0.7.0`). Linux is pinned to
22.04 because a binary linked against a newer glibc refuses to start on older
distributions, with an error that blames the wrong thing.

`bundle.createUpdaterArtifacts` in `tauri.conf.json` makes each updater
artifact carry a `.sig`. The workflow signs them with the
`TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` secrets.
Without them the build still succeeds, and no installed copy accepts the
result. `uploadUpdaterJson: true` attaches `latest.json`, the manifest the
updater reads.

The release is created as a draft (`releaseDraft: true`), named
`Agent Profiles v<version>`, with a body that warns the builds are unsigned.

Running the workflow by hand (`workflow_dispatch`) is a rehearsal. `tagName`
is empty there, so nothing is uploaded to a release. The bundles come back as
workflow artifacts, which expire.

### Publishing the draft ships the update

The updater endpoint in `tauri.conf.json` is
`https://github.com/husniadil/agent-profiles/releases/latest/download/latest.json`.
GitHub resolves `/releases/latest` to the newest published release only. A
draft is invisible to every installed copy until someone clicks
**Publish release**.

That is the policy. Tagging builds and signs. Publishing installs on other
people's machines, and a person decides when. The cost is one click. It guards
against a bad build reaching every installed copy with nothing to undo it,
since the updater only moves forward.

The other failure is quieter. A draft left unpublished ships nothing, and every
installed copy keeps reporting itself up to date. Publishing the draft is part
of the release.
