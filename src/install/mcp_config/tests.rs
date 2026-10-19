use super::*;

#[test]
fn set_entry_adds_updates_removes() {
    let mut m = Map::new();
    assert!(set_entry(&mut m, "mcpServers", Some(json!({"a": 1}))));
    assert!(!set_entry(&mut m, "mcpServers", Some(json!({"a": 1}))), "same value is a no-op");
    assert!(set_entry(&mut m, "mcpServers", Some(json!({"a": 2}))));
    assert!(set_entry(&mut m, "mcpServers", None));
    assert!(!m.contains_key("mcpServers"), "an emptied section goes too");
    assert!(!set_entry(&mut m, "mcpServers", None));
    m.insert("mcpServers".into(), json!({}));
    assert!(!set_entry(&mut m, "mcpServers", None));
    assert!(m.contains_key("mcpServers"), "a user's own empty section stays");
}

#[test]
fn edit_json_keeps_other_servers_and_backs_up_once() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("mcp.json");
    fs::write(&p, r#"{"mcpServers":{"other":{"command":"x"}},"theme":"dark"}"#).unwrap();

    assert!(edit_json(&p, |m| set_entry(m, "mcpServers", Some(server_json()))).unwrap());
    let v: Value = serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
    assert!(v["mcpServers"]["other"].is_object());
    assert!(v["mcpServers"][NAME].is_object());
    assert_eq!(v["theme"], "dark");

    let bak = dir.path().join("mcp.json.redline.bak");
    let original = fs::read_to_string(&bak).unwrap();
    assert!(!original.contains(NAME));
    assert!(edit_json(&p, |m| set_entry(m, "mcpServers", None)).unwrap());
    assert_eq!(fs::read_to_string(&bak).unwrap(), original, "backup is taken only once");
}

#[test]
fn edit_json_refuses_jsonc() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("opencode.json");
    fs::write(&p, "{ // comment\n }").unwrap();
    assert!(edit_json(&p, |m| set_entry(m, "mcp", Some(opencode_json()))).is_err());
    assert_eq!(fs::read_to_string(&p).unwrap(), "{ // comment\n }");
}

#[test]
fn edit_toml_preserves_formatting() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let original = "# my settings\nmodel = \"o3\"\n\n[mcp_servers.other]\ncommand = \"x\"\n";
    fs::write(&p, original).unwrap();

    assert!(edit_toml(&p, true).unwrap());
    assert!(!edit_toml(&p, true).unwrap());
    let text = fs::read_to_string(&p).unwrap();
    assert!(text.starts_with("# my settings\nmodel = \"o3\""));
    assert!(text.contains("[mcp_servers.redline]"));

    assert!(edit_toml(&p, false).unwrap());
    assert_eq!(fs::read_to_string(&p).unwrap(), original);
}

#[test]
fn edit_json_treats_empty_file_as_empty_object() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("mcp_config.json");
    fs::write(&p, "").unwrap();
    assert!(edit_json(&p, |m| set_entry(m, "mcpServers", Some(server_json()))).unwrap());
    let v: Value = serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
    assert!(v["mcpServers"][NAME].is_object());
}

// Exact entry shapes, per the agents' docs (checked 2026-10-15):
// - Codex: https://learn.chatgpt.com/docs/extend/mcp?surface=cli, https://learn.chatgpt.com/docs/config-file/config-advanced
// - opencode: https://opencode.ai/docs/config/, https://opencode.ai/docs/mcp-servers/
// - Cursor: https://cursor.com/docs/context/mcp
// - Gemini CLI: https://geminicli.com/docs/tools/mcp-server/
// - Antigravity: https://antigravity.google/docs/mcp

#[test]
fn servers_json_shape_gemini_agy() {
    let cmd = mcp_command().to_string_lossy().into_owned();
    assert_eq!(server_json(), json!({ "command": cmd, "args": ["mcp"] }));
}

#[test]
fn cursor_shape_has_stdio_type() {
    // https://cursor.com/docs/context/mcp lists `type` as required for stdio; cursor-agent
    // 2025.10.02 loads the server with or without it (checked 2026-10-16).
    let cmd = mcp_command().to_string_lossy().into_owned();
    assert_eq!(cursor_json(), json!({ "type": "stdio", "command": cmd, "args": ["mcp"] }));
}

#[test]
fn opencode_shape() {
    let cmd = mcp_command().to_string_lossy().into_owned();
    assert_eq!(opencode_json(), json!({ "type": "local", "command": [cmd, "mcp"], "enabled": true }));
}

#[test]
fn codex_toml_shape() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    assert!(edit_toml(&p, true).unwrap());
    let doc: toml_edit::DocumentMut = fs::read_to_string(&p).unwrap().parse().unwrap();
    let t = doc["mcp_servers"][NAME].as_table().unwrap();
    assert_eq!(t.len(), 2);
    assert_eq!(t["command"].as_str(), Some(mcp_command().to_string_lossy().as_ref()));
    let args: Vec<_> = t["args"].as_array().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert_eq!(args, ["mcp"]);
}

#[test]
fn edit_toml_keeps_trailing_markers_above_our_table() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    let original = "# BEGIN sync mcp\n[mcp_servers.other]\ncommand = \"x\"\n# END sync mcp\n";
    fs::write(&p, original).unwrap();
    assert!(edit_toml(&p, true).unwrap());
    let text = fs::read_to_string(&p).unwrap();
    let (end, ours) = (text.find("# END sync mcp").unwrap(), text.find("[mcp_servers.redline]").unwrap());
    assert!(end < ours, "our table must come after the managed block:\n{text}");
    text.parse::<toml_edit::DocumentMut>().unwrap();
    assert!(edit_toml(&p, false).unwrap());
    assert_eq!(fs::read_to_string(&p).unwrap(), original, "uninstall restores the file byte for byte");
}

#[test]
fn edit_json_keeps_key_order_and_uninstall_restores_the_original() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("opencode.json");
    let original = "{\n    \"model\": \"m\",\n    \"mcp\": { \"z\": {\"type\": \"local\", \"command\": [\"z\"]} },\n    \"agent\": {}\n}\n";
    fs::write(&p, original).unwrap();
    assert!(edit_json(&p, |m| set_entry(m, "mcp", Some(opencode_json()))).unwrap());
    let text = fs::read_to_string(&p).unwrap();
    let (model, mcp, agent) =
        (text.find("\"model\"").unwrap(), text.find("\"mcp\"").unwrap(), text.find("\"agent\"").unwrap());
    assert!(model < mcp && mcp < agent, "key order kept:\n{text}");
    assert!(text.find("\"z\"").unwrap() < text.find("\"redline\"").unwrap());
    assert!(edit_json(&p, |m| set_entry(m, "mcp", None)).unwrap());
    assert_eq!(fs::read_to_string(&p).unwrap(), original);
}

#[test]
fn edit_toml_keeps_the_marker_when_the_backup_no_longer_matches() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("config.toml");
    fs::write(&p, "[mcp_servers.other]\ncommand = \"x\"\n# END sync mcp\n").unwrap();
    assert!(edit_toml(&p, true).unwrap());
    let changed = fs::read_to_string(&p).unwrap().replace("command = \"x\"", "command = \"y\"");
    fs::write(&p, changed).unwrap();
    assert!(edit_toml(&p, false).unwrap());
    let text = fs::read_to_string(&p).unwrap();
    assert!(text.contains("command = \"y\"") && text.contains("# END sync mcp") && !text.contains(NAME), "{text}");
}
