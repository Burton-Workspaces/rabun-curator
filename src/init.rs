use std::path::Path;

const AGENTS: &str = include_str!("../templates/vault/AGENTS.md");
const INDEX: &str = include_str!("../templates/vault/index.md");
const LOG: &str = include_str!("../templates/vault/log.md");

pub fn run(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(path)?;
    for dir in [
        "raw",
        "wiki/entities",
        "wiki/concepts",
        "wiki/sources",
        "wiki/comparisons",
    ] {
        std::fs::create_dir_all(path.join(dir))?;
        let gitkeep = path.join(dir).join(".gitkeep");
        if !gitkeep.exists() {
            std::fs::write(gitkeep, "")?;
        }
    }

    write_if_missing(path.join("AGENTS.md"), AGENTS)?;
    write_if_missing(path.join("index.md"), INDEX)?;
    write_if_missing(path.join("log.md"), LOG)?;

    println!("Initialized curator wiki at {}", path.display());
    println!("  raw/     immutable sources");
    println!("  wiki/    LLM-owned pages");
    println!("  index.md catalog");
    println!("  log.md   append-only operations log");
    println!("  AGENTS.md schema");
    println!();
    println!("Next: drop a source into raw/ and run /curator ingest");
    Ok(())
}

fn write_if_missing(
    path: std::path::PathBuf,
    contents: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if path.exists() {
        println!("Keeping existing {}", path.display());
        return Ok(());
    }
    std::fs::write(path, contents)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_creates_karpathy_layout() {
        let dir = tempdir().unwrap();
        run(dir.path()).unwrap();
        assert!(dir.path().join("raw").is_dir());
        assert!(dir.path().join("wiki/entities").is_dir());
        assert!(dir.path().join("wiki/concepts").is_dir());
        assert!(dir.path().join("wiki/sources").is_dir());
        assert!(dir.path().join("AGENTS.md").is_file());
        assert!(dir.path().join("index.md").is_file());
        assert!(dir.path().join("log.md").is_file());
        let agents = std::fs::read_to_string(dir.path().join("AGENTS.md")).unwrap();
        assert!(agents.contains("raw/"));
        assert!(agents.contains("immutable"));
    }
}
