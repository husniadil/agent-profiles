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

#[derive(Serialize, Deserialize, Default)]
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
        let mut store = match std::fs::read(&file) {
            Ok(raw) => match serde_json::from_slice::<ProfileStore>(&raw) {
                Ok(store) => store,
                Err(_) => {
                    preserve_corrupt_registry(&file)?;
                    Self::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self::default(),
            // Everything else is a fault, not a verdict on the contents. A
            // locked file, a disk that hiccuped, a descriptor limit, a home
            // directory restored with the wrong owner — the bytes may be
            // perfectly good, and we have not read one of them. Treating that as
            // corruption moved a valid registry aside and started the user over
            // with a single profile, silently, while their real profile
            // directories stayed on disk with no way back to them.
            Err(error) => return Err(anyhow!("{}: {error}", file.display())),
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
        // Write a complete copy aside, then swap it in with a rename. A bare
        // write truncates the live registry before it knows whether the bytes
        // will land — on ENOSPC that leaves profiles.json empty or half-written,
        // and the next load treats the whole registry as corrupt. rename is
        // atomic and replaces an existing file on Windows too, so the live file
        // only ever changes from one complete registry to another.
        let tmp = file.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(self)?)?;
        std::fs::rename(&tmp, &file)?;
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
            if let Err(error) = std::fs::remove_dir_all(&removed.path) {
                // The directory could not be removed, so the registry has to go
                // on owning whatever is left of it: an entry beats an orphan
                // nothing will ever clean up. But remove_dir_all deletes children
                // as it walks and stops on the first it cannot remove, so this is
                // routinely a PARTIAL delete — nothing here inspected what
                // survived, so the message must not promise the data is intact.
                // It names the path because from here only the user can deal with
                // it. If putting the entry back fails too, name it all the same.
                let path = removed.path.clone();
                self.profiles.insert(idx, removed);
                if let Err(save_error) = self.save(paths) {
                    return Err(anyhow!(
                        "could not remove {}: {error} — and the profile could not be put back in the registry either: {save_error}. Part of its directory may already have been deleted; check {} before relying on it",
                        path.display(),
                        path.display()
                    ));
                }
                return Err(anyhow!(
                    "could not remove {}: {error}. The profile is still listed, but part of its directory may already have been deleted — check it before using it again",
                    path.display()
                ));
            }
        }
        Ok(())
    }

    pub fn set_account(&mut self, id: &str, account: Option<String>) {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.id == id) {
            p.account = account;
        }
    }
}

/// Moves a registry we cannot parse out of the way, without destroying one that
/// was moved aside earlier.
///
/// The first copy is the one worth keeping: by the time a second arrives, the
/// app has already rewritten the registry down to whatever it could still see,
/// so the newer file is the poorer record. Later copies are numbered rather than
/// dropped — each one costs a corruption event to create, so there is no run of
/// them to bound.
fn preserve_corrupt_registry(file: &Path) -> Result<()> {
    let mut corrupt = file.with_extension("json.corrupt");
    let mut nth = 1;
    while corrupt.exists() {
        corrupt = file.with_extension(format!("json.corrupt.{nth}"));
        nth += 1;
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
    fn a_failed_save_does_not_corrupt_the_registry_the_user_still_has() {
        // A bare write opens, truncates, then writes. If the write cannot finish
        // — ENOSPC is the realistic cause, and the delete rollback issues a save
        // precisely when the disk is already misbehaving — the file is left empty
        // or half-written, and the next load reads invalid JSON, moves the whole
        // registry aside, and starts over with a lone Default. Writing to a temp
        // file and renaming means the live registry is only ever replaced by a
        // complete one: a save that fails leaves it byte for byte as it was.
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        store.add("Kerja", &paths).unwrap();
        let file = paths.profiles_json();
        let intact = std::fs::read(&file).unwrap();

        // Stand a directory where the temp file has to be written, so the write
        // step of save fails before it can ever touch the live registry.
        std::fs::create_dir_all(file.with_extension("json.tmp")).unwrap();

        assert!(store.save(&paths).is_err(), "the save must fail");
        assert_eq!(
            std::fs::read(&file).unwrap(),
            intact,
            "a failed save must not corrupt the registry the user still has"
        );
    }

    #[test]
    fn a_successful_save_leaves_no_temp_file_behind() {
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        store.add("Kerja", &paths).unwrap();
        store.save(&paths).unwrap();
        assert!(
            !paths.profiles_json().with_extension("json.tmp").exists(),
            "the temp file must be renamed into place, not left lying around"
        );
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

    /// Makes the directory removal fail without permission games: a plain file
    /// standing where the profile directory belongs still `exists()`, and
    /// `remove_dir_all` refuses it on every platform.
    fn block_the_directory(path: &Path) {
        std::fs::remove_dir_all(path).unwrap();
        std::fs::write(path, b"not a directory").unwrap();
    }

    #[test]
    fn a_directory_that_cannot_be_removed_keeps_its_registry_entry() {
        // The directory holds the profile's credentials. If it survives the
        // delete, the registry has to go on owning it — an orphaned directory
        // is owned by nothing and cleaned up by nobody.
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        let p = store.add("Kerja", &paths).unwrap();
        block_the_directory(&p.path);

        let error = store.remove(&p.id, &paths).unwrap_err().to_string();
        assert!(
            error.contains(&p.path.display().to_string()),
            "the error has to name the directory so only-the-user can deal with it, got: {error}"
        );
        // remove_dir_all deletes children as it walks and stops on the first it
        // cannot remove, so a failure is routinely a PARTIAL delete — the message
        // must not promise the data is intact when nothing inspected what
        // survived.
        assert!(
            !error.contains("still on disk") && !error.contains("still listed and its data"),
            "the message must not claim the data is intact, got: {error}"
        );
        assert!(
            error.contains("may already have been deleted"),
            "the message must warn the directory may be partially gone, got: {error}"
        );
        assert!(
            store.get(&p.id).is_some(),
            "the profile must still be listed, matching what is on disk"
        );
        let reloaded = ProfileStore::load(&paths, &def).unwrap();
        assert!(
            reloaded.get(&p.id).is_some(),
            "the committed registry must not have dropped it either"
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

    #[test]
    fn a_registry_that_cannot_be_read_is_not_mistaken_for_a_corrupt_one() {
        // Corruption is a statement about bytes. A read that never returned any
        // has not earned it — the file may be perfectly good and merely locked,
        // on a disk that hiccuped, or behind a descriptor limit.
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        store.add("Kerja", &paths).unwrap();
        block_the_registry(&paths);

        assert!(
            ProfileStore::load(&paths, &def).is_err(),
            "a registry we cannot read is not an empty one: refuse, do not start over"
        );
        assert!(
            !paths
                .profiles_json()
                .with_extension("json.corrupt")
                .exists(),
            "nothing is moved aside on the strength of an error we never read past"
        );
    }

    /// The shape the bug was actually found in: a valid registry, a permission
    /// fault, and the question of whether the file is still there afterwards.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_registry_is_left_exactly_where_it_is() {
        use std::os::unix::fs::PermissionsExt;
        let (_d, paths, def) = fixture();
        let mut store = ProfileStore::load(&paths, &def).unwrap();
        store.add("Kerja", &paths).unwrap();
        let file = paths.profiles_json();
        let before = std::fs::read(&file).unwrap();

        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o000)).unwrap();
        // A test running as root can still read it, and would be asserting
        // nothing. Skip rather than fail: the same gate runs in a container.
        if std::fs::read(&file).is_ok() {
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
            return;
        }
        let outcome = ProfileStore::load(&paths, &def);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();

        assert!(outcome.is_err(), "a permission fault is not corruption");
        assert_eq!(
            std::fs::read(&file).unwrap(),
            before,
            "the registry is still the user's registry, byte for byte"
        );
        assert!(!file.with_extension("json.corrupt").exists());
    }

    #[test]
    fn a_second_corrupt_registry_does_not_destroy_the_first_one_preserved() {
        // The first copy set aside is by construction the likeliest to be the
        // good one: by the time a second arrives, the app has already rewritten
        // the registry down to whatever it could still see.
        let (_d, paths, def) = fixture();
        let file = paths.profiles_json();
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();

        std::fs::write(&file, b"{ the first, and the one worth keeping").unwrap();
        ProfileStore::load(&paths, &def).unwrap();
        std::fs::write(&file, b"{ the second").unwrap();
        ProfileStore::load(&paths, &def).unwrap();

        assert_eq!(
            std::fs::read(file.with_extension("json.corrupt")).unwrap(),
            b"{ the first, and the one worth keeping",
            "the first preserved copy survives the second event"
        );
        assert_eq!(
            std::fs::read(file.with_extension("json.corrupt.1")).unwrap(),
            b"{ the second",
            "and the second is kept too, under a name of its own"
        );
    }
}
