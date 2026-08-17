use crate::platform::Platform;
use anyhow::Result;
use std::path::{Path, PathBuf};

#[derive(Debug, PartialEq)]
pub enum LinkState {
    AlreadyLinked,
    AdoptFile(String),
    CreateFresh,
}

pub fn inspect(link: &Path, is_linked: bool) -> Result<LinkState> {
    if is_linked {
        return Ok(LinkState::AlreadyLinked);
    }
    match std::fs::read_to_string(link) {
        Ok(contents) => Ok(LinkState::AdoptFile(contents)),
        Err(_) => Ok(LinkState::CreateFresh),
    }
}

/// Where a profile's own copy is moved aside to when a shared config already
/// exists. Appends rather than replacing the extension, so `config.toml` becomes
/// `config.toml.replaced` and not `config.replaced`.
pub fn displaced_path(link: &Path) -> PathBuf {
    let mut name = link.file_name().unwrap_or_default().to_os_string();
    name.push(".replaced");
    link.with_file_name(name)
}

/// The seed written when there is nothing to adopt. An app that shares a JSON
/// file needs a valid empty object; one sharing TOML is happy with an empty file.
fn empty_document(filename: &str) -> &'static str {
    if filename.ends_with(".json") {
        "{}"
    } else {
        ""
    }
}

pub fn ensure_shared(
    platform: &dyn Platform,
    profile_dir: &Path,
    shared: &Path,
    filename: &str,
) -> Result<()> {
    let link = profile_dir.join(filename);
    let is_linked = is_same_file(&link, shared);

    match inspect(&link, is_linked)? {
        LinkState::AlreadyLinked => {
            write_shared_if_absent(shared, filename)?;
            return Ok(());
        }
        LinkState::AdoptFile(contents) => {
            create_parent(shared)?;
            // Adopt ONLY into an empty slot. Once a shared config exists it is the
            // single source of truth for every profile, and a newly-added profile
            // carrying its own file must not overwrite it — that would silently
            // destroy the servers every other profile is already using. The
            // displaced file is kept beside the profile so nothing is lost.
            if shared.exists() {
                std::fs::rename(&link, displaced_path(&link))?;
            } else {
                std::fs::write(shared, contents)?;
            }
        }
        LinkState::CreateFresh => write_shared_if_absent(shared, filename)?,
    }

    if std::fs::symlink_metadata(&link).is_ok() {
        std::fs::remove_file(&link)?;
    }
    platform.link(shared, &link)
}

fn write_shared_if_absent(shared: &Path, filename: &str) -> Result<()> {
    create_parent(shared)?;
    if !shared.exists() {
        std::fs::write(shared, empty_document(filename))?;
    }
    Ok(())
}

fn create_parent(shared: &Path) -> Result<()> {
    if let Some(parent) = shared.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
}

/// True when `link` and `shared` are the same underlying file — covering both the
/// symlink case (macOS, Linux) and the hardlink case (Windows).
fn is_same_file(link: &Path, shared: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        match (std::fs::metadata(link), std::fs::metadata(shared)) {
            (Ok(a), Ok(b)) => a.dev() == b.dev() && a.ino() == b.ino(),
            _ => false,
        }
    }
    #[cfg(windows)]
    {
        // Compare the volume serial + file index, which is what actually identifies
        // a file on Windows. Length-and-creation-time is not sufficient: two freshly
        // written `{}` files can share both values and be mistaken for hardlinks.
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::HANDLE;
        use windows::Win32::Storage::FileSystem::{
            GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION,
        };

        fn identity(path: &Path) -> Option<(u32, u32, u32)> {
            let file = std::fs::File::open(path).ok()?;
            let mut info = BY_HANDLE_FILE_INFORMATION::default();
            unsafe {
                GetFileInformationByHandle(HANDLE(file.as_raw_handle() as _), &mut info).ok()?;
            }
            Some((
                info.dwVolumeSerialNumber,
                info.nFileIndexHigh,
                info.nFileIndexLow,
            ))
        }

        match (identity(link), identity(shared)) {
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_config::tests_support::FakePlatform;

    const JSON: &str = "claude_desktop_config.json";
    const TOML: &str = "config.toml";

    fn tmp() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    #[test]
    fn an_already_linked_config_needs_no_work() {
        let d = tmp();
        let link = d.path().join("cfg.json");
        std::fs::write(&link, "{}").unwrap();
        assert_eq!(inspect(&link, true).unwrap(), LinkState::AlreadyLinked);
    }

    #[test]
    fn an_existing_regular_file_is_adopted_with_its_contents() {
        let d = tmp();
        let link = d.path().join("cfg.json");
        std::fs::write(&link, r#"{"mcpServers":{"a":{}}}"#).unwrap();
        assert_eq!(
            inspect(&link, false).unwrap(),
            LinkState::AdoptFile(r#"{"mcpServers":{"a":{}}}"#.to_string())
        );
    }

    #[test]
    fn a_missing_config_means_create_fresh() {
        let d = tmp();
        assert_eq!(
            inspect(&d.path().join("nothing.json"), false).unwrap(),
            LinkState::CreateFresh
        );
    }

    #[test]
    fn a_displaced_file_keeps_its_whole_name() {
        // `with_extension` would turn config.toml into config.replaced, quietly
        // changing which file the user has to go looking for.
        assert_eq!(
            displaced_path(Path::new("/p/config.toml")),
            PathBuf::from("/p/config.toml.replaced")
        );
        assert_eq!(
            displaced_path(Path::new("/p/claude_desktop_config.json")),
            PathBuf::from("/p/claude_desktop_config.json.replaced")
        );
    }

    #[test]
    fn a_json_seed_is_a_valid_empty_document_and_a_toml_seed_is_empty() {
        // `{}` in a TOML file is a syntax error, and an empty JSON file is not
        // parseable — the seed has to match the format the app expects.
        assert_eq!(empty_document(JSON), "{}");
        assert_eq!(empty_document(TOML), "");
    }

    #[test]
    fn ensure_shared_seeds_an_empty_document_when_there_is_nothing_to_adopt() {
        let d = tmp();
        let profile = d.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        let shared = d.path().join("shared").join(JSON);

        ensure_shared(&FakePlatform::default(), &profile, &shared, JSON).unwrap();

        assert_eq!(std::fs::read_to_string(&shared).unwrap().trim(), "{}");
    }

    #[test]
    fn ensure_shared_works_for_an_app_that_shares_toml() {
        let d = tmp();
        let profile = d.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join(TOML), "model = \"gpt-5.6\"\n").unwrap();
        let shared = d.path().join("shared").join(TOML);

        ensure_shared(&FakePlatform::default(), &profile, &shared, TOML).unwrap();

        assert!(std::fs::read_to_string(&shared)
            .unwrap()
            .contains("gpt-5.6"));
    }

    #[test]
    fn ensure_shared_copies_an_existing_config_into_shared_before_linking() {
        let d = tmp();
        let profile = d.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join(JSON), r#"{"mcpServers":{"keep":{}}}"#).unwrap();
        let shared = d.path().join("shared").join(JSON);

        ensure_shared(&FakePlatform::default(), &profile, &shared, JSON).unwrap();

        assert!(std::fs::read_to_string(&shared).unwrap().contains("keep"));
    }

    #[test]
    fn ensure_shared_does_not_overwrite_an_existing_shared_config() {
        let d = tmp();
        let profile = d.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        let link = profile.join(JSON);
        let profile_contents = r#"{"mcpServers":{"profile":{}}}"#;
        std::fs::write(&link, profile_contents).unwrap();

        let shared = d.path().join("shared").join(JSON);
        std::fs::create_dir_all(shared.parent().unwrap()).unwrap();
        let shared_contents = r#"{"mcpServers":{"shared":{}}}"#;
        std::fs::write(&shared, shared_contents).unwrap();

        ensure_shared(&FakePlatform::default(), &profile, &shared, JSON).unwrap();

        assert_eq!(std::fs::read_to_string(&shared).unwrap(), shared_contents);
        assert_eq!(
            std::fs::read_to_string(displaced_path(&link)).unwrap(),
            profile_contents
        );
    }
}

#[cfg(test)]
pub mod tests_support {
    use crate::app_spec::Locations;
    use crate::platform::{FocusHint, FocusOutcome, Platform, RunningProcess, ScanTarget};
    use std::path::{Path, PathBuf};

    /// Minimal Platform stand-in: real linking is a per-backend concern, so this
    /// one just copies. `running` lets a test stage a live instance.
    #[derive(Default)]
    pub struct FakePlatform {
        running: Vec<RunningProcess>,
        scan_fails: bool,
    }

    impl FakePlatform {
        pub fn with_running(running: Vec<RunningProcess>) -> Self {
            Self {
                running,
                scan_fails: false,
            }
        }

        pub fn failing_scan() -> Self {
            Self {
                running: Vec::new(),
                scan_fails: true,
            }
        }
    }

    impl Platform for FakePlatform {
        fn declared_here(&self, _locations: &Locations) -> bool {
            true
        }
        fn data_root(&self) -> anyhow::Result<PathBuf> {
            unimplemented!()
        }
        fn default_profile_dir(&self, _locations: &Locations) -> anyhow::Result<PathBuf> {
            unimplemented!()
        }
        fn binary(&self, _locations: &Locations, _product: &str) -> anyhow::Result<PathBuf> {
            Ok(PathBuf::from("/fake/app"))
        }
        fn process_marker(&self, _locations: &Locations) -> anyhow::Result<String> {
            Ok("/fake/app".into())
        }
        fn scan(&self, _targets: &[ScanTarget]) -> anyhow::Result<Vec<RunningProcess>> {
            if self.scan_fails {
                anyhow::bail!("fake process scan failed")
            }
            Ok(self.running.clone())
        }
        fn link(&self, source: &Path, target: &Path) -> anyhow::Result<()> {
            std::fs::copy(source, target)?;
            Ok(())
        }
        fn focus(&self, _pid: i32, _hint: &FocusHint) -> anyhow::Result<FocusOutcome> {
            Ok(FocusOutcome::Focused)
        }
        fn quit(&self, _pid: i32) -> anyhow::Result<()> {
            unimplemented!()
        }
        /// Overridden where the default bails, so a test can reach what
        /// `keep_awake::restore` does *after* the setting goes back. `hold` is
        /// deliberately left as the real flag-file default — the flag is the
        /// channel to a watchdog this process cannot see, so a fake one would
        /// test nothing.
        fn restore_sleep(&self) -> anyhow::Result<()> {
            Ok(())
        }
    }
}
