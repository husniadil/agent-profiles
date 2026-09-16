# Security policy

## Reporting a vulnerability

Report privately, not in a public issue. Open a
[security advisory](https://github.com/husniadil/agent-profiles/security/advisories/new),
or email husni.adil@gmail.com.

This is a personal project maintained in spare time. Expect a first response
within a week. Silence is not dismissal.

## What this app touches

Use this to judge whether something is a security issue.

### Profile directories

A profile directory holds a supported app's state for a signed-in account,
credentials included. For Claude Desktop and the other Electron apps it is
the `--user-data-dir`. For the ChatGPT desktop app it is `CODEX_HOME`, which
holds `auth.json` and the tokens the `codex` CLI also uses. Anything that leaks
a path, reads the contents, or deletes the wrong directory is a security issue.

### Deleting a profile

`delete_profile` removes a directory tree recursively. Two guards stand in
front of it:

- The Default profile cannot be removed (`profile_store.rs`, `remove`).
- A running profile cannot be deleted (`refuse_if_running` in `commands.rs`).
  A process scan that fails refuses the delete too, because the scan error
  propagates.

### Shared configuration

Claude's `claude_desktop_config.json` and ChatGPT's `config.toml` are linked
into every profile of that app (`shared_config` in `app_spec.rs`). A write to
one reaches every profile at once, and these files can hold MCP server
credentials. The other apps share no configuration.

### Account identifier

To warn when two profiles of one app look signed in to the same account, the
app reads one field: `lastKnownAccountUuid` from Claude's `config.json`, or
`tokens.account_id` from ChatGPT's `auth.json` (`account.rs`). The file is
parsed whole in memory to reach that field. Nothing else in it is used or
stored, so the access and refresh tokens beside `tokens.account_id` are never
used. The other apps declare no identity, and nothing is read for them. No
email address is read or shown, and nothing about your account or profiles is
sent anywhere.

### Root on macOS: Keep Awake and Schedule

Keep Awake holds `pmset -a disablesleep`, which only root can set. After you
enter an administrator password, the app starts a root shell loop through
`osascript` with `with administrator privileges`. The loop polls every three
seconds. It sets or restores `disablesleep` based on whether a flag file
exists, and exits when the app's process (pid and start time) is gone. The
script is built in memory and passed inline. It is never written to disk,
because a root script under user-writable Application Support would be a
standing privilege escalation. The loop tests the flag for existence and never
reads it. A data root containing a quote, backslash, CR or LF is refused
(`paths::unquotable_refusal`).

The flag is user-writable, so any process running as you can keep the Mac
awake while the loop runs. That costs battery. It does not grant root.

Schedule arms one-off `pmset schedule wake` events, which also need the
administrator password through the same path. Opening the app at the set time
is a per-user LaunchAgent in `~/Library/LaunchAgents` that runs
`/usr/bin/open`. That half needs no password.

On Windows, Keep Awake writes the current power scheme's lid-close action through
the `powrprof` API, records the prior action, and restores it. On Linux it holds a
`systemd-inhibit` lid-switch inhibitor. Neither asks for elevation.

### Network: the updater

The updater is the only network client in the app. It is on by default and
switched off with **Update automatically** in the General tab. Off means no
request at all.

- **When it runs:** once per app launch, on each **Check now**, and when you
  switch the setting on in a session that started with it off.
- **What it does:** if a newer release exists, it downloads it, installs it,
  and relaunches, without asking.
- **Hosts:** the endpoint in `tauri.conf.json` is
  `https://github.com/husniadil/agent-profiles/releases/latest/download/latest.json`.
  GitHub redirects that to `release-assets.githubusercontent.com`. The manifest
  points each bundle at `api.github.com`. A firewall allowlist that names only
  `github.com` breaks the updater, and it shows only "Couldn't check for
  updates".
- **What the check discloses:** your IP address and a
  `tauri-plugin-updater/2.10.1` user agent (the version in
  `src-tauri/Cargo.lock`). The endpoint has no `{{current_version}}` or
  `{{target}}` placeholder, so the check does not send your version or
  platform. The bundle download that follows is a per-platform URL, so that
  request does reveal your OS and package format.

## Release integrity

Release binaries are not OS code-signed. The operating system cannot verify
where they came from. Trust only artifacts from this repository's Releases
page.

The updater verifies each bundle against a minisign public key built into the
app (`plugins.updater.pubkey`). The secret half lives only in the release
workflow's secrets. A forged bundle needs that key. This signature is separate
from OS code signing. It does nothing for macOS Gatekeeper or Windows
SmartScreen. A build installed by the updater is as unsigned as one downloaded
by hand, and the warnings described in the README still apply.

**The signature covers the bundle. It does not cover the choice of bundle.**
The manifest has no signature of its own and is trusted on TLS alone. It
supplies the version the app compares against, plus the URL and signature of
the bundle to fetch. Someone able to serve that response cannot forge a
bundle. They can point the app at a different, genuinely signed release of this
project and declare any version for it. That is a downgrade that verifies
correctly. It needs control of the response for a `github.com` URL: a
repository takeover, an account with release-edit rights, or a TLS interception
the operating system trusts. The first two already allow worse. Closing it
needs a signed manifest, and the updater plugin has no hook for that.
