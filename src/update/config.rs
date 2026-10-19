//! `config.toml`, the `[update]` table: whether to check for releases, and versions the user skipped.
//! Edits go through `toml_edit`, so the user's comments and layout survive.

use anyhow::{Context, Result, bail};
use std::fs;
use std::path::Path;
use toml_edit::{Array, DocumentMut, Item, Table, value};

/// Written when redline creates the file, so a user who opens it knows what each key does.
const TEMPLATE: &str = "\
# redline settings. Edit by hand, or with `redline update off|on|skip`.

[update]
# Check for a new release once a day and show a notice (overlay, CLI, agent). false: no checks, no notices.
check = true
# Releases not to be notified about. A newer release still shows.
skip = []
";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateConfig {
    pub check: bool,
    pub skip: Vec<String>,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self { check: true, skip: vec![] }
    }
}

/// The `[update]` settings; defaults when the file or a key is missing.
pub fn load(path: &Path) -> Result<UpdateConfig> {
    let Ok(text) = fs::read_to_string(path) else { return Ok(UpdateConfig::default()) };
    let doc: DocumentMut = text.parse().with_context(|| format!("parsing {}", path.display()))?;
    let mut c = UpdateConfig::default();
    let Some(t) = doc.get("update") else { return Ok(c) };
    if let Some(v) = t.get("check") {
        c.check = v.as_bool().with_context(|| format!("{}: update.check must be true or false", path.display()))?;
    }
    if let Some(v) = t.get("skip") {
        let arr = v.as_array().with_context(|| format!("{}: update.skip must be a list", path.display()))?;
        c.skip = arr.iter().filter_map(|s| s.as_str().map(str::to_string)).collect();
    }
    Ok(c)
}

/// Sets `update.check`.
pub fn set_check(path: &Path, on: bool) -> Result<()> {
    edit(path, |t| {
        t["check"] = value(on);
        Ok(())
    })
}

/// Adds `version` to `update.skip` (once).
pub fn skip(path: &Path, version: &str) -> Result<()> {
    edit(path, |t| {
        let skip = t.entry("skip").or_insert_with(|| value(Array::new()));
        let arr = skip.as_array_mut().context("update.skip must be a list")?;
        if !arr.iter().any(|v| v.as_str() == Some(version)) {
            arr.push(version);
        }
        Ok(())
    })
}

/// Loads the file (or the template), changes `[update]`, and writes it back atomically.
fn edit(path: &Path, f: impl FnOnce(&mut Table) -> Result<()>) -> Result<()> {
    let text = fs::read_to_string(path).unwrap_or_else(|_| TEMPLATE.to_string());
    let mut doc: DocumentMut = text.parse().with_context(|| format!("parsing {}", path.display()))?;
    let Some(t) = doc.entry("update").or_insert_with(|| Item::Table(Table::new())).as_table_mut() else {
        bail!("{}: [update] must be a table", path.display());
    };
    f(t)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("toml.tmp");
    fs::write(&tmp, doc.to_string())?;
    fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_means_defaults() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load(&dir.path().join("none.toml")).unwrap(), UpdateConfig::default());
    }

    #[test]
    fn first_edit_writes_the_commented_template() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("redline/config.toml");
        skip(&p, "0.3.0").unwrap();
        skip(&p, "0.3.0").unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.contains("# Releases not to be notified about"), "{text}");
        assert_eq!(load(&p).unwrap(), UpdateConfig { check: true, skip: vec!["0.3.0".into()] });
    }

    #[test]
    fn edits_keep_the_users_comments_and_other_tables() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        fs::write(&p, "# mine\n[other]\nx = 1 # keep\n").unwrap();
        set_check(&p, false).unwrap();
        skip(&p, "1.0.0").unwrap();
        let text = fs::read_to_string(&p).unwrap();
        assert!(text.starts_with("# mine\n[other]\nx = 1 # keep\n"), "{text}");
        assert_eq!(load(&p).unwrap(), UpdateConfig { check: false, skip: vec!["1.0.0".into()] });
    }

    #[test]
    fn wrong_types_are_reported() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("config.toml");
        fs::write(&p, "[update]\ncheck = \"no\"\n").unwrap();
        assert!(format!("{:#}", load(&p).unwrap_err()).contains("update.check"));
    }
}
