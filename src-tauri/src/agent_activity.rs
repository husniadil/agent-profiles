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

/// Claude Code keeps one directory per project under its root, so its sessions
/// can be listed individually. Verified against a real installation; Codex's
/// layout has not been, so it stays a single row rather than a guessed one —
/// the same rule `Locations` follows for a platform nobody has checked.
const CLAUDE_PROJECTS: &str = ".claude/projects";

/// How far back a session is still worth listing.
///
/// The list answers "is the agent I care about seen, and is it seen as
/// working". A session nobody has touched in a week answers neither, and on a
/// real machine eleven of them buried the two that did.
const RECENT: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// The most sessions shown at once, newest first.
///
/// Capping the display cannot hide a working session from the decision: the
/// newest N always contains every session fresh enough to count as working.
const MAX_SESSIONS: usize = 6;

/// How much of a transcript is read to find the directory it runs in.
///
/// `cwd` sits in the first record or two — measured at byte 704 on a real
/// session — so this is one small read, not a parse of a file that reaches
/// megabytes.
const LABEL_PROBE_BYTES: usize = 8192;

/// How many transcripts in a project are tried before giving up on a label.
///
/// The newest file in a project is sometimes a short one carrying no `cwd` at
/// all, which is how two real projects ended up labelled with their raw slug.
const LABEL_PROBE_FILES: usize = 3;

/// How much of a transcript's tail is read to find the last conversation
/// record.
///
/// Measured against every transcript on a real machine, from 100KB to 6.7MB:
/// 8KB already reached the last conversation record in all of them, and 64KB
/// gave the identical verdict. The margin is for the metadata records that can
/// carry payloads — `file-history-snapshot` holds file contents — and can pile
/// up at the end of a file after the agent has stopped.
const TURN_PROBE_BYTES: u64 = 65_536;

/// The one `stop_reason` that means the agent handed control back.
///
/// Recognising the end rather than the middle is deliberate: 2714 of the 3088
/// assistant records on a real machine carry `tool_use`, and a `stop_reason`
/// nobody has seen yet must not be mistaken for "finished". Anything
/// unrecognised leaves the agent mid-turn, which errs toward holding the
/// machine awake — the direction where being wrong costs power rather than
/// somebody's work.
const END_TURN: &str = "end_turn";

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

/// How a root's state can be read.
///
/// Freshness alone cannot answer "is an agent working". A transcript is written
/// when a turn *ends* as well as while it runs, so the moment an agent hands
/// control back its file is zero seconds old — the signal peaks exactly when the
/// work stops. Merely resuming a session touches it too: measured on this
/// machine, a session whose last message ended 83 minutes earlier was stamped
/// one second after its process started, and read as working for the next five
/// minutes with no work behind it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Reading {
    /// A Claude Code project directory. The transcript carries the agent's own
    /// verdict — the last conversation record says whether the turn is over —
    /// so that is read and freshness only bounds it.
    Transcript,
    /// Anything whose layout has not been verified against a real installation.
    /// Freshness is all there is, exactly as before.
    Mtime,
}

pub struct Root {
    pub label: String,
    pub path: PathBuf,
    pub reading: Reading,
}

/// One root, how long ago anything under it was last written, and whether the
/// agent there is part-way through a turn.
#[derive(Serialize, Clone, PartialEq, Eq, Debug)]
pub struct Freshness {
    pub label: String,
    pub path: String,
    /// `None` means nothing has ever been written under this root, or it does
    /// not exist. Never confused with zero.
    pub seconds_ago: Option<u64>,
    /// `false` only when the transcript positively says the turn ended. A root
    /// read by mtime alone, or one whose transcript could not be understood, is
    /// `true` — which leaves it behaving exactly as it did before this existed.
    pub mid_turn: bool,
}

/// The directory slug, shortened from the left.
///
/// Only reached when no transcript in a project carries a `cwd` — an older
/// Claude Code wrote some without one. The slug encodes an absolute path, so
/// its tail identifies the project while its head is the same home directory on
/// every row; cutting the head is the one shortening that loses nothing.
fn shortened_slug(slug: &str) -> String {
    const KEEP: usize = 26;
    let count = slug.chars().count();
    if count <= KEEP {
        return slug.to_string();
    }
    format!("…{}", slug.chars().skip(count - KEEP).collect::<String>())
}

/// The directory a session runs in, taken from the head of its transcript.
///
/// Claude Code records `cwd` on nearly every entry, and the folder is what a
/// person actually calls the thing they are working on. The alternative is the
/// directory slug — `-Users-yudha-Documents--develop-Personal-AMBBU-AMS` —
/// which cannot be reversed, because `-` there stands for both a path separator
/// and a literal hyphen in a name.
///
/// Two segments, not one. A real machine listed `Source` twice, from
/// `VMMP2025/Source` and `PMOP.net/Source`, which is exactly the ambiguity this
/// list exists to remove.
fn label_from_transcript(transcript: &Path) -> Option<String> {
    use std::io::Read;

    let mut head = vec![0u8; LABEL_PROBE_BYTES];
    let read = std::fs::File::open(transcript).ok()?.read(&mut head).ok()?;
    let text = String::from_utf8_lossy(&head[..read]);

    // A capped read almost always cuts its last line in half, so a line that
    // will not parse is skipped rather than ending the search.
    let cwd = text
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|record| Some(record.get("cwd")?.as_str()?.to_string()))?;

    let parts: Vec<_> = Path::new(&cwd)
        .components()
        .filter_map(|c| match c {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect();
    match parts.len() {
        0 => None,
        1 => Some(parts[0].clone()),
        n => Some(format!("{}/{}", parts[n - 2], parts[n - 1])),
    }
}

/// The newest transcript in a Claude Code project directory.
///
/// Only the files directly inside it, matching `claude_sessions`: the
/// `subagents` folder below belongs to a session already counted by its parent.
fn newest_transcript(project: &Path) -> Option<PathBuf> {
    std::fs::read_dir(project)
        .ok()?
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|ext| ext == "jsonl"))
        .filter_map(|file| Some((file.path().metadata().ok()?.modified().ok()?, file.path())))
        .max_by_key(|(at, _)| *at)
        .map(|(_, path)| path)
}

/// Whether the agent in this transcript is part-way through a turn.
///
/// `None` when the file says nothing either way — unreadable, or holding no
/// conversation record in its tail — so the caller can fall back rather than
/// invent an answer.
///
/// The *last conversation record* decides, not the last record. A transcript
/// ends with metadata far more often than with a turn: fourteen record types
/// were seen on a real machine and most are sidecars written at times unrelated
/// to the work — `custom-title`, `mode`, `pr-link`, `queue-operation`,
/// `last-prompt`, `file-history-snapshot`. A live session mid-task ended with
/// `custom-title`; classifying that would have read the busiest agent on the
/// machine as neither working nor idle.
///
/// A trailing `user` record counts as mid-turn whether it is a person's prompt
/// or a tool result being handed back: both mean the agent's next move is owed.
fn mid_turn(transcript: &Path) -> Option<bool> {
    use std::io::{Read, Seek, SeekFrom};

    let mut file = std::fs::File::open(transcript).ok()?;
    let len = file.metadata().ok()?.len();
    let mut tail = Vec::new();
    // Read from the end, not the start: these files reach several megabytes,
    // and the answer is always in the last few lines.
    file.seek(SeekFrom::Start(len.saturating_sub(TURN_PROBE_BYTES)))
        .ok()?;
    file.read_to_end(&mut tail).ok()?;
    // Lossy rather than a `str` parse: a seek into the middle of a file lands
    // mid-character as often as not, and one mangled byte at the front must not
    // cost the whole read.
    let text = String::from_utf8_lossy(&tail);

    // Backwards, so the first conversation record found is the last one written.
    // The leading partial line simply fails to parse and is skipped, and it is
    // reached last anyway.
    text.lines()
        .rev()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .find_map(|record| match record.get("type")?.as_str()? {
            "assistant" => Some(
                record
                    .get("message")
                    .and_then(|message| message.get("stop_reason"))
                    .and_then(|reason| reason.as_str())
                    != Some(END_TURN),
            ),
            "user" => Some(true),
            _ => None,
        })
}

/// One row per Claude Code project, newest first.
///
/// Replaces a single "Claude Code" row that aggregated every session on the
/// machine while being drawn as though it described one. Stopping the session
/// you were watching left it green, on behalf of a session in another project
/// that the row had no way to name.
fn claude_sessions(root: &Path, now: SystemTime) -> Vec<Root> {
    let Ok(entries) = std::fs::read_dir(root) else {
        return Vec::new();
    };

    let mut found: Vec<(Duration, Root)> = entries
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter_map(|entry| {
            let project = entry.path();
            // Only the transcripts directly inside the project directory. The
            // `subagents` folder beneath it belongs to a session already counted
            // by its parent, and would otherwise list the same work twice.
            let mut transcripts: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(&project)
                .ok()?
                .flatten()
                .filter(|file| file.path().extension().is_some_and(|ext| ext == "jsonl"))
                .filter_map(|file| {
                    Some((file.path().metadata().ok()?.modified().ok()?, file.path()))
                })
                .collect();
            transcripts.sort_by_key(|(at, _)| std::cmp::Reverse(*at));

            let (newest, _) = transcripts.first()?;
            let age = now.duration_since(*newest).unwrap_or_default();
            if age > RECENT {
                return None;
            }

            let label = transcripts
                .iter()
                .take(LABEL_PROBE_FILES)
                .find_map(|(_, path)| label_from_transcript(path))
                .unwrap_or_else(|| shortened_slug(&entry.file_name().to_string_lossy()));

            Some((
                age,
                Root {
                    label,
                    path: project,
                    reading: Reading::Transcript,
                },
            ))
        })
        .collect();

    found.sort_by_key(|(age, _)| *age);
    found.truncate(MAX_SESSIONS);
    found.into_iter().map(|(_, root)| root).collect()
}

pub fn cli_roots(home: &Path) -> Vec<Root> {
    let mut roots = claude_sessions(&home.join(CLAUDE_PROJECTS), SystemTime::now());
    // Everything else stays one row. Listing a layout nobody has verified would
    // be inventing it, and one honest row beats several wrong ones.
    roots.extend(
        CLI_ROOTS
            .iter()
            .filter(|(_, rest)| *rest != CLAUDE_PROJECTS)
            .map(|(label, rest)| Root {
                label: (*label).to_string(),
                path: home.join(rest),
                reading: Reading::Mtime,
            }),
    );
    roots
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
            mid_turn: match root.reading {
                Reading::Mtime => true,
                Reading::Transcript => newest_transcript(&root.path)
                    .and_then(|transcript| mid_turn(&transcript))
                    .unwrap_or(true),
            },
        })
        .collect()
}

/// Whether any watched root holds an agent that is part-way through a turn and
/// still writing.
///
/// Both halves are needed. Without `mid_turn` every finished session held the
/// machine for the whole window, because a transcript is written when a turn
/// ends. Without the window a session abandoned mid-tool-call holds it forever:
/// one on this machine had been sitting in that state for three and a half
/// hours.
pub fn any_working(freshness: &[Freshness], window: Duration) -> bool {
    freshness
        .iter()
        .any(|root| root.mid_turn && root.seconds_ago.is_some_and(|ago| ago <= window.as_secs()))
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

    /// A root as `scan` would report it, so the decision can be tested without
    /// a filesystem behind it.
    fn seen(seconds_ago: Option<u64>, mid_turn: bool) -> Vec<Freshness> {
        vec![Freshness {
            label: "Claude Code".into(),
            path: "/x".into(),
            seconds_ago,
            mid_turn,
        }]
    }

    #[test]
    fn an_agent_counts_as_working_only_inside_the_window() {
        assert!(any_working(&seen(Some(30), true), Duration::from_secs(300)));
        assert!(!any_working(&seen(Some(30), true), Duration::from_secs(10)));
    }

    #[test]
    fn a_finished_turn_releases_the_machine_at_once() {
        // The defect this replaces, measured on a real machine: a session whose
        // last message ended 83 minutes earlier was stamped one second ago —
        // resuming it rewrote the transcript — and held the Mac awake for the
        // whole window with no work behind it. Freshness peaks exactly when the
        // work stops, so freshness alone can never see the end of a turn.
        assert!(!any_working(
            &seen(Some(1), false),
            Duration::from_secs(300)
        ));
    }

    #[test]
    fn a_session_abandoned_mid_turn_is_still_bounded_by_the_window() {
        // One on this machine had been sitting mid-tool-call for three and a
        // half hours. `mid_turn` alone would hold the Mac awake for as long as
        // the file sits there.
        assert!(!any_working(
            &seen(Some(12_993), true),
            Duration::from_secs(120)
        ));
    }

    #[test]
    fn a_root_with_no_activity_at_all_never_arms_the_trigger() {
        // `None` is "nothing has ever been written here", which must not be
        // mistaken for "written just now".
        assert!(!any_working(&seen(None, true), Duration::from_secs(86_400)));
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
                reading: Reading::Mtime,
            },
            Root {
                label: "Absent".into(),
                path: d.path().join("nope"),
                reading: Reading::Mtime,
            },
        ];
        let seen = scan(&roots, SystemTime::now());
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[1].label, "Absent");
        assert_eq!(seen[1].seconds_ago, None);
    }

    #[test]
    fn a_home_with_no_claude_sessions_still_watches_the_other_agent() {
        let roots = cli_roots(Path::new("/Users/h"));
        let paths: Vec<_> = roots.iter().map(|r| r.path.clone()).collect();
        assert!(paths.contains(&PathBuf::from("/Users/h/.codex/sessions")));
    }

    /// Writes one Claude Code project directory holding one transcript.
    fn project(home: &Path, slug: &str, cwd: Option<&str>, now: SystemTime, ago: Duration) {
        let dir = home.join(".claude/projects").join(slug);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("11111111-2222-3333-4444-555555555555.jsonl");
        let body = match cwd {
            Some(cwd) => format!("{{\"type\":\"user\",\"cwd\":\"{cwd}\"}}\n"),
            None => "{\"type\":\"summary\"}\n".to_string(),
        };
        std::fs::write(&file, body).unwrap();
        let when = std::fs::FileTimes::new().set_modified(now - ago);
        std::fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_times(when)
            .unwrap();
    }

    #[test]
    fn each_claude_project_is_its_own_row() {
        // The defect this replaces: one "Claude Code" row aggregated every
        // session on the machine, so stopping the one you were watching left it
        // green on behalf of a session in another project.
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        project(
            d.path(),
            "-a",
            Some("/Users/y/work/alpha"),
            now,
            Duration::from_secs(2),
        );
        project(
            d.path(),
            "-b",
            Some("/Users/y/work/beta"),
            now,
            Duration::from_secs(90),
        );

        let labels: Vec<_> = cli_roots(d.path())
            .iter()
            .map(|r| r.label.clone())
            .collect();
        assert!(labels.contains(&"work/alpha".to_string()), "got {labels:?}");
        assert!(labels.contains(&"work/beta".to_string()), "got {labels:?}");
    }

    #[test]
    fn two_projects_ending_in_the_same_folder_stay_distinguishable() {
        // Seen on a real machine: `VMMP2025/Source` and `PMOP.net/Source` both
        // reduced to "Source", which is the exact ambiguity this list exists to
        // remove.
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        project(
            d.path(),
            "-v",
            Some("/Users/y/VMMP2025/Source"),
            now,
            Duration::from_secs(5),
        );
        project(
            d.path(),
            "-p",
            Some("/Users/y/PMOP.net/Source"),
            now,
            Duration::from_secs(6),
        );

        let labels: Vec<_> = cli_roots(d.path())
            .iter()
            .map(|r| r.label.clone())
            .collect();
        assert!(
            labels.contains(&"VMMP2025/Source".to_string()),
            "got {labels:?}"
        );
        assert!(
            labels.contains(&"PMOP.net/Source".to_string()),
            "got {labels:?}"
        );
    }

    #[test]
    fn the_newest_session_is_listed_first() {
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        project(
            d.path(),
            "-a",
            Some("/Users/y/w/older"),
            now,
            Duration::from_secs(3600),
        );
        project(
            d.path(),
            "-b",
            Some("/Users/y/w/newer"),
            now,
            Duration::from_secs(5),
        );

        assert_eq!(cli_roots(d.path())[0].label, "w/newer");
    }

    #[test]
    fn a_session_nobody_has_touched_in_a_week_is_not_listed() {
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        project(
            d.path(),
            "-old",
            Some("/Users/y/w/ancient"),
            now,
            RECENT + Duration::from_secs(60),
        );

        let labels: Vec<_> = cli_roots(d.path())
            .iter()
            .map(|r| r.label.clone())
            .collect();
        assert!(!labels.contains(&"w/ancient".to_string()), "got {labels:?}");
    }

    #[test]
    fn no_more_than_a_screenful_of_sessions_is_listed() {
        let d = tempfile::tempdir().unwrap();
        let now = SystemTime::now();
        for i in 0..12 {
            project(
                d.path(),
                &format!("-p{i}"),
                Some(&format!("/Users/y/w/p{i}")),
                now,
                Duration::from_secs(i + 1),
            );
        }
        // Capping the display cannot hide a working session: the newest N always
        // contains every session fresh enough to count.
        assert_eq!(cli_roots(d.path()).len() - 1, MAX_SESSIONS);
    }

    #[test]
    fn a_transcript_with_no_cwd_falls_back_to_its_directory_name() {
        // The slug cannot be reversed — `-` is both a separator and a literal —
        // so it is shown as-is rather than mangled into a wrong guess.
        let d = tempfile::tempdir().unwrap();
        project(
            d.path(),
            "-Users-y-thing",
            None,
            SystemTime::now(),
            Duration::from_secs(5),
        );

        let labels: Vec<_> = cli_roots(d.path())
            .iter()
            .map(|r| r.label.clone())
            .collect();
        assert!(
            labels.contains(&"-Users-y-thing".to_string()),
            "got {labels:?}"
        );
    }

    /// Writes one transcript holding exactly these JSONL records, and returns
    /// the project directory it lives in.
    fn transcript(records: &[&str]) -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(
            d.path().join("s.jsonl"),
            format!("{}\n", records.join("\n")),
        )
        .unwrap();
        d
    }

    const ENDED: &str =
        r#"{"type":"assistant","message":{"role":"assistant","stop_reason":"end_turn"}}"#;
    const CALLING: &str =
        r#"{"type":"assistant","message":{"role":"assistant","stop_reason":"tool_use"}}"#;
    const ASKED: &str = r#"{"type":"user","message":{"role":"user"}}"#;

    #[test]
    fn a_finished_turn_reads_as_finished() {
        let d = transcript(&[CALLING, ENDED]);
        assert_eq!(mid_turn(&d.path().join("s.jsonl")), Some(false));
    }

    #[test]
    fn an_agent_waiting_on_a_tool_is_still_working() {
        // The property freshness cannot have. A tool call that runs for minutes
        // writes nothing while it runs and its process sits idle — measured on
        // this machine, a working session and an idle one both read 1.4% CPU —
        // but the transcript still says the agent's next move is owed.
        let d = transcript(&[ENDED, ASKED, CALLING]);
        assert_eq!(mid_turn(&d.path().join("s.jsonl")), Some(true));
    }

    #[test]
    fn a_prompt_with_no_answer_yet_is_working() {
        let d = transcript(&[ENDED, ASKED]);
        assert_eq!(mid_turn(&d.path().join("s.jsonl")), Some(true));
    }

    #[test]
    fn metadata_written_after_the_turn_ended_does_not_hide_it() {
        // The first version of this read the *last* record and got the busiest
        // agent on the machine wrong: a live session mid-task ended with
        // `custom-title`. Fourteen record types were seen on a real machine and
        // most are sidecars written at times unrelated to the work.
        let d = transcript(&[
            ENDED,
            r#"{"type":"custom-title","title":"x"}"#,
            r#"{"type":"last-prompt"}"#,
            r#"{"type":"queue-operation"}"#,
        ]);
        assert_eq!(mid_turn(&d.path().join("s.jsonl")), Some(false));
    }

    #[test]
    fn a_transcript_with_no_conversation_in_it_says_nothing_either_way() {
        // `None`, not `false`. The caller falls back to freshness alone, which
        // is how this degrades to its old behaviour rather than to "idle".
        let d = transcript(&[r#"{"type":"summary"}"#]);
        assert_eq!(mid_turn(&d.path().join("s.jsonl")), None);
    }

    #[test]
    fn the_verdict_is_found_in_the_tail_of_a_file_far_bigger_than_the_probe() {
        // Real transcripts reach 6.7MB. Reading them whole on a fifteen-second
        // sweep is not an option, and seeking into the middle of one lands
        // mid-character.
        let d = tempfile::tempdir().unwrap();
        let filler = format!(
            "{}\n",
            r#"{"type":"user","message":{"role":"user","text":"— padding ——"}}"#
        );
        let mut body = filler.repeat(4000);
        assert!(body.len() as u64 > TURN_PROBE_BYTES * 2, "{}", body.len());
        body.push_str(&format!("{ENDED}\n"));
        std::fs::write(d.path().join("s.jsonl"), &body).unwrap();

        assert_eq!(mid_turn(&d.path().join("s.jsonl")), Some(false));
    }

    #[test]
    fn a_claude_project_is_scanned_by_its_transcript_and_a_codex_root_is_not() {
        // Same directory, same freshness, opposite verdicts — the whole point.
        let ended = transcript(&[ENDED]);
        let working = transcript(&[CALLING]);
        let roots = vec![
            Root {
                label: "ended".into(),
                path: ended.path().to_path_buf(),
                reading: Reading::Transcript,
            },
            Root {
                label: "working".into(),
                path: working.path().to_path_buf(),
                reading: Reading::Transcript,
            },
            Root {
                // An unverified layout keeps behaving exactly as it did.
                label: "codex".into(),
                path: ended.path().to_path_buf(),
                reading: Reading::Mtime,
            },
        ];

        let seen = scan(&roots, SystemTime::now());
        assert!(!seen[0].mid_turn, "a finished turn must read as finished");
        assert!(seen[1].mid_turn);
        assert!(
            seen[2].mid_turn,
            "mtime-only roots are never called finished"
        );
        assert!(any_working(&seen, Duration::from_secs(120)));
        assert!(!any_working(&seen[..1], Duration::from_secs(120)));
    }

    #[test]
    fn a_long_slug_fallback_is_cut_from_the_left_not_the_right() {
        // The head of a slug is the same home directory on every row; the tail
        // is what names the project. A real machine produced
        // `-Users-yudha-Documents--develop-PMOP-net-Source`, which ran past its
        // row and pushed the age out of the column.
        let long = "-Users-yudha-Documents--develop-PMOP-net-Source";
        let short = shortened_slug(long);
        assert!(short.starts_with('…'), "got {short}");
        assert!(
            short.ends_with("PMOP-net-Source"),
            "the tail must survive: {short}"
        );
        assert!(short.chars().count() < long.chars().count());

        // Anything that already fits is left exactly as it is.
        assert_eq!(shortened_slug("-Users-y-thing"), "-Users-y-thing");
    }
}
