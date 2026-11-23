//! `redline wait`'s saved cursor: where the last wait with the same filter stopped, so a bare
//! `redline wait` returns only new comments (the MCP server keeps the same thing in memory).

use std::fs;
use std::path::{Path, PathBuf};

/// The cursor file for a wait filter, under `<state>/cursors/`.
pub fn file(state: &Path, artifact: Option<&str>, session: Option<&str>) -> PathBuf {
    let key = match (artifact, session) {
        (None, None) => "all".to_string(),
        (a, s) => format!("a-{}.s-{}", encode(a.unwrap_or("")), encode(s.unwrap_or(""))),
    };
    state.join("cursors").join(key)
}

/// Where to start: `since` if given, else the saved cursor, else 0. A saved cursor past `head` means
/// the event log was reset (a new state dir or database), so it starts over.
pub fn start(since: Option<i64>, saved: Option<i64>, head: impl FnOnce() -> Option<i64>) -> i64 {
    match (since, saved) {
        (Some(s), _) => s,
        (None, Some(c)) if head().is_none_or(|h| c <= h) => c,
        _ => 0,
    }
}

pub fn load(path: &Path) -> Option<i64> {
    fs::read_to_string(path).ok()?.trim().parse().ok()
}

/// Saves `next`. Best effort: a failed save only means the next bare wait repeats some events.
pub fn save(path: &Path, next: i64) {
    let write = || -> std::io::Result<()> {
        fs::create_dir_all(path.parent().unwrap_or(path))?;
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, next.to_string())?;
        fs::rename(tmp, path)
    };
    if let Err(e) = write() {
        tracing::debug!("saving wait cursor {}: {e}", path.display());
    }
}

/// Session and artifact ids are user-chosen strings: keep file-name-safe bytes, `%XX` the rest.
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-' => char::from(b).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_is_per_filter_and_safe() {
        let st = Path::new("/s");
        assert_eq!(file(st, None, None), Path::new("/s/cursors/all"));
        assert_eq!(file(st, None, Some("ses_1")), Path::new("/s/cursors/a-.s-ses_1"));
        assert_eq!(file(st, Some("art_1"), None), Path::new("/s/cursors/a-art_1.s-"));
        assert_eq!(file(st, None, Some("../x y")), Path::new("/s/cursors/a-.s-%2E%2E%2Fx%20y"));
        assert_ne!(file(st, Some("x"), None), file(st, None, Some("x")));
    }

    #[test]
    fn start_prefers_since_then_saved_then_zero() {
        assert_eq!(start(Some(3), Some(9), || Some(10)), 3);
        assert_eq!(start(None, Some(9), || Some(10)), 9);
        assert_eq!(start(None, None, || Some(10)), 0);
        assert_eq!(start(None, Some(9), || Some(4)), 0, "log reset: start over");
        assert_eq!(start(None, Some(9), || None), 9, "head unknown: trust the saved cursor");
    }

    #[test]
    fn save_then_load() {
        let dir = tempfile::tempdir().unwrap();
        let f = file(dir.path(), None, Some("dev"));
        assert_eq!(load(&f), None);
        save(&f, 42);
        assert_eq!(load(&f), Some(42));
        save(&f, 43);
        assert_eq!(load(&f), Some(43));
    }
}
