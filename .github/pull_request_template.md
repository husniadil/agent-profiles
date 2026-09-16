**What this changes, and why**

**How it was verified**

- [ ] `pnpm check` (frontend build, `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`)
- [ ] Ran the app and exercised the change by hand. On which platform?

**If this touches launch, process scanning, or profile deletion:** describe what happens when the process scan fails. These paths must fail closed and refuse. They must never assume nothing is running.

**If this touches Keep Awake or Schedule on macOS:** say whether it changes what runs as root, and confirm the root script is still never written to disk.
