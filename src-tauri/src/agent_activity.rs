//! Whether an agentic AI is working right now, answered from the filesystem.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Session roots for agent CLIs that keep their state outside any profile.
/// Home-relative.
///
/// Claude Code writes `projects/<slug>/<session>.jsonl` and appends on every
/// message and every tool result. Measured on a live session: the transcript
/// was two seconds stale while its agent was mid-task. That is the property
/// that makes this the right signal — it moves during a long network wait, when
/// the process itself looks perfectly idle and a CPU heuristic reads "done".
const CLI_ROOTS: &[(&str, &str)] = &[
    ("Claude Code", ".claude/projects"),
    ("Codex CLI", ".codex/sessions"),
];

/// How deep a session root is walked.
///
/// Both known layouts are `<root>/<project>/<session file>`, so two levels
/// reaches every transcript. A bound rather than an unlimited walk because this
/// runs on a timer, and an unlimited one would follow whatever a user happened
/// to leave in the folder.
///
/// ponytail: a flat re-stat of every transcript each sweep — around a thousand
/// `stat` calls on a heavily used machine, a few milliseconds. Directory mtimes
/// cannot prune it, because appending to a file does not touch its directory.
/// If this ever shows up in a profile, watch the roots with FSEvents instead.
const MAX_DEPTH: u32 = 2;

pub struct Root {
    pub label: String,
    pub path: PathBuf,
}

/// One root and how long ago anything under it was last written.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Freshness {
    pub label: String,
    pub path: String,
    /// `None` means nothing has ever been written under this root, or it does
    /// not exist. Never confused with zero.
    pub seconds_ago: Option<u64>,
}

pub fn cli_roots(home: &Path) -> Vec<Root> {
    CLI_ROOTS
        .iter()
        .map(|(label, rest)| Root {
            label: (*label).to_string(),
            path: home.join(rest),
        })
        .collect()
}

/// How long ago the newest file anywhere under `root` was modified.
///
/// `None` for a root that does not exist, holds no files, or cannot be read.
/// A file stamped in the future — a clock that moved, a copied archive — is
/// clamped to zero by `duration_since`'s error path rather than wrapping.
pub fn newest_age(root: &Path, now: SystemTime) -> Option<Duration> {
    fn newest(dir: &Path, depth: u32, best: &mut Option<SystemTime>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let Ok(meta) = entry.metadata() else { continue };
            if meta.is_dir() {
                if depth > 0 {
                    newest(&entry.path(), depth - 1, best);
                }
            } else if let Ok(modified) = meta.modified() {
                if best.is_none_or(|seen| modified > seen) {
                    *best = Some(modified);
                }
            }
        }
    }

    let mut best = None;
    newest(root, MAX_DEPTH, &mut best);
    best.map(|at| now.duration_since(at).unwrap_or_default())
}

pub fn scan(roots: &[Root], now: SystemTime) -> Vec<Freshness> {
    roots
        .iter()
        .map(|root| Freshness {
            label: root.label.clone(),
            path: root.path.display().to_string(),
            seconds_ago: newest_age(&root.path, now).map(|age| age.as_secs()),
        })
        .collect()
}

/// Whether any watched root has been written inside `window`.
pub fn any_within(freshness: &[Freshness], window: Duration) -> bool {
    freshness
        .iter()
        .any(|root| root.seconds_ago.is_some_and(|ago| ago <= window.as_secs()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sets a file's mtime to `ago` before `now`, so a test can describe a
    /// transcript that has gone quiet without waiting for it to.
    fn write_aged(path: &Path, now: SystemTime, ago: Duration) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"{}").unwrap();
        let when = std::fs::FileTimes::new().set_modified(now - ago);
        std::fs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_times(when)
            .unwrap();
    }

    #[test]
    fn a_transcript_written_moments_ago_reads_as_fresh() {
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        write_aged(
            &d.path().join("proj/session.jsonl"),
            now,
            Duration::from_secs(3),
        );

        let age = newest_age(d.path(), now).expect("a written file must have an age");
        assert!(age < Duration::from_secs(30), "got {age:?}");
    }

    #[test]
    fn the_newest_file_anywhere_under_the_root_is_the_one_that_counts() {
        // A user with fifty projects has one live session. The stale forty-nine
        // must not drag the answer down.
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        write_aged(
            &d.path().join("old/a.jsonl"),
            now,
            Duration::from_secs(86_400),
        );
        write_aged(&d.path().join("live/b.jsonl"), now, Duration::from_secs(4));

        let age = newest_age(d.path(), now).unwrap();
        assert!(age < Duration::from_secs(30), "got {age:?}");
    }

    #[test]
    fn a_root_that_does_not_exist_is_absent_rather_than_an_error() {
        // Most users have Claude Code or Codex, not both. A missing root is the
        // normal case and must not disable the trigger.
        let d = tempfile::tempdir().unwrap();
        assert!(newest_age(&d.path().join("never-created"), SystemTime::now()).is_none());
    }

    #[test]
    fn an_empty_root_is_absent_rather_than_infinitely_fresh() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("empty")).unwrap();
        assert!(newest_age(&d.path().join("empty"), SystemTime::now()).is_none());
    }

    #[test]
    fn an_agent_counts_as_working_only_inside_the_window() {
        let fresh = vec![Freshness {
            label: "Claude Code".into(),
            path: "/x".into(),
            seconds_ago: Some(30),
        }];
        assert!(any_within(&fresh, Duration::from_secs(300)));
        assert!(!any_within(&fresh, Duration::from_secs(10)));
    }

    #[test]
    fn a_root_with_no_activity_at_all_never_arms_the_trigger() {
        // `None` is "nothing has ever been written here", which must not be
        // mistaken for "written just now".
        let quiet = vec![Freshness {
            label: "Codex".into(),
            path: "/x".into(),
            seconds_ago: None,
        }];
        assert!(!any_within(&quiet, Duration::from_secs(86_400)));
    }

    #[test]
    fn every_watched_root_is_reported_even_when_it_is_missing() {
        // The window draws this list. A root that silently vanished from it
        // would read as "we are watching everything we said we would".
        let d = tempfile::tempdir().unwrap();
        let roots = vec![
            Root {
                label: "Present".into(),
                path: d.path().to_path_buf(),
            },
            Root {
                label: "Absent".into(),
                path: d.path().join("nope"),
            },
        ];
        let seen = scan(&roots, SystemTime::now());
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].label, "Absent");
        assert_eq!(seen[1].seconds_ago, None);
    }

    #[test]
    fn the_cli_roots_are_the_two_agents_that_write_transcripts() {
        let roots = cli_roots(Path::new("/Users/h"));
        let paths: Vec<_> = roots.iter().map(|r| r.path.clone()).collect();
        assert!(paths.contains(&PathBuf::from("/Users/h/.claude/projects")));
        assert!(paths.contains(&PathBuf::from("/Users/h/.codex/sessions")));
    }
}
