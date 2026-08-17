//! The app's own two settings: whether it updates itself, and what it speaks.
//!
//! Wired into the running app: `Handle` lives on `AppState` (see
//! `runtime.rs`), the `general_settings`/`set_general_settings` commands read
//! and write it, and `tray_strings`/`Locale` drive the tray's own translated
//! text.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The languages the window and the tray are translated into.
///
/// A closed enum rather than a string: an unknown tag reaching the frontend
/// would index a dictionary that has no such key, and the window would render
/// nothing at all. Serde rejects it at the boundary instead, and a hand-edited
/// `general.json` naming a language we do not have falls back to the default
/// along with the rest of an unparseable file.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Id,
    Ja,
    De,
    Es,
    Pt,
}

impl Locale {
    /// Every locale, in the order the picker offers them: English first because
    /// it is the fallback, then the rest as they were added.
    pub const ALL: &'static [Locale] = &[
        Locale::En,
        Locale::Id,
        Locale::Ja,
        Locale::De,
        Locale::Es,
        Locale::Pt,
    ];

    /// The BCP-47 language subtag, which is also the key the frontend
    /// dictionaries are filed under.
    pub fn tag(self) -> &'static str {
        match self {
            Locale::En => "en",
            Locale::Id => "id",
            Locale::Ja => "ja",
            Locale::De => "de",
            Locale::Es => "es",
            Locale::Pt => "pt",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        Locale::ALL.iter().copied().find(|l| l.tag() == tag)
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// Checked once per launch when on, and never when off. Off means no request
    /// to GitHub at any point, not a check whose result is ignored.
    #[serde(default = "default_auto_update")]
    pub auto_update: bool,
    /// `None` is "follow the system", which is not the same as choosing English:
    /// someone who has never opened this tab gets their own language when we add
    /// support for it, and someone who explicitly chose English keeps English.
    #[serde(default)]
    pub locale: Option<Locale>,
}

/// On, and on for a settings file written before this existed. The alternative
/// is an installed copy that silently stops receiving fixes because nobody
/// opened a tab they had no reason to open.
fn default_auto_update() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_update: default_auto_update(),
            locale: None,
        }
    }
}

impl Settings {
    /// Falls back to the defaults for anything unreadable, like
    /// [`crate::keep_awake::Settings::load`] and for the same reason: the
    /// defaults are safe, and losing a language preference is a smaller failure
    /// than refusing to start.
    pub fn load(file: &Path) -> Self {
        std::fs::read_to_string(file)
            .ok()
            .and_then(|raw| serde_json::from_str::<Settings>(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, file: &Path) -> Result<()> {
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(file, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

/// Which language to actually speak, given what was chosen and what the system
/// reports.
///
/// Matched on the language subtag alone. A system reporting `pt-BR` or
/// `de_AT.UTF-8` wants Portuguese and German respectively, and a lookup on the
/// full tag would hand both of them English.
pub fn resolve_locale(chosen: Option<Locale>, system: Option<&str>) -> Locale {
    if let Some(locale) = chosen {
        return locale;
    }
    system
        .and_then(|tag| {
            let language = tag
                .split(['-', '_', '.'])
                .next()
                .unwrap_or_default()
                .to_ascii_lowercase();
            Locale::from_tag(&language)
        })
        .unwrap_or(Locale::En)
}

/// The tray's strings, which are the only translated text outside the window.
///
/// A second, tiny table rather than a share of the frontend's dictionary: three
/// strings duplicated across two languages of code is a far smaller thing to
/// carry than a build step that hands a JSON file to both. The test above is
/// what keeps this table from falling behind [`Locale::ALL`].
pub struct TrayStrings {
    pub settings: &'static str,
    pub quit: &'static str,
    /// Appended to a profile label, so it keeps its leading space.
    pub same_account: &'static str,
}

pub fn tray_strings(locale: Locale) -> TrayStrings {
    match locale {
        Locale::En => TrayStrings {
            settings: "Settings…",
            quit: "Quit",
            same_account: " (same account)",
        },
        Locale::Id => TrayStrings {
            settings: "Pengaturan…",
            quit: "Keluar",
            same_account: " (akun sama)",
        },
        Locale::Ja => TrayStrings {
            settings: "設定…",
            quit: "終了",
            same_account: " （同じアカウント）",
        },
        Locale::De => TrayStrings {
            settings: "Einstellungen…",
            quit: "Beenden",
            same_account: " (gleiches Konto)",
        },
        Locale::Es => TrayStrings {
            settings: "Ajustes…",
            quit: "Salir",
            same_account: " (misma cuenta)",
        },
        Locale::Pt => TrayStrings {
            settings: "Configurações…",
            quit: "Sair",
            same_account: " (mesma conta)",
        },
    }
}

/// Reads the operating system's language once, at startup.
pub fn system_locale() -> Option<String> {
    sys_locale::get_locale()
}

/// The live settings, mirroring [`crate::keep_awake::Handle`]: the file is the
/// record and the mutex is the copy everything reads, so the tray does not touch
/// the disk every time it is opened.
pub struct Handle {
    data_root: PathBuf,
    settings: Mutex<Settings>,
    /// Read once at startup rather than per call: the OS language does not
    /// change under a running process, and `get_locale` is not free.
    system: Option<String>,
}

impl Handle {
    /// `system` is read by the caller (via [`system_locale`]) rather than here,
    /// so this takes its environment as a parameter like
    /// [`crate::keep_awake::Handle::new`] takes its `Capabilities`/`Recovery` —
    /// and so the "follow the system" path can be tested with a known locale
    /// instead of whatever happens to be set on the machine running the tests.
    pub fn new(data_root: PathBuf, system: Option<String>) -> Self {
        let settings = Settings::load(&crate::paths::general_settings(&data_root));
        Self {
            data_root,
            settings: Mutex::new(settings),
            system,
        }
    }

    pub fn settings(&self) -> Settings {
        self.settings.lock().map(|held| *held).unwrap_or_default()
    }

    /// The language to actually render in, which is what the tray asks for.
    pub fn locale(&self) -> Locale {
        resolve_locale(self.settings().locale, self.system.as_deref())
    }

    pub fn set_settings(&self, next: Settings) -> Result<()> {
        next.save(&crate::paths::general_settings(&self.data_root))?;
        if let Ok(mut held) = self.settings.lock() {
            *held = next;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_install_updates_itself_and_follows_the_system() {
        let settings = Settings::default();
        assert!(
            settings.auto_update,
            "a tray app people forget about is exactly the case auto-update exists for"
        );
        assert_eq!(
            settings.locale, None,
            "None means follow the system, which is not the same as choosing English"
        );
    }

    #[test]
    fn an_unreadable_or_missing_file_lands_on_the_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("general.json");
        assert_eq!(Settings::load(&missing), Settings::default());

        let broken = dir.path().join("broken.json");
        std::fs::write(&broken, b"{ not json").unwrap();
        assert_eq!(Settings::load(&broken), Settings::default());
    }

    #[test]
    fn a_file_written_before_a_field_existed_keeps_the_other_one() {
        let dir = tempfile::tempdir().unwrap();
        let partial = dir.path().join("partial.json");
        std::fs::write(&partial, br#"{"locale":"ja"}"#).unwrap();
        let loaded = Settings::load(&partial);
        assert_eq!(loaded.locale, Some(Locale::Ja));
        assert!(loaded.auto_update, "the missing field takes its default");
    }

    #[test]
    fn settings_survive_a_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("general.json");
        let written = Settings {
            auto_update: false,
            locale: Some(Locale::Pt),
        };
        written.save(&file).unwrap();
        assert_eq!(Settings::load(&file), written);
    }

    #[test]
    fn an_explicit_choice_beats_the_system() {
        assert_eq!(
            resolve_locale(Some(Locale::De), Some("ja-JP")),
            Locale::De,
            "someone who picked a language is not overruled by their OS"
        );
    }

    #[test]
    fn no_choice_follows_the_system_by_language_not_by_region() {
        // The region is what makes this a prefix match rather than a lookup:
        // there is one `pt`, and `pt-BR` must not fall through to English.
        assert_eq!(resolve_locale(None, Some("pt-BR")), Locale::Pt);
        assert_eq!(resolve_locale(None, Some("de_AT.UTF-8")), Locale::De);
        assert_eq!(resolve_locale(None, Some("ja")), Locale::Ja);
        assert_eq!(resolve_locale(None, Some("ID")), Locale::Id);
    }

    #[test]
    fn an_unsupported_or_unreadable_system_locale_falls_back_to_english() {
        assert_eq!(resolve_locale(None, Some("fr-FR")), Locale::En);
        assert_eq!(resolve_locale(None, Some("")), Locale::En);
        assert_eq!(resolve_locale(None, None), Locale::En);
    }

    #[test]
    fn every_locale_translates_the_tray() {
        // The tray's strings live in Rust, so nothing type-checks them against
        // English the way the frontend dictionaries check each other. This is
        // that check: a new locale that forgets the tray fails here.
        for locale in Locale::ALL {
            let strings = tray_strings(*locale);
            assert!(
                !strings.settings.is_empty(),
                "{locale:?} has no Settings row"
            );
            assert!(!strings.quit.is_empty(), "{locale:?} has no Quit row");
            assert!(
                !strings.same_account.is_empty(),
                "{locale:?} has no same-account suffix"
            );
            assert!(
                strings.same_account.starts_with(' '),
                "{locale:?} must keep the leading space — it is appended to a label"
            );
        }
    }

    #[test]
    fn a_handle_with_no_choice_follows_the_injected_system_locale() {
        let dir = tempfile::tempdir().unwrap();
        let handle = Handle::new(dir.path().to_path_buf(), Some("de-DE".into()));
        // No stored choice yet: it follows the system.
        assert_eq!(handle.locale(), Locale::De);
        // An explicit choice is written through and then wins, on disk too.
        handle
            .set_settings(Settings {
                auto_update: false,
                locale: Some(Locale::Es),
            })
            .unwrap();
        assert_eq!(handle.locale(), Locale::Es);
        let reopened = Handle::new(dir.path().to_path_buf(), Some("de-DE".into()));
        assert_eq!(reopened.locale(), Locale::Es);
        assert!(!reopened.settings().auto_update);
    }

    #[test]
    fn a_handle_reads_what_was_written_through_it() {
        let dir = tempfile::tempdir().unwrap();
        let handle = Handle::new(dir.path().to_path_buf(), None);
        // Whatever the machine running this test speaks, an explicit choice wins.
        handle
            .set_settings(Settings {
                auto_update: false,
                locale: Some(Locale::Es),
            })
            .unwrap();
        assert_eq!(handle.locale(), Locale::Es);
        assert!(!handle.settings().auto_update);

        // And it is on disk, not only in the mutex: a second handle over the same
        // directory is what the next launch of the app amounts to.
        let reopened = Handle::new(dir.path().to_path_buf(), None);
        assert_eq!(reopened.locale(), Locale::Es);
        assert!(!reopened.settings().auto_update);
    }

    #[test]
    fn locale_tags_are_the_ones_the_frontend_uses() {
        // The tag crosses the command boundary as a string and is looked up in a
        // TypeScript record. A serde rename here silently breaks that lookup.
        assert_eq!(serde_json::to_string(&Locale::Ja).unwrap(), "\"ja\"");
        assert_eq!(Locale::ALL.len(), 6);
    }
}
