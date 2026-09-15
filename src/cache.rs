use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::links::{extract_aliases, extract_wikilinks, Title};
use crate::search::SearchIndex;
use crate::server::CuratorServer;

#[derive(Default)]
pub struct VaultCache {
    pub search_index: SearchIndex,
    pub outgoing: HashMap<String, Vec<String>>,
    pub incoming: HashMap<String, Vec<String>>,
    pub titles: Vec<Title>,
    file_mtimes: HashMap<PathBuf, SystemTime>,
}

impl VaultCache {
    pub fn build_full(server: &CuratorServer) -> VaultCache {
        let mut search_files = Vec::new();
        let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
        let mut incoming: HashMap<String, Vec<String>> = HashMap::new();
        let mut titles = Vec::new();
        let mut file_mtimes = HashMap::new();

        for path in server.all_md_files() {
            let stem = match path.file_stem() {
                Some(s) => s.to_string_lossy().to_string(),
                None => continue,
            };
            let rel = server.relative_path(&path);
            let Ok(content) = std::fs::read_to_string(&path) else {
                continue;
            };
            if let Ok(meta) = std::fs::metadata(&path) {
                if let Ok(mtime) = meta.modified() {
                    file_mtimes.insert(path.clone(), mtime);
                }
            }
            search_files.push((path.clone(), content.clone()));
            outgoing.entry(stem.clone()).or_default();
            for link in extract_wikilinks(&content) {
                outgoing.entry(stem.clone()).or_default().push(link.clone());
                incoming.entry(link).or_default().push(stem.clone());
            }
            titles.push((stem.clone(), stem.clone(), rel.clone()));
            for alias in extract_aliases(&content) {
                titles.push((alias, stem.clone(), rel.clone()));
            }
        }

        eprintln!("Curator: indexed {} markdown files", search_files.len());
        VaultCache {
            search_index: SearchIndex::build(&search_files),
            outgoing,
            incoming,
            titles,
            file_mtimes,
        }
    }

    pub fn update_single_file(&mut self, path: &Path, content: &str, server: &CuratorServer) {
        self.remove_file_entries(path, server);
        self.add_file_entries(path, content, server);
        if let Ok(meta) = std::fs::metadata(path) {
            if let Ok(mtime) = meta.modified() {
                self.file_mtimes.insert(path.to_path_buf(), mtime);
            }
        }
    }

    fn remove_file_entries(&mut self, path: &Path, server: &CuratorServer) {
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            return;
        };
        let rel = server.relative_path(path);
        if let Some(targets) = self.outgoing.remove(&stem) {
            for target in &targets {
                if let Some(sources) = self.incoming.get_mut(target) {
                    sources.retain(|s| s != &stem);
                    if sources.is_empty() {
                        self.incoming.remove(target);
                    }
                }
            }
        }
        self.incoming.remove(&stem);
        self.titles.retain(|(_, _, r)| r != &rel);
        self.search_index.remove_file(path);
    }

    fn add_file_entries(&mut self, path: &Path, content: &str, server: &CuratorServer) {
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_string()) else {
            return;
        };
        let rel = server.relative_path(path);
        self.outgoing.entry(stem.clone()).or_default();
        for link in extract_wikilinks(content) {
            self.outgoing
                .entry(stem.clone())
                .or_default()
                .push(link.clone());
            self.incoming.entry(link).or_default().push(stem.clone());
        }
        self.titles.push((stem.clone(), stem.clone(), rel.clone()));
        for alias in extract_aliases(content) {
            self.titles.push((alias, stem.clone(), rel.clone()));
        }
        self.search_index.add_file(path, content);
    }
}
