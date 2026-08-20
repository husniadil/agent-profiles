use crate::paths::Paths;
use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: String,
    pub label: String,
    pub path: PathBuf,
    pub is_default: bool,
    /// The account this profile is signed into, as reported by whichever field
    /// the app records it in. `None` means unknown, never "no account".
    #[serde(default)]
    pub account: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Debug)]
pub struct ProfileStore {
    profiles: Vec<Profile>,
}

/// How many characters a profile id is. Public because the socket-path budget
/// is calculated for a profile that has not been created yet — see
/// [`crate::paths::socket_path_len`].
pub const ID_LEN: usize = 8;

impl ProfileStore {
    pub fn load(paths: &Paths, default_dir: &Path) -> Result<Self> {
        let file = paths.profiles_json();
        let mut store = match classify_registry(std::fs::read(&file)) {
            RegistryRead::Loaded(store) => store,
            RegistryRead::Missing => Self::default(),
            // Bytes we read but could not parse: the contents are unusable, so
            // preserve them and start over. This is the only kind of failure
            // that means the registry itself is bad.
            RegistryRead::Corrupt => {
                preserve_corrupt_registry(&file)?;
                Self::default()
            }
            // Bytes we could not read at all — EACCES, EIO, EMFILE, a locked or
            // wrong-owned file. The registry may be perfectly valid; we simply
            // cannot see it this moment. Discarding it here would replace the
            // real profiles with a lone Default and (via save) overwrite the
            // original for good. So refuse, like a failed process scan does, and
            // leave the file untouched for a later start that can read it.
            RegistryRead::Unreadable(error) => {
                return Err(anyhow!(
                    "could not read the profile registry at {} ({error}); \
                     refusing to start rather than discard it",
                    file.display()
                ));
            }
        };

        if !store.profiles.iter().any(|p| p.is_default) {
            store.profiles.insert(
                0,
                Profile {
                    id: "default".into(),
                    label: "Default".into(),
                    path: default_dir.to_path_buf(),
                    is_default: true,
                    account: None,
                },
            );
        }
        Ok(store)
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        let file = paths.profiles_json();
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn list(&self) -> &[Profile] {
        &self.profiles
    }

    pub fn get(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.id == id)
    }

    /// The first [`ID_LEN`] hex characters of a uuid, rather than the whole one.
    ///
    /// A profile id is a directory name, and its length is charged against the
    /// socket budget documented in `paths`: a uuid spends 36 bytes of it, which
    /// was enough on its own to push a real installation past the limit. At this
    /// width a draw collides about once in four billion, and the loop makes even
    /// that a non-event — an id only has to be unique within this store.
    fn fresh_id(&self) -> String {
        loop {
            let candidate: String = uuid::Uuid::new_v4()
                .simple()
                .to_string()
                .chars()
                .take(ID_LEN)
                .collect();
            if !self.profiles.iter().any(|p| p.id == candidate) {
                return candidate;
            }
        }
    }

    pub fn add(&mut self, label: &str, paths: &Paths) -> Result<Profile> {
        let id = self.fresh_id();
        let path = paths.profile_dir(&id);
        if let Some(reason) = crate::paths::socket_refusal(&path) {
            return Err(anyhow!(reason));
        }
        std::fs::create_dir_all(&path)?;
        let profile = Profile {
            id,
            label: label.to_string(),
            path,
            is_default: false,
            account: None,
        };
        self.profiles.push(profile.clone());
        // The registry is the commit point. A profile the registry does not
        // record does not exist, so if the write fails the directory has to go
        // back too — otherwise it lingers forever, owned by nothing.
        if let Err(error) = self.save(paths) {
            self.profiles.pop();
            let _ = std::fs::remove_dir_all(&profile.path);
            return Err(error);
        }
        Ok(profile)
    }

    pub fn rename(&mut self, id: &str, label: &str, paths: &Paths) -> Result<()> {
        let p = self
            .profiles
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow!("no profile with id {id}"))?;
        let previous = std::mem::replace(&mut p.label, label.to_string());
        if let Err(error) = self.save(paths) {
            self.rename_in_memory(id, &previous);
            return Err(error);
        }
        Ok(())
    }

    fn rename_in_memory(&mut self, id: &str, label: &str) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.label = label.to_string();
        }
    }

    pub fn remove(&mut self, id: &str, paths: &Paths) -> Result<()> {
        let idx = self
            .profiles
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow!("no profile with id {id}"))?;
        if self.profiles[idx].is_default {
            return Err(anyhow!("the Default profile cannot be removed"));
        }
        let removed = self.profiles.remove(idx);
        // Write the registry BEFORE the directory goes. Deleting first and
        // saving second means a failed save leaves an entry pointing at bytes
        // that no longer exist — the app would list a profile it had already
        // destroyed. This way a failed save costs the user nothing at all.
        if let Err(error) = self.save(paths) {
            self.profiles.insert(idx, removed);
            return Err(error);
        }
        if removed.path.exists() {
            std::fs::remove_dir_all(&removed.path)?;
        }
        Ok(())
    }

    pub fn set_account(&mut self, id: &str, account: Option<String>) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.account = account;
        }
    }
}

/// What a read of `profiles.json` amounts to. Kept apart from the IO so the one
/// judgement that matters — a fault reaching the bytes is *not* the same as the
/// bytes being bad — can be tested without a filesystem or a particular user.
#[derive(Debug)]
enum RegistryRead {
    /// Read and parsed: use it.
    Loaded(ProfileStore),
    /// No file yet: a fresh install, seed the Default.
    Missing,
    /// Read, but the bytes are not valid JSON: preserve them, start over.
    Corrupt,
    /// The read itself failed (anything but not-found): a transient fault, not
    /// corruption. The caller must refuse rather than discard the registry.
    Unreadable(std::io::Error),
}

fn classify_registry(read: std::io::Result<Vec<u8>>) -> RegistryRead {
    match read {
        Ok(raw) => match serde_json::from_slice::<ProfileStore>(&raw) {
            Ok(store) => RegistryRead::Loaded(store),
            Err(_) => RegistryRead::Corrupt,
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => RegistryRead::Missing,
        Err(error) => RegistryRead::Unreadable(error),
    }
}

fn preserve_corrupt_registry(file: &Path) -> Result<()> {
    let corrupt = file.with_extension("json.corrupt");
    match std::fs::remove_file(&corrupt) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    std::fs::rename(file, corrupt)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paths::Paths;

    fn fixture() -> (tempfile::TempDir, Paths, std::path::PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path().join("root"));
        let default_dir = dir.path().join("stock-claude");
        std::fs::create_dir_all(&default_dir).unwrap();
        (dir, paths, default_dir)
    }

    #[test]
    fn first_load_seeds_only_the_default_profile() {
        let (_d, paths, def) = fixture();
        let store = ProfileStore::load(&paths, &def).unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.list()[0].is_default);
        assert_eq!(store.list()[0].label, "Default");
        assert_eq!(store.list()[0].path, def);
        assert!(!paths
            .profiles_json()
            .with_extension("json.corrupt")
            .exists());
    }

    #[test]
    fn a_corrupt_registry_is_preserved_before_falling_back_to_default() {
        let (_d, paths, def) = fixture();
        let corrupt_bytes = b"{ not json";
        std::fs::create_dir_all(paths.profiles_json().parent().unwrap()).unwrap();
        std::fs::write(paths.profiles_json(), corrupt_bytes).unwrap();

        let store = ProfileStore::load(&paths, &def).unwrap();

        assert_eq!(store.list().len(), 1);
        assert!(store.list()[0].is_default);
        assert!(!paths.profiles_json().exists());
        assert_eq!(
            std::fs::read(paths.profiles_json().with_extension("json.corrupt")).unwrap(),
            corrupt_bytes
        );
    }

    #[test]
    fn a_corrupt_registry_falls_back_to_the_default_profile() {
        let (_d, paths, def) = fixture();
        std::fs::create_dir_all(paths.profiles_json().parent().unwrap()).unwrap();
        std::fs::write(paths.profiles_json(), b"{ not json").unwrap();
        let store = ProfileStore::load(&paths, &def).unwrap();
        assert_eq!(store.list().len(), 1);
        assert!(store.list()[0].is_default);
    }

    // The whole point of the fix: the four things a read of the registry can
    // mean are distinguished by classification alone, with no filesystem and no
    // dependence on running as an unprivileged user (so a root CI runner, where
    // chmod is a no-op, still exercises the transient-error branch).
    #[test]
    fn a_valid_registry_is_loaded() {
        let raw = serde_json::to_vec(&ProfileStore::default()).unwrap();
        assert!(matches!(
            classify_registry(Ok(raw)),
            RegistryRead::Loaded(_)
        ));
    }

    #[test]
    fn a_missing_registry_is_fresh_not_corrupt() {
        let missing = Err(std::io::Error::from(std::io::ErrorKind::NotFound));
        assert!(matches!(classify_registry(missing), RegistryRead::Missing));
    }

    #[test]
    fn unparseable_bytes_are_the_only_corruption() {
        assert!(matches!(
            classify_registry(Ok(b"{ not json".to_vec())),
            RegistryRead::Corrupt
        ));
    }

    #[test]
    fn a_transient_read_error_is_not_corruption() {
        // EACCES/EIO/EMFILE, a file a backup tool has locked, wrong ownership
        // after a restore — the bytes are intact, we just could not reach them.
        let denied = Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        match classify_registry(denied) {
            RegistryRead::Unreadable(error) => {
                assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied);
            }
            other => panic!("a read fault must not be classified as {other:?}"),
        }
    }

    // And the wiring: `load` must refuse rather than discard a registry it could
    // not read, leaving the file untouched for a later, readable start.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_registry_makes_load_refuse_and_keeps_the_file() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, paths, def) = fixture();
        let file = paths.profiles_json();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        let original = br#"{"profiles":[{"id":"aaaa1111","label":"Work","path":"/tmp/p/aaaa1111","is_default":false,"account":null}]}"#;
        std::fs::write(&file, original).unwrap();
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Root ignores the mode bits, so this branch is only meaningful for an
        // unprivileged user; the pure-function tests above cover the rest.
        if std::fs::read(&file).is_ok() {
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
            return;
        }

        let result = ProfileStore::load(&paths, &def);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();

        assert!(
            result.is_err(),
            "an unreadable registry must not load as Default"
        );
        assert!(
            file.exists(),
            "the registry must be left in place, not renamed"
        );
        assert!(
            !file.with_extension("json.corrupt").exists(),
            "a read fault must not preserve the file as corrupt"
        );
        assert_eq!(
            std::fs::read(&file).unwrap(),
            original,
            "the bytes must be untouched"
        );
    }

    #[test]
    fn added_profiles_get_a_directory_and_survive_a_reload() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let created = store.add("Kerja", &paths).unwrap();
        store.save(&paths).unwrap();

        assert!(created.path.is_dir());
        assert!(!created.is_default);
        assert_eq!(created.path, paths.profile_dir(&created.id));

        let reloaded = ProfileStore::load(&paths, &def).unwrap();
        assert_eq!(reloaded.list().len(), 2);
        assert_eq!(reloaded.get(&created.id).unwrap().label, "Kerja");
    }

    #[test]
    fn a_new_id_is_short_and_unique() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let mut seen = std::collections::HashSet::new();
        for _ in 0..50 {
            let created = store.add("x", &paths).unwrap();
            assert_eq!(created.id.len(), 8, "a uuid would eat the socket budget");
            assert!(
                created.id.chars().all(|c| c.is_ascii_hexdigit()),
                "an id is a directory name: {}",
                created.id
            );
            assert!(seen.insert(created.id.clone()), "ids must not repeat");
            // Labels are validated at the command layer, not here, so reusing
            // one is fine and keeps this focused on the id.
            store
                .rename(&created.id, &format!("x{}", created.id), &paths)
                .unwrap();
        }
    }

    #[test]
    fn an_id_never_collides_with_the_stock_profile() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let created = store.add("Kerja", &paths).unwrap();
        assert_ne!(created.id, "default");
    }

    #[test]
    fn renaming_changes_only_the_label() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let p = store.add("Kerja", &paths).unwrap();
        store.rename(&p.id, "Kantor", &paths).unwrap();
        assert_eq!(store.get(&p.id).unwrap().label, "Kantor");
        assert_eq!(store.get(&p.id).unwrap().path, p.path);
    }

    /// Makes `save` fail without needing a full disk: a directory standing where
    /// `profiles.json` belongs cannot be written to. Everything else about the
    /// layout still works, so the failure lands exactly where it is wanted.
    fn block_the_registry(paths: &Paths) {
        std::fs::remove_file(paths.profiles_json()).ok();
        std::fs::create_dir_all(paths.profiles_json()).unwrap();
    }

    #[test]
    fn a_registry_that_cannot_be_written_does_not_cost_the_user_their_data() {
        // The whole point of saving before deleting: the user asked to remove a
        // profile, the registry write failed, and their data is still there to
        // try again with rather than gone with no record of it.
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let p = store.add("Kerja", &paths).unwrap();
        block_the_registry(&paths);

        assert!(store.remove(&p.id, &paths).is_err());
        assert!(p.path.is_dir(), "the directory must survive a failed save");
        assert!(
            store.get(&p.id).is_some(),
            "the profile must still be listed, matching what is on disk"
        );
    }

    #[test]
    fn a_profile_the_registry_never_recorded_leaves_no_directory_behind() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        block_the_registry(&paths);

        assert!(store.add("Kerja", &paths).is_err());
        assert_eq!(store.list().len(), 1, "only the stock profile remains");
        let leftovers = std::fs::read_dir(paths.profiles_dir())
            .map(|entries| entries.count())
            .unwrap_or(0);
        assert_eq!(
            leftovers, 0,
            "an orphaned directory is owned by nothing and cleaned up by nobody"
        );
    }

    #[test]
    fn removing_deletes_the_directory() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let p = store.add("Kerja", &paths).unwrap();
        store.remove(&p.id, &paths).unwrap();
        assert!(store.get(&p.id).is_none());
        assert!(!p.path.exists());
    }

    #[test]
    fn the_published_id_length_is_the_one_ids_actually_use() {
        // The socket budget is computed for a profile that does not exist yet, so
        // it has to know this width without creating one.
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        assert_eq!(store.add("Kerja", &paths).unwrap().id.len(), ID_LEN);
    }

    #[test]
    fn the_default_profile_cannot_be_removed() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let id = store.list()[0].id.clone();
        assert!(store.remove(&id, &paths).is_err());
        assert_eq!(store.list().len(), 1);
        assert!(def.exists());
    }
}
