use rmcp::{
    handler::server::wrapper::Parameters, model::*, schemars, tool, tool_handler, tool_router,
    ErrorData as McpError, ServerHandler,
};
use serde::{Deserialize, Serialize};

use crate::graph;
use crate::links;
use crate::report;
use crate::server::CuratorServer;
use crate::vault;
use crate::viz;
use crate::wiki;

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchParams {
    /// Text query to search for
    pub query: String,
    /// Maximum number of results (default 20)
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PathParams {
    /// Relative path within the vault
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WriteParams {
    /// Relative path within the vault
    pub path: String,
    /// Markdown content to write
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ListParams {
    /// Subdirectory to list (omit for vault root)
    pub directory: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct TraverseParams {
    /// Note title (file stem) to start from
    pub start: String,
    /// Maximum hops (default 2)
    pub depth: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct ShortestPathParams {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, schemars::JsonSchema)]
pub struct OutputPathParams {
    /// Output path within the vault
    pub output_path: Option<String>,
}

fn json_text(value: &serde_json::Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".into())
}

impl CuratorServer {
    pub fn new_tool_router() -> rmcp::handler::server::router::tool::ToolRouter<Self> {
        Self::tool_router()
    }
}

fn snippet_for(content: &str, query: &str) -> String {
    let lower = content.to_lowercase();
    let q = query.to_lowercase();
    if let Some(pos) = lower.find(&q) {
        let start = pos.saturating_sub(80);
        let end = (pos + q.len() + 80).min(content.len());
        content[start..end].replace('\n', " ")
    } else {
        content
            .chars()
            .take(160)
            .collect::<String>()
            .replace('\n', " ")
    }
}

#[tool_router]
impl CuratorServer {
    #[tool(
        description = "Search the wiki and raw sources by text query. Uses an in-memory trigram + BM25 index."
    )]
    async fn search(&self, params: Parameters<SearchParams>) -> Result<String, McpError> {
        let limit = params.0.limit.unwrap_or(20);
        let query = params.0.query;
        let cache = self.cache.lock().unwrap();
        let matches = cache.search_index.search(&query, limit);
        drop(cache);
        let results: Vec<_> = matches
            .iter()
            .map(|(path, content, score)| {
                serde_json::json!({
                    "path": self.relative_path(path),
                    "snippet": snippet_for(content, &query),
                    "score": format!("{:.2}", score),
                })
            })
            .collect();
        Ok(json_text(&serde_json::Value::Array(results)))
    }

    #[tool(description = "Read a vault file by relative path. Works for wiki/ and raw/.")]
    async fn read(&self, params: Parameters<PathParams>) -> Result<String, McpError> {
        match self.resolve_path(&params.0.path) {
            Ok(full) => match std::fs::read_to_string(&full) {
                Ok(content) => Ok(content),
                Err(e) => Ok(format!("Error reading {}: {e}", params.0.path)),
            },
            Err(e) => Ok(format!("Error: {e}")),
        }
    }

    #[tool(
        description = "Write a wiki page (not raw/). Auto-links mentions of existing notes as [[wikilinks]]. Creates parent directories."
    )]
    async fn write(&self, params: Parameters<WriteParams>) -> Result<String, McpError> {
        if vault::is_under_raw(&params.0.path) {
            return Ok(
                "Error: raw/ is immutable. Curator never writes source files. Put LLM-owned pages under wiki/."
                    .into(),
            );
        }
        let full = match self.resolve_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let titles = {
            let cache = self.cache.lock().unwrap();
            cache.titles.clone()
        };
        let (linked_content, links_added) = links::auto_link(
            &params.0.content,
            &params.0.path,
            &titles,
            &self.link_stoplist,
        );
        match std::fs::write(&full, &linked_content) {
            Ok(_) => {
                if let Ok(mut cache) = self.cache.lock() {
                    cache.update_single_file(&full, &linked_content, self);
                }
                let link_msg = if links_added.is_empty() {
                    String::new()
                } else {
                    format!(", auto-linked: {}", links_added.join(", "))
                };
                Ok(format!(
                    "Written: {} ({} bytes{})",
                    params.0.path,
                    linked_content.len(),
                    link_msg
                ))
            }
            Err(e) => Ok(format!("Error writing {}: {e}", params.0.path)),
        }
    }

    #[tool(
        description = "List files and directories in the vault. Omit directory for the vault root."
    )]
    async fn list(&self, params: Parameters<ListParams>) -> Result<String, McpError> {
        match vault::list_dir(&self.roots, params.0.directory.as_deref()) {
            Ok(entries) => Ok(entries.join("\n")),
            Err(e) => Ok(format!("Error: {e}")),
        }
    }

    #[tool(description = "Get backlinks (files linking TO this note) and outgoing [[wikilinks]].")]
    async fn links(&self, params: Parameters<PathParams>) -> Result<String, McpError> {
        let full = match self.resolve_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        let stem = full
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
        let cache = self.cache.lock().unwrap();
        let outgoing = cache.outgoing.get(&stem).cloned().unwrap_or_default();
        let incoming = cache.incoming.get(&stem).cloned().unwrap_or_default();
        Ok(json_text(&serde_json::json!({
            "path": params.0.path,
            "outgoing": outgoing,
            "backlinks": incoming,
        })))
    }

    #[tool(description = "Read YAML frontmatter from a vault file.")]
    async fn metadata(&self, params: Parameters<PathParams>) -> Result<String, McpError> {
        let full = match self.resolve_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        match std::fs::read_to_string(&full) {
            Ok(content) => match links::extract_frontmatter(&content) {
                Some(fm) => Ok(fm),
                None => Ok("(no frontmatter)".into()),
            },
            Err(e) => Ok(format!("Error: {e}")),
        }
    }

    #[tool(description = "Suggest [[wikilinks]] for a file without modifying it.")]
    async fn suggest_links(&self, params: Parameters<PathParams>) -> Result<String, McpError> {
        let full = match self.resolve_path(&params.0.path) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        let content = match std::fs::read_to_string(&full) {
            Ok(c) => c,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        let titles = {
            let cache = self.cache.lock().unwrap();
            cache.titles.clone()
        };
        let (_linked, added) =
            links::auto_link(&content, &params.0.path, &titles, &self.link_stoplist);
        Ok(json_text(&serde_json::json!({
            "path": params.0.path,
            "suggestions": added,
        })))
    }

    #[tool(description = "Vault statistics: file count, words, links, orphans.")]
    async fn stats(&self) -> Result<String, McpError> {
        let files = self.all_md_files();
        let cache = self.cache.lock().unwrap();
        let mut words = 0usize;
        let mut links = 0usize;
        for targets in cache.outgoing.values() {
            links += targets.len();
        }
        for (_path, content) in &cache.search_index.files {
            words += content.split_whitespace().count();
        }
        let mut orphans = Vec::new();
        for path in &files {
            if let Some(stem) = path.file_stem() {
                let stem = stem.to_string_lossy().to_string();
                let rel = self.relative_path(path);
                if vault::is_under_raw(&rel) {
                    continue;
                }
                if graph::is_orphan_stem(&stem, &cache.outgoing, &cache.incoming) {
                    orphans.push(rel);
                }
            }
        }
        orphans.sort();
        Ok(json_text(&serde_json::json!({
            "files": files.len(),
            "words": words,
            "links": links,
            "orphan_count": orphans.len(),
            "orphans": orphans,
            "vaults": self.roots.iter().map(|p| p.display().to_string()).collect::<Vec<_>>(),
        })))
    }

    #[tool(
        description = "BFS traversal from a note, N hops deep. Explores the topic neighborhood."
    )]
    async fn traverse(&self, params: Parameters<TraverseParams>) -> Result<String, McpError> {
        let depth = params.0.depth.unwrap_or(2);
        let cache = self.cache.lock().unwrap();
        let rows = graph::traverse(&cache.outgoing, &cache.incoming, &params.0.start, depth);
        if rows.is_empty() {
            return Ok(format!("No note named '{}' in the graph.", params.0.start));
        }
        let results: Vec<_> = rows
            .into_iter()
            .map(|(name, distance, neighbors)| {
                serde_json::json!({
                    "note": name,
                    "distance": distance,
                    "neighbors": neighbors,
                })
            })
            .collect();
        Ok(json_text(&serde_json::Value::Array(results)))
    }

    #[tool(description = "Shortest [[wikilink]] chain between two notes.")]
    async fn shortest_path(
        &self,
        params: Parameters<ShortestPathParams>,
    ) -> Result<String, McpError> {
        let cache = self.cache.lock().unwrap();
        match graph::shortest_path(
            &cache.outgoing,
            &cache.incoming,
            &params.0.from,
            &params.0.to,
        ) {
            Some(path) => Ok(json_text(&serde_json::json!({ "path": path }))),
            None => Ok(format!(
                "No path between '{}' and '{}'.",
                params.0.from, params.0.to
            )),
        }
    }

    #[tool(description = "Detect topic communities with Louvain modularity optimization.")]
    async fn cluster(&self) -> Result<String, McpError> {
        let cache = self.cache.lock().unwrap();
        let (community_of, communities) =
            graph::detect_communities(&cache.outgoing, &cache.incoming);
        let listed: Vec<_> = communities
            .iter()
            .enumerate()
            .map(|(i, members)| {
                serde_json::json!({
                    "id": i,
                    "size": members.len(),
                    "label": members.first().cloned().unwrap_or_default(),
                    "members": members,
                })
            })
            .collect();
        Ok(json_text(&serde_json::json!({
            "communities": listed,
            "assignments": community_of,
        })))
    }

    #[tool(
        description = "Write an interactive HTML graph visualization (nodes colored by community, sized by importance)."
    )]
    async fn visualize(&self, params: Parameters<OutputPathParams>) -> Result<String, McpError> {
        let rel = params
            .0
            .output_path
            .unwrap_or_else(|| "GRAPH_VIZ.html".into());
        if vault::is_under_raw(&rel) {
            return Ok("Error: cannot write visualization under raw/.".into());
        }
        let full = match self.resolve_path(&rel) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        let cache = self.cache.lock().unwrap();
        let (community_of, communities) =
            graph::detect_communities(&cache.outgoing, &cache.incoming);
        let gods = graph::god_nodes(&cache.outgoing, &cache.incoming, 25);
        let vault_name = self
            .roots
            .first()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "wiki".into());
        let html = viz::generate_html(
            &vault_name,
            &cache.outgoing,
            &community_of,
            &communities,
            &gods,
        );
        drop(cache);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&full, html) {
            Ok(_) => Ok(format!("Wrote graph visualization: {rel}")),
            Err(e) => Ok(format!("Error writing {rel}: {e}")),
        }
    }

    #[tool(
        description = "Write GRAPH_REPORT.md with god nodes, communities, bridges, and suggested questions."
    )]
    async fn report(&self, params: Parameters<OutputPathParams>) -> Result<String, McpError> {
        let rel = params
            .0
            .output_path
            .unwrap_or_else(|| "GRAPH_REPORT.md".into());
        if vault::is_under_raw(&rel) {
            return Ok("Error: cannot write report under raw/.".into());
        }
        let full = match self.resolve_path(&rel) {
            Ok(p) => p,
            Err(e) => return Ok(format!("Error: {e}")),
        };
        let cache = self.cache.lock().unwrap();
        let (community_of, communities) =
            graph::detect_communities(&cache.outgoing, &cache.incoming);
        let gods = graph::god_nodes(&cache.outgoing, &cache.incoming, 15);
        let bc = graph::betweenness_centrality(&cache.outgoing, &cache.incoming);
        let surprising = graph::surprising_connections(&cache.outgoing, &community_of, &bc, 10);
        let mut orphan_count = 0usize;
        let mut nodes: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut edges = 0usize;
        for (k, vs) in &cache.outgoing {
            nodes.insert(k.clone());
            edges += vs.len();
            for v in vs {
                nodes.insert(v.clone());
            }
            if graph::is_orphan_stem(k, &cache.outgoing, &cache.incoming) {
                orphan_count += 1;
            }
        }
        let vault_name = self
            .roots
            .first()
            .and_then(|p| p.file_name())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "wiki".into());
        let md = report::generate_report(
            &vault_name,
            nodes.len(),
            edges,
            &communities,
            &gods,
            &surprising,
            orphan_count,
            &community_of,
        );
        drop(cache);
        if let Some(parent) = full.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match std::fs::write(&full, md) {
            Ok(_) => Ok(format!("Wrote graph report: {rel}")),
            Err(e) => Ok(format!("Error writing {rel}: {e}")),
        }
    }

    #[tool(
        description = "Structural wiki lint: orphans, broken wikilinks, pages missing from index.md, raw sources without wiki/sources summaries, pages never mentioned in log.md."
    )]
    async fn lint(&self) -> Result<String, McpError> {
        let files = self.vault_files();
        let cache = self.cache.lock().unwrap();
        let report = wiki::lint(&files, &cache.outgoing, &cache.incoming);
        Ok(json_text(&serde_json::json!({
            "orphans": report.orphans,
            "broken_wikilinks": report.broken_wikilinks.iter().map(|(p, t)| {
                serde_json::json!({"page": p, "target": t})
            }).collect::<Vec<_>>(),
            "wiki_pages_missing_from_index": report.wiki_pages_missing_from_index,
            "raw_without_source_summary": report.raw_without_source_summary,
            "pages_never_logged": report.pages_never_logged,
        })))
    }
}

#[tool_handler]
impl ServerHandler for CuratorServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build()).with_instructions(
            "Curator maintains a Karpathy LLM wiki. raw/ is immutable. Write only wiki pages, index.md, and log.md. Prefer search then read. After ingest, update index.md, append log.md, and run lint.",
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::VaultCache;
    use std::sync::{Arc, Mutex};

    #[test]
    fn advertises_tools_capability() {
        let server = CuratorServer {
            roots: vec![],
            link_stoplist: vec![],
            cache: Arc::new(Mutex::new(VaultCache::default())),
            tool_router: CuratorServer::new_tool_router(),
        };
        assert!(
            server.get_info().capabilities.tools.is_some(),
            "server must advertise the tools capability"
        );
    }

    #[test]
    fn raw_write_is_rejected_by_helper() {
        assert!(vault::is_under_raw("raw/paper.md"));
    }

    #[test]
    fn indexes_example_wiki() {
        let root =
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/minimal-wiki");
        let server = CuratorServer {
            roots: vec![root],
            link_stoplist: vec![],
            cache: Arc::new(Mutex::new(VaultCache::default())),
            tool_router: CuratorServer::new_tool_router(),
        };
        let cache = VaultCache::build_full(&server);
        assert!(cache.search_index.total_docs >= 7);
        let hits = cache.search_index.search("persistent wiki", 10);
        assert!(!hits.is_empty());
        assert!(cache
            .outgoing
            .get("Persistent Wiki")
            .map(|v| !v.is_empty())
            .unwrap_or(false));
    }
}
