use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rmcp::handler::server::router::tool::ToolRouter;

use crate::cache::VaultCache;
use crate::vault;

pub const DEFAULT_LINK_STOPLIST: &[&str] = &[
    "index",
    "readme",
    "agents",
    "claude",
    "log",
    "skill",
    "changelog",
];

#[derive(Clone)]
pub struct CuratorServer {
    pub roots: Vec<PathBuf>,
    pub link_stoplist: Vec<String>,
    pub cache: Arc<Mutex<VaultCache>>,
    #[allow(dead_code)]
    pub tool_router: ToolRouter<Self>,
}

impl CuratorServer {
    pub fn build_link_stoplist(roots: &[PathBuf]) -> Vec<String> {
        let mut stop: std::collections::HashSet<String> = DEFAULT_LINK_STOPLIST
            .iter()
            .map(|s| s.to_string())
            .collect();
        for root in roots {
            if let Ok(contents) = std::fs::read_to_string(root.join(".rabun-curatorstoplist")) {
                for line in contents.lines() {
                    let term = line.trim();
                    if !term.is_empty() && !term.starts_with('#') {
                        stop.insert(term.to_lowercase());
                    }
                }
            }
        }
        stop.into_iter().collect()
    }

    pub fn all_md_files(&self) -> Vec<PathBuf> {
        vault::all_md_files(&self.roots)
    }

    pub fn relative_path(&self, abs: &Path) -> String {
        vault::relative_path(&self.roots, abs)
    }

    pub fn resolve_path(&self, rel: &str) -> Result<PathBuf, String> {
        vault::resolve_path(&self.roots, rel)
    }

    pub fn vault_files(&self) -> Vec<crate::wiki::VaultFile> {
        self.all_md_files()
            .into_iter()
            .filter_map(|path| {
                let content = std::fs::read_to_string(&path).ok()?;
                let stem = path.file_stem()?.to_string_lossy().to_string();
                Some(crate::wiki::VaultFile {
                    rel: self.relative_path(&path),
                    stem,
                    content,
                })
            })
            .collect()
    }
}
