# Security Policy

## Reporting a vulnerability

Please report privately rather than in a public issue: open a [security advisory](https://github.com/husniadil/agent-profiles/security/advisories/new), or email husni.adil@gmail.com.

This is a personal project maintained in spare time. Expect a first response within a week; please do not read silence as dismissal.

## What this app touches

Worth knowing when judging whether something is a security issue:

- **Profile directories** hold a supported app's state for a signed-in account, including its credentials — Claude Desktop's user-data directory, or the ChatGPT desktop app's `CODEX_HOME`, which contains `auth.json` and the tokens the `codex` CLI also uses. Anything that leaks a path, reads their contents, or deletes the wrong one is a security concern, not just a bug.
- **`delete_profile` removes a directory tree recursively.** Its guards — the Default profile can never be removed, and a running profile refuses deletion — are what stand between a mistake and real data loss.
- **The shared configuration file** — `claude_desktop_config.json` for Claude, `config.toml` for ChatGPT — is linked into every profile of that app. A change that writes to it affects every one of them at once, and these files can contain MCP server credentials.
- **The account identifier** — `lastKnownAccountUuid` for Claude, `tokens.account_id` for ChatGPT — is read only to warn when two profiles of the same app look signed in to the same account. No other field of those files is read: notably, the access and refresh tokens sitting beside `tokens.account_id` are never touched. Email addresses are never read or displayed, and nothing about your account or profiles is ever sent anywhere. The only request this app makes on its own is the auto-updater asking `github.com/husniadil/agent-profiles` whether a newer release exists — on by default, switched off in Settings → General. When one exists it is downloaded, installed, and the app relaunches, without asking first.

## Release integrity

Release binaries are **unsigned**. This means the operating system cannot verify they came from this project, and you should only trust artifacts downloaded from this repository's Releases page. If you obtained an "Agent Profiles" build anywhere else, do not trust it. The auto-updater installs from that same Releases page and verifies each bundle against the public key baked into the app, so a forged bundle needs the release signing key.
