use std::collections::HashSet;

use crate::graph;
use crate::links::extract_wikilinks;
use crate::vault::is_under_raw;

#[derive(Debug, Clone)]
pub struct VaultFile {
    pub rel: String,
    pub stem: String,
    pub content: String,
}

#[derive(Debug, Default)]
pub struct LintReport {
    pub orphans: Vec<String>,
    pub broken_wikilinks: Vec<(String, String)>,
    pub wiki_pages_missing_from_index: Vec<String>,
    pub raw_without_source_summary: Vec<String>,
    pub pages_never_logged: Vec<String>,
}

fn is_schema_file(rel: &str) -> bool {
    matches!(
        rel,
        "index.md" | "log.md" | "AGENTS.md" | "CLAUDE.md" | "GRAPH_REPORT.md"
    ) || rel.eq_ignore_ascii_case("readme.md")
}

pub fn lint(
    files: &[VaultFile],
    outgoing: &std::collections::HashMap<String, Vec<String>>,
    incoming: &std::collections::HashMap<String, Vec<String>>,
) -> LintReport {
    let stems: HashSet<String> = files.iter().map(|f| f.stem.clone()).collect();
    let index = files.iter().find(|f| f.rel == "index.md");
    let log = files.iter().find(|f| f.rel == "log.md");
    let index_text = index.map(|f| f.content.as_str()).unwrap_or("");
    let log_text = log.map(|f| f.content.as_str()).unwrap_or("");

    let mut report = LintReport::default();

    for file in files {
        if is_schema_file(&file.rel) || is_under_raw(&file.rel) {
            continue;
        }
        if graph::is_orphan_stem(&file.stem, outgoing, incoming) {
            report.orphans.push(file.rel.clone());
        }
        for target in extract_wikilinks(&file.content) {
            let target_stem = target
                .rsplit('/')
                .next()
                .unwrap_or(&target)
                .trim_end_matches(".md");
            if !stems.iter().any(|s| s.eq_ignore_ascii_case(target_stem)) {
                report.broken_wikilinks.push((file.rel.clone(), target));
            }
        }
    }
    report.orphans.sort();

    for file in files {
        if !file.rel.starts_with("wiki/") {
            continue;
        }
        if is_schema_file(&file.rel) {
            continue;
        }
        let in_index = index_text.contains(&file.rel)
            || index_text.contains(&format!("[[{}]]", file.stem))
            || index_text.contains(&format!("[[{}|", file.stem));
        if !in_index {
            report.wiki_pages_missing_from_index.push(file.rel.clone());
        }
        let logged = log_text.contains(&file.rel) || log_text.contains(&file.stem);
        if !logged {
            report.pages_never_logged.push(file.rel.clone());
        }
    }
    report.wiki_pages_missing_from_index.sort();
    report.pages_never_logged.sort();

    let source_stems: HashSet<String> = files
        .iter()
        .filter(|f| f.rel.starts_with("wiki/sources/"))
        .map(|f| f.stem.to_lowercase())
        .collect();
    for file in files {
        if !is_under_raw(&file.rel) || file.rel.ends_with(".gitkeep") {
            continue;
        }
        if !source_stems.contains(&file.stem.to_lowercase()) {
            report.raw_without_source_summary.push(file.rel.clone());
        }
    }
    report.raw_without_source_summary.sort();
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn lint_finds_structural_issues() {
        let files = vec![
            VaultFile {
                rel: "index.md".into(),
                stem: "index".into(),
                content: "- [[Persistent Wiki]]\n".into(),
            },
            VaultFile {
                rel: "log.md".into(),
                stem: "log".into(),
                content: "## [2026-09-15] ingest | LLM Wiki\n".into(),
            },
            VaultFile {
                rel: "wiki/concepts/Persistent Wiki.md".into(),
                stem: "Persistent Wiki".into(),
                content: "See [[Missing Page]]\n".into(),
            },
            VaultFile {
                rel: "wiki/concepts/Orphan Idea.md".into(),
                stem: "Orphan Idea".into(),
                content: "lonely\n".into(),
            },
            VaultFile {
                rel: "raw/LLM Wiki.md".into(),
                stem: "LLM Wiki".into(),
                content: "source\n".into(),
            },
        ];
        let mut outgoing = HashMap::new();
        outgoing.insert("Persistent Wiki".into(), vec!["Missing Page".into()]);
        outgoing.insert("Orphan Idea".into(), vec![]);
        let mut incoming = HashMap::new();
        incoming.insert("Persistent Wiki".into(), vec![]);
        incoming.insert("Orphan Idea".into(), vec![]);

        let report = lint(&files, &outgoing, &incoming);
        assert!(report.orphans.iter().any(|p| p.contains("Orphan Idea")));
        assert!(report
            .broken_wikilinks
            .iter()
            .any(|(_, t)| t == "Missing Page"));
        assert!(report
            .wiki_pages_missing_from_index
            .iter()
            .any(|p| p.contains("Orphan Idea")));
        assert!(report
            .raw_without_source_summary
            .iter()
            .any(|p| p.contains("LLM Wiki")));
        assert!(report
            .pages_never_logged
            .iter()
            .any(|p| p.contains("Orphan Idea")));
    }
}
