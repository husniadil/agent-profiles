<p align="center">
  <img src="assets/banner.png" alt="Agent Profiles: run your coding agents side by side, one profile each" width="100%">
</p>

<p align="center">
  <a href="https://github.com/husniadil/agent-profiles/actions/workflows/ci.yml"><img src="https://github.com/husniadil/agent-profiles/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="MIT licence"></a>
  <img src="https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg" alt="Platforms">
</p>

# Agent Profiles

Run several accounts of a coding-agent desktop app at the same time, one profile each.

> **Unofficial.** This is a third-party tool with no affiliation to, endorsement by, or support from Anthropic or OpenAI. "Claude" and "Claude Desktop" are trademarks of Anthropic. "ChatGPT" and "Codex" are trademarks of OpenAI. This project only launches the applications you already installed, pointed at a different profile directory.

Agent Profiles is a menu bar and system tray app. Every profile gets its own directory, so you never sign out of one account to use another. Profiles of different apps can run side by side.

## What it does

- **Keeps accounts apart.** Each profile is its own directory. The installation you already have stays in place as the Default profile.
- **Launches and focuses.** Open a profile from the tray or the window. A profile that is already running is focused instead of started twice.
- **Shares one config file per app.** Claude and ChatGPT profiles share one configuration file, so an edit in one reaches all of them.
- **Keeps the machine awake while an agent works.** Close the lid and the run continues. Sleep returns when the work stops.
- **Wakes the Mac on a schedule.** Wake a sleeping Mac at a set time on chosen days and open an app.
- **Updates itself.** New releases install in the background, and you can turn that off.

## Supported apps

| App | Profile is selected by | Shared file | Declared on |
| --- | --- | --- | --- |
| **Claude** (Claude Desktop) | `--user-data-dir` | `claude_desktop_config.json` | macOS, Windows, Linux |
| **ChatGPT** (bundle id `com.openai.codex`) | `--user-data-dir` and `CODEX_HOME` | `config.toml` | macOS, Windows, Linux |
| **Cursor** | `--user-data-dir` | None | macOS |
| **Devin** (identifies as `com.exafunction.windsurf`) | `--user-data-dir` | None | macOS |
| **Windsurf** | `--user-data-dir` | None | macOS |
| **T3 Code** | `--user-data-dir` | None | macOS |
| **VS Code** | `--user-data-dir` | None | macOS |

The `codex` CLI reads the same `CODEX_HOME`, so a ChatGPT profile is the same profile on the command line.

Windsurf is declared but has not been probed against a real installation. If its path is wrong, it shows as not installed.

## Status

- **macOS** is verified against real installations.
- **Windows and Linux** compile and pass their tests in CI. Nobody has run the app on either yet.

[docs/platform-status.md](docs/platform-status.md) lists what has been checked on each platform.

## Installing a release build

Download the build for your system from the [Releases page](https://github.com/husniadil/agent-profiles/releases).

Releases are **unsigned**, so the operating system warns you before the first launch. The warning is misleading.

- **macOS** says the app "is damaged and can't be opened". The app is fine. It is only unsigned. Right-click the app, choose **Open**, then confirm. If macOS still refuses, clear the quarantine flag: `xattr -d com.apple.quarantine "/Applications/Agent Profiles.app"`
- **Windows** shows a SmartScreen warning about an unknown publisher. Choose **More info**, then **Run anyway**.

Only do this for a build from this project's Releases page. Treat either warning on a download from anywhere else with suspicion.

## Updating

Agent Profiles checks this repository's GitHub releases once per launch and installs a newer one by itself. Turn this off with **Update automatically** in the **General** tab. The [manual](docs/manual.md#updating) has the details.

## Learn more

| If you want to | Read |
| --- | --- |
| Use a feature and find it in the app | [The manual](docs/manual.md) |
| Know why something works the way it does | [Design](docs/design.md) |
| See what has been checked on each platform | [Platform status](docs/platform-status.md) |
| Build from a checkout, add an app, or cut a release | [Contributing](CONTRIBUTING.md) |
| Change the code as an agent | [CLAUDE.md](CLAUDE.md) |
| Report a vulnerability or check release integrity | [Security](SECURITY.md) |

## If you are an AI agent helping someone with Agent Profiles

Read the docs before you answer. Each kind of answer has one home.

- **Installing, or the unsigned-build warning:** [Installing a release build](#installing-a-release-build) above.
- **What a feature does and where it is in the window or tray:** [docs/manual.md](docs/manual.md). UI labels there match the English build.
- **Why it behaves that way:** [docs/design.md](docs/design.md).
- **Whether something works on Windows or Linux:** [docs/platform-status.md](docs/platform-status.md). An unchecked box means nobody has observed it.
- **Building, testing, or adding an app:** [CONTRIBUTING.md](CONTRIBUTING.md).
- **Changing the code:** [CLAUDE.md](CLAUDE.md).
- **A security question:** [SECURITY.md](SECURITY.md).

## License

MIT. See [LICENSE](LICENSE).
