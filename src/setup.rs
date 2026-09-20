use std::path::{Path, PathBuf};

const APP_NAME: &str = env!("CARGO_PKG_NAME");
const SKILL_CONTENT: &str = include_str!("../skills/rabun-curator/SKILL.md");

pub fn find_binary_path() -> String {
    std::env::current_exe()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| APP_NAME.to_string())
}

fn claude_desktop_config_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        dirs::home_dir()
            .map(|h| h.join("Library/Application Support/Claude/claude_desktop_config.json"))
    }
    #[cfg(target_os = "linux")]
    {
        dirs::home_dir().map(|h| h.join(".config/Claude/claude_desktop_config.json"))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA")
            .ok()
            .map(|a| PathBuf::from(a).join("Claude/claude_desktop_config.json"))
    }
}

fn claude_code_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude/settings.json"))
}

fn grok_config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".grok/config.toml"))
}

fn cursor_mcp_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".cursor/mcp.json"))
}

fn mcp_server_json(binary: &str, vault_args: &[String]) -> serde_json::Value {
    serde_json::json!({
        "command": binary,
        "args": vault_args,
    })
}

fn upsert_json_mcp(
    config_path: &Path,
    binary: &str,
    vault_args: &[String],
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(parent) = config_path.parent() else {
        return Ok(false);
    };
    if !parent.exists() {
        return Ok(false);
    }
    let mut config: serde_json::Value = if config_path.exists() {
        let content = std::fs::read_to_string(config_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };
    if config_path.exists() {
        let backup = config_path.with_extension("json.bak");
        let _ = std::fs::copy(config_path, backup);
    }
    let servers = config
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert(serde_json::json!({}));
    servers
        .as_object_mut()
        .unwrap()
        .insert(APP_NAME.to_string(), mcp_server_json(binary, vault_args));
    std::fs::write(config_path, serde_json::to_string_pretty(&config)?)?;
    Ok(true)
}

fn upsert_grok_toml(
    config_path: &Path,
    binary: &str,
    vault_args: &[String],
) -> Result<bool, Box<dyn std::error::Error>> {
    let Some(parent) = config_path.parent() else {
        return Ok(false);
    };
    if !parent.exists() {
        return Ok(false);
    }
    let mut doc = if config_path.exists() {
        let backup = config_path.with_extension("toml.bak");
        let _ = std::fs::copy(config_path, backup);
        std::fs::read_to_string(config_path)?.parse::<toml_edit::DocumentMut>()?
    } else {
        toml_edit::DocumentMut::new()
    };
    if doc.get("mcp_servers").is_none() {
        doc["mcp_servers"] = toml_edit::table();
    }
    let mut args = toml_edit::Array::new();
    for arg in vault_args {
        args.push(arg.as_str());
    }
    doc["mcp_servers"][APP_NAME]["command"] = toml_edit::value(binary);
    doc["mcp_servers"][APP_NAME]["args"] = toml_edit::Item::Value(toml_edit::Value::Array(args));
    std::fs::write(config_path, doc.to_string())?;
    Ok(true)
}

fn install_skill(dir: PathBuf) -> Result<bool, Box<dyn std::error::Error>> {
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join("SKILL.md"), SKILL_CONTENT)?;
    Ok(true)
}

pub fn run_setup(vault_paths: &[PathBuf]) -> Result<(), Box<dyn std::error::Error>> {
    let binary = find_binary_path();
    let vault_args: Vec<String> = vault_paths
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let mut configured = Vec::new();

    if let Some(path) = claude_desktop_config_path() {
        match upsert_json_mcp(&path, &binary, &vault_args) {
            Ok(true) => configured.push(format!("Claude Desktop ({})", path.display())),
            Ok(false) => {}
            Err(e) => eprintln!("Warning: Claude Desktop config failed: {e}"),
        }
    }
    if let Some(path) = claude_code_config_path() {
        match upsert_json_mcp(&path, &binary, &vault_args) {
            Ok(true) => configured.push(format!("Claude Code ({})", path.display())),
            Ok(false) => {}
            Err(e) => eprintln!("Warning: Claude Code config failed: {e}"),
        }
    }
    if let Some(path) = grok_config_path() {
        match upsert_grok_toml(&path, &binary, &vault_args) {
            Ok(true) => configured.push(format!("Grok ({})", path.display())),
            Ok(false) => {}
            Err(e) => eprintln!("Warning: Grok config failed: {e}"),
        }
    }
    if let Some(path) = cursor_mcp_path() {
        match upsert_json_mcp(&path, &binary, &vault_args) {
            Ok(true) => configured.push(format!("Cursor ({})", path.display())),
            Ok(false) => {}
            Err(e) => eprintln!("Warning: Cursor MCP config failed: {e}"),
        }
    }

    if let Some(home) = dirs::home_dir() {
        for (label, dir) in [
            ("Claude skill", home.join(".claude/skills").join(APP_NAME)),
            ("Grok skill", home.join(".grok/skills").join(APP_NAME)),
            ("Cursor skill", home.join(".cursor/skills").join(APP_NAME)),
        ] {
            match install_skill(dir.clone()) {
                Ok(_) => configured.push(format!("{label} ({}/SKILL.md)", dir.display())),
                Err(e) => eprintln!("Warning: could not install {label}: {e}"),
            }
        }
    }

    if configured.is_empty() {
        eprintln!("No Claude, Grok, or Cursor config directories found.");
        eprintln!("Add this MCP server manually:\n");
        eprintln!("  command: {binary}");
        eprintln!("  args: {vault_args:?}");
        eprintln!("\nJSON clients:");
        let mut servers = serde_json::Map::new();
        servers.insert(APP_NAME.to_string(), mcp_server_json(&binary, &vault_args));
        let mut example = serde_json::Map::new();
        example.insert("mcpServers".to_string(), serde_json::Value::Object(servers));
        eprintln!("{}", serde_json::to_string_pretty(&example)?);
        eprintln!("\nGrok (~/.grok/config.toml):");
        eprintln!("[mcp_servers.{APP_NAME}]");
        eprintln!("command = \"{binary}\"");
        eprintln!("args = {vault_args:?}");
    } else {
        println!("{APP_NAME} configured for:");
        for target in &configured {
            println!("  {target}");
        }
        println!();
        println!(
            "Vault{}: {}",
            if vault_args.len() > 1 { "s" } else { "" },
            vault_args.join(", ")
        );
        println!();
        println!("Restart Claude / Grok / Cursor to connect the wiki.");
        println!(
            "Skill command: /{APP_NAME} (ingest, query, lint, search, graph, analyze, status)"
        );
    }

    Ok(())
}
