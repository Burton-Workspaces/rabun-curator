use std::collections::HashSet;

use regex::Regex;

/// (match_term, canonical stem, relative path)
pub type Title = (String, String, String);

pub fn extract_frontmatter(content: &str) -> Option<String> {
    if !content.starts_with("---\n") {
        return None;
    }
    content[4..]
        .find("\n---")
        .map(|end| content[4..4 + end].to_string())
}

pub fn extract_aliases(content: &str) -> Vec<String> {
    let fm = match extract_frontmatter(content) {
        Some(fm) => fm,
        None => return Vec::new(),
    };
    for line in fm.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("aliases:") {
            let rest = rest.trim();
            if rest.starts_with('[') {
                return rest
                    .trim_start_matches('[')
                    .trim_end_matches(']')
                    .split(',')
                    .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            if !rest.is_empty() {
                return vec![rest.trim_matches('"').trim_matches('\'').to_string()];
            }
        }
    }
    Vec::new()
}

pub fn extract_wikilinks(content: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]|]+)(?:\|[^\]]+)?\]\]").expect("wikilink regex");
    re.captures_iter(content)
        .map(|c| c[1].trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn find_exclusion_zones(text: &str) -> Vec<(usize, usize)> {
    let mut zones = Vec::new();
    let fenced = Regex::new(r"(?ms)^```[^\n]*\n.*?^```").expect("fenced");
    for m in fenced.find_iter(text) {
        zones.push((m.start(), m.end()));
    }
    let inline = Regex::new(r"`[^`]+`").expect("inline");
    for m in inline.find_iter(text) {
        zones.push((m.start(), m.end()));
    }
    let urls = Regex::new(r"https?://\S+").expect("urls");
    for m in urls.find_iter(text) {
        zones.push((m.start(), m.end()));
    }
    let wikilinks = Regex::new(r"\[\[[^\]]+\]\]").expect("existing links");
    for m in wikilinks.find_iter(text) {
        zones.push((m.start(), m.end()));
    }
    zones
}

fn top_folder(rel: &str) -> &str {
    rel.split('/').next().unwrap_or("")
}

/// Wrap unlinked mentions of existing note titles in `[[wikilinks]]`.
/// Skips code, URLs, existing links, and stoplisted stems.
pub fn auto_link(
    content: &str,
    exclude_path: &str,
    titles: &[Title],
    stoplist: &[String],
) -> (String, Vec<String>) {
    let existing_links = extract_wikilinks(content);
    let existing_set: HashSet<&str> = existing_links.iter().map(|s| s.as_str()).collect();
    let writing_dir = top_folder(exclude_path);

    let mut result = content.to_string();
    let mut links_added = Vec::new();

    let mut candidates: Vec<&Title> = titles
        .iter()
        .filter(|(match_term, canonical, rel)| {
            match_term.chars().count() >= 3
                && rel != exclude_path
                && !stoplist.iter().any(|s| s.eq_ignore_ascii_case(match_term))
                && top_folder(rel) != "raw"
                && writing_dir != "raw"
                && !existing_set.contains(canonical.as_str())
                && !existing_set.contains(match_term.as_str())
        })
        .collect();
    candidates.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

    let mut linked_stems: HashSet<String> = HashSet::new();

    for (match_term, canonical, _rel) in candidates {
        if linked_stems.contains(canonical) {
            continue;
        }
        let Ok(re) = Regex::new(&format!(r"(?i)\b{}\b", regex::escape(match_term))) else {
            continue;
        };

        let fm_end = if let Some(rest) = result.strip_prefix("---\n") {
            rest.find("\n---\n").map(|i| 4 + i + 5).unwrap_or(0)
        } else {
            0
        };
        let body_part = &result[fm_end..];
        if body_part.contains(&format!("[[{canonical}]]"))
            || body_part.contains(&format!("[[{canonical}|"))
        {
            continue;
        }

        let exclusion_zones = find_exclusion_zones(body_part);
        let mut search_start = 0;
        let found = loop {
            if search_start >= body_part.len() {
                break None;
            }
            match re.find(&body_part[search_start..]) {
                Some(m) => {
                    let abs_start = search_start + m.start();
                    let abs_end = search_start + m.end();
                    let overlaps = exclusion_zones
                        .iter()
                        .any(|(zs, ze)| abs_start < *ze && abs_end > *zs);
                    if overlaps {
                        search_start = abs_end;
                    } else {
                        break Some((abs_start, abs_end));
                    }
                }
                None => break None,
            }
        };

        if let Some((m_start, m_end)) = found {
            let replacement = if match_term.eq_ignore_ascii_case(canonical) {
                format!("[[{canonical}]]")
            } else {
                let matched_text = &body_part[m_start..m_end];
                format!("[[{canonical}|{matched_text}]]")
            };
            let new_body = format!(
                "{}{}{}",
                &body_part[..m_start],
                replacement,
                &body_part[m_end..]
            );
            result = format!("{}{}", &result[..fm_end], new_body);
            links_added.push(canonical.to_string());
            linked_stems.insert(canonical.to_string());
        }
    }

    (result, links_added)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_wikilinks_and_aliases() {
        let content = "---\naliases: [LLM Wiki, wiki pattern]\n---\nSee [[Persistent Wiki|the wiki]] and [[RAG]].\n";
        assert_eq!(extract_aliases(content), vec!["LLM Wiki", "wiki pattern"]);
        assert_eq!(extract_wikilinks(content), vec!["Persistent Wiki", "RAG"]);
    }

    #[test]
    fn auto_link_wraps_titles_and_skips_code() {
        let titles = vec![
            (
                "Persistent Wiki".into(),
                "Persistent Wiki".into(),
                "wiki/concepts/Persistent Wiki.md".into(),
            ),
            ("RAG".into(), "RAG".into(), "wiki/concepts/RAG.md".into()),
        ];
        let input = "A Persistent Wiki beats RAG.\n\n```\nRAG in code\n```\nAnd `RAG` inline.\n";
        let (out, added) = auto_link(input, "wiki/notes.md", &titles, &[]);
        assert!(added.contains(&"Persistent Wiki".to_string()));
        assert!(out.contains("[[Persistent Wiki]]"));
        assert!(out.contains("```\nRAG in code\n```"));
        assert!(out.contains("`RAG`"));
    }

    #[test]
    fn auto_link_respects_stoplist() {
        let titles = vec![
            ("Index".into(), "Index".into(), "index.md".into()),
            (
                "Karpathy".into(),
                "Karpathy".into(),
                "wiki/entities/Karpathy.md".into(),
            ),
        ];
        let (out, added) = auto_link(
            "See the Index and Karpathy notes.",
            "wiki/notes.md",
            &titles,
            &["index".into()],
        );
        assert!(!added.iter().any(|l| l == "Index"));
        assert!(added.iter().any(|l| l == "Karpathy"));
        assert!(out.contains("[[Karpathy]]"));
        assert!(!out.contains("[[Index]]"));
    }
}
