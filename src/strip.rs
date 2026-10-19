//! `redline strip`: remove embed snippets from an artifact before it ships.
//!
//! Removes `<!-- redline:begin -->…<!-- redline:end -->` blocks (what `register` emits) and any
//! leftover `<script>` tag that loads redline (`data-redline-*`, or `/client.js` / `/overlay.js`
//! from a loopback origin). Text-based, so it works on HTML, JSX, templates and Markdown alike.

use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const BEGIN: &str = "<!-- redline:begin";
pub const END: &str = "<!-- redline:end -->";

/// File types that can hold a snippet. Others are skipped when walking a directory.
const EXTENSIONS: &[&str] = &[
    "html", "htm", "xhtml", "jsx", "tsx", "vue", "svelte", "astro", "md", "mdx", "php", "erb", "ejs", "hbs", "njk",
    "liquid",
];
/// Directories never worth walking into.
const SKIP_DIRS: &[&str] = &["node_modules", "target", "dist", "build", "__pycache__", "venv"];

/// `text` without redline snippets, and how many were removed.
#[must_use]
pub fn strip(text: &str) -> (String, usize) {
    let (text, blocks) = strip_blocks(text);
    let (text, tags) = strip_tags(&text);
    (text, blocks + tags)
}

fn strip_blocks(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let (mut rest, mut n) = (text, 0);
    while let Some(b) = rest.find(BEGIN) {
        let Some(e) = rest[b..].find(END).map(|e| b + e + END.len()) else { break };
        out.push_str(trim_line_start(&rest[..b]));
        rest = skip_newline(&rest[e..]);
        n += 1;
    }
    out.push_str(rest);
    (out, n)
}

fn strip_tags(text: &str) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let (mut rest, mut n) = (text, 0);
    while let Some(b) = find_ci(rest, "<script") {
        let open_end = rest[b..].find('>').map_or(rest.len(), |i| b + i + 1);
        let close = find_ci(&rest[open_end..], "</script>").map(|i| open_end + i + "</script>".len());
        let (Some(e), true) = (close, is_redline_tag(&rest[b..open_end])) else {
            out.push_str(&rest[..open_end]);
            rest = &rest[open_end..];
            continue;
        };
        out.push_str(trim_line_start(&rest[..b]));
        rest = skip_newline(&rest[e..]);
        n += 1;
    }
    out.push_str(rest);
    (out, n)
}

fn is_redline_tag(open: &str) -> bool {
    let o = open.to_ascii_lowercase();
    let loopback = ["//127.0.0.1:", "//localhost:", "//[::1]:"].iter().any(|h| o.contains(h));
    o.contains("data-redline-") || (loopback && (o.contains("/client.js") || o.contains("/overlay.js")))
}

fn find_ci(hay: &str, needle: &str) -> Option<usize> {
    hay.to_ascii_lowercase().find(needle)
}

/// Drops indentation before a removed snippet, so no blank-but-indented line is left.
fn trim_line_start(before: &str) -> &str {
    let trimmed = before.trim_end_matches([' ', '\t']);
    if trimmed.is_empty() || trimmed.ends_with('\n') { trimmed } else { before }
}

fn skip_newline(after: &str) -> &str {
    after.strip_prefix("\r\n").or_else(|| after.strip_prefix('\n')).unwrap_or(after)
}

/// Files under `paths` (files as given; directories walked, filtered by extension).
pub fn files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = vec![];
    for p in paths {
        if p.is_dir() { walk(p, &mut out)? } else { out.push(p.clone()) }
    }
    Ok(out)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if path.is_dir() {
            if !name.starts_with('.') && !SKIP_DIRS.contains(&name) {
                walk(&path, out)?;
            }
        } else if path.extension().and_then(|e| e.to_str()).is_some_and(|e| EXTENSIONS.contains(&e)) {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = "<!doctype html>\n<body>\n  <p>Hi</p>\n  <!-- redline:begin (review only; remove with `redline strip`) -->\n  <script src=\"http://127.0.0.1:7465/client.js\" data-redline-artifact=\"art_1\" data-redline-token=\"abc\"></script>\n  <script src=\"http://127.0.0.1:7465/overlay.js\"></script>\n  <!-- redline:end -->\n</body>\n";

    #[test]
    fn strips_a_marked_block_cleanly() {
        let (out, n) = strip(PAGE);
        assert_eq!(n, 1);
        assert_eq!(out, "<!doctype html>\n<body>\n  <p>Hi</p>\n</body>\n");
        assert_eq!(strip(&out), (out.clone(), 0), "idempotent");
    }

    #[test]
    fn strips_unmarked_redline_tags_only() {
        let page = "<head><script src=\"/app.js\"></script>\n<SCRIPT src=\"http://localhost:7465/overlay.js\"></SCRIPT>\n</head>\n<script data-redline-artifact=\"a\" src=\"x\"></script><p>keep</p>\n<script>const s = '/client.js';</script>\n";
        let (out, n) = strip(page);
        assert_eq!(n, 2);
        assert_eq!(
            out,
            "<head><script src=\"/app.js\"></script>\n</head>\n<p>keep</p>\n<script>const s = '/client.js';</script>\n"
        );
    }

    #[test]
    fn leaves_unterminated_markers_alone() {
        let page = "<!-- redline:begin -->\n<p>no end marker</p>\n";
        assert_eq!(strip(page), (page.to_string(), 0));
    }

    #[test]
    fn walks_dirs_by_extension_and_skips_vendored() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path();
        for f in ["a.html", "b.tsx", "c.rs", "node_modules/d.html", ".git/e.html", "sub/f.md"] {
            let path = p.join(f);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, "x").unwrap();
        }
        let mut got: Vec<_> = files(&[p.to_path_buf()])
            .unwrap()
            .iter()
            .map(|f| f.strip_prefix(p).unwrap().display().to_string().replace('\\', "/"))
            .collect();
        got.sort();
        assert_eq!(got, ["a.html", "b.tsx", "sub/f.md"]);
    }
}
