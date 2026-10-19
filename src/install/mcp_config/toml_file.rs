//! Codex's `config.toml`: add or remove `[mcp_servers.redline]` with `toml_edit`, keeping the user's
//! formatting, comments and any managed-block markers in place.

use super::{NAME, mcp_command, read_backup, write_with_backup};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

/// Adds or removes `[mcp_servers.redline]`, preserving the rest of the file's formatting.
pub(crate) fn edit_toml(path: &Path, add: bool) -> Result<bool> {
    use toml_edit::{Array, DocumentMut, Item, Table, value};
    let text = fs::read_to_string(path).unwrap_or_default();
    let mut doc: DocumentMut = text.parse().with_context(|| format!("parsing {}", path.display()))?;
    let servers = doc.entry("mcp_servers").or_insert_with(|| {
        let mut t = Table::new();
        t.set_implicit(true);
        Item::Table(t)
    });
    let servers = servers.as_table_mut().context("mcp_servers isn't a table")?;
    let changed = if add {
        if servers.contains_key(NAME) {
            false
        } else {
            let mut t = Table::new();
            t["command"] = value(mcp_command().to_string_lossy().as_ref());
            t["args"] = value(Array::from_iter(["mcp"]));
            servers.insert(NAME, Item::Table(t));
            true
        }
    } else if let Some(item) = servers.remove(NAME) {
        // Hand back comments we carried above our table (see below) to the end of the file.
        let carried = item.as_table().and_then(|t| t.decor().prefix()).and_then(|p| p.as_str()).map(str::trim);
        if let Some(c) = carried.filter(|c| !c.is_empty()) {
            let rest = doc.trailing().as_str().unwrap_or_default().to_string();
            doc.set_trailing(format!("{c}\n{rest}"));
        }
        true
    } else {
        false
    };
    if !changed {
        return Ok(false);
    }
    if add {
        // A comment at the end of the file (e.g. a tool's `# END managed block` marker) would otherwise end
        // up *below* our table, pulling it into that block. Keep such trailing text above it.
        let trailing = doc.trailing().as_str().unwrap_or_default().to_string();
        if !trailing.trim().is_empty() {
            let t = doc["mcp_servers"][NAME].as_table_mut().context("redline entry vanished")?;
            t.decor_mut().set_prefix(format!("{}\n\n", trailing.trim_end()));
            doc.set_trailing("\n");
        }
    }
    let out = doc.to_string();
    let restored = read_backup(path).filter(|b| same_toml(b, &out));
    write_with_backup(path, &restored.unwrap_or(out))?;
    Ok(true)
}

/// Same TOML content, ignoring formatting.
fn same_toml(a: &str, b: &str) -> bool {
    let norm = |s: &str| {
        s.parse::<toml_edit::DocumentMut>().ok().map(|mut d| {
            d.fmt();
            d.to_string()
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
                .collect::<Vec<_>>()
                .join("\n")
        })
    };
    norm(a).is_some_and(|x| Some(x) == norm(b))
}
