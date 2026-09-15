use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

#[derive(Default)]
pub struct SearchIndex {
    pub files: Vec<(PathBuf, String)>,
    trigrams: HashMap<[u8; 3], Vec<usize>>,
    pub doc_lengths: Vec<usize>,
    pub avg_doc_length: f64,
    pub doc_freq: HashMap<String, usize>,
    pub total_docs: usize,
}

impl SearchIndex {
    pub fn build(paths: &[(PathBuf, String)]) -> Self {
        let mut idx = SearchIndex {
            files: paths.to_vec(),
            trigrams: HashMap::new(),
            doc_lengths: Vec::with_capacity(paths.len()),
            avg_doc_length: 0.0,
            doc_freq: HashMap::new(),
            total_docs: paths.len(),
        };

        for (i, (_path, content)) in paths.iter().enumerate() {
            let lower = content.to_lowercase();
            let bytes = lower.as_bytes();
            let mut seen = HashSet::new();
            for window in bytes.windows(3) {
                let tri = [window[0], window[1], window[2]];
                if seen.insert(tri) {
                    idx.trigrams.entry(tri).or_default().push(i);
                }
            }

            idx.doc_lengths.push(content.split_whitespace().count());
            let unique_words: HashSet<&str> = lower.split_whitespace().collect();
            for word in unique_words {
                *idx.doc_freq.entry(word.to_string()).or_insert(0) += 1;
            }
        }

        idx.recompute_avg_doc_length();
        idx
    }

    pub fn search(&self, query: &str, limit: usize) -> Vec<(PathBuf, String, f64)> {
        let query_lower = query.to_lowercase();
        let terms: Vec<String> = query_lower
            .split_whitespace()
            .map(|t| t.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
            .filter(|t| !t.is_empty())
            .collect();

        let term_candidates = |term: &str| -> HashSet<usize> {
            if term.len() < 3 {
                return self
                    .files
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, c))| c.to_lowercase().contains(term))
                    .map(|(i, _)| i)
                    .collect();
            }
            let tb = term.as_bytes();
            let mut acc: Option<HashSet<usize>> = None;
            for w in tb.windows(3) {
                let tri = [w[0], w[1], w[2]];
                match self.trigrams.get(&tri) {
                    Some(idx) => {
                        let s: HashSet<usize> = idx.iter().copied().collect();
                        acc = Some(match acc {
                            Some(p) => p.intersection(&s).copied().collect(),
                            None => s,
                        });
                    }
                    None => return HashSet::new(),
                }
            }
            acc.unwrap_or_default()
                .into_iter()
                .filter(|&i| self.files[i].1.to_lowercase().contains(term))
                .collect()
        };

        let mut candidate_set: HashSet<usize> = HashSet::new();
        if terms.is_empty() {
            for (i, (_, content)) in self.files.iter().enumerate() {
                if content.to_lowercase().contains(&query_lower) {
                    candidate_set.insert(i);
                }
            }
        } else {
            for term in &terms {
                candidate_set.extend(term_candidates(term));
            }
        }

        let k1 = 1.2_f64;
        let b = 0.75_f64;
        let n = self.total_docs as f64;
        let avgdl = if self.avg_doc_length == 0.0 {
            1.0
        } else {
            self.avg_doc_length
        };
        let query_terms: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored: Vec<(PathBuf, String, f64)> = candidate_set
            .into_iter()
            .map(|i| {
                let (path, content) = &self.files[i];
                let doc_len = self.doc_lengths.get(i).copied().unwrap_or(0) as f64;
                let content_lower = content.to_lowercase();
                let doc_words: Vec<&str> = content_lower.split_whitespace().collect();

                let mut score = 0.0_f64;
                for term in &query_terms {
                    let tf = doc_words.iter().filter(|w| *w == term).count() as f64;
                    let df = self.doc_freq.get(*term).copied().unwrap_or(0) as f64;
                    let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
                    score += idf * (tf * (k1 + 1.0)) / (tf + k1 * (1.0 - b + b * doc_len / avgdl));
                }
                (path.clone(), content.clone(), score)
            })
            .collect();

        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
        scored.truncate(limit);
        scored
    }

    pub fn add_file(&mut self, path: &Path, content: &str) {
        self.files.push((path.to_path_buf(), content.to_string()));
        let idx = self.files.len() - 1;
        self.doc_lengths.push(0);
        self.total_docs += 1;
        self.add_trigrams_for(idx);
        self.add_bm25_for(idx);
    }

    pub fn remove_file(&mut self, path: &Path) {
        let Some(idx) = self.files.iter().position(|(p, _)| p == path) else {
            return;
        };
        self.remove_trigrams_for(idx);
        self.remove_bm25_for(idx);
        self.doc_lengths.remove(idx);
        self.total_docs = self.total_docs.saturating_sub(1);
        self.files.remove(idx);
        for indices in self.trigrams.values_mut() {
            indices.retain(|&i| i != idx);
            for i in indices.iter_mut() {
                if *i > idx {
                    *i -= 1;
                }
            }
        }
        self.trigrams.retain(|_, v| !v.is_empty());
        self.recompute_avg_doc_length();
    }

    fn remove_trigrams_for(&mut self, idx: usize) {
        for indices in self.trigrams.values_mut() {
            indices.retain(|&i| i != idx);
        }
        self.trigrams.retain(|_, v| !v.is_empty());
    }

    fn add_trigrams_for(&mut self, idx: usize) {
        let lower = self.files[idx].1.to_lowercase();
        let bytes = lower.as_bytes();
        let mut seen = HashSet::new();
        for window in bytes.windows(3) {
            let tri = [window[0], window[1], window[2]];
            if seen.insert(tri) {
                self.trigrams.entry(tri).or_default().push(idx);
            }
        }
    }

    fn remove_bm25_for(&mut self, idx: usize) {
        let lower = self.files[idx].1.to_lowercase();
        let unique_words: HashSet<&str> = lower.split_whitespace().collect();
        for word in unique_words {
            if let Some(count) = self.doc_freq.get_mut(word) {
                *count = count.saturating_sub(1);
                if *count == 0 {
                    self.doc_freq.remove(word);
                }
            }
        }
    }

    fn add_bm25_for(&mut self, idx: usize) {
        let content = &self.files[idx].1;
        self.doc_lengths[idx] = content.split_whitespace().count();
        let lower = content.to_lowercase();
        let unique_words: HashSet<&str> = lower.split_whitespace().collect();
        for word in unique_words {
            *self.doc_freq.entry(word.to_string()).or_insert(0) += 1;
        }
        self.recompute_avg_doc_length();
    }

    fn recompute_avg_doc_length(&mut self) {
        if self.doc_lengths.is_empty() {
            self.avg_doc_length = 0.0;
        } else {
            let total: usize = self.doc_lengths.iter().sum();
            self.avg_doc_length = total as f64 / self.doc_lengths.len() as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idx() -> SearchIndex {
        SearchIndex::build(&[
            (
                PathBuf::from("a.md"),
                "VWAP mean reversion strategy for crypto".into(),
            ),
            (
                PathBuf::from("b.md"),
                "regime gate using a Markov transition matrix".into(),
            ),
            (
                PathBuf::from("c.md"),
                "gospel study notes on covenant and mercy".into(),
            ),
        ])
    }

    #[test]
    fn search_is_term_based_not_substring() {
        let results = idx().search("VWAP regime reversion", 10);
        let paths: Vec<_> = results
            .iter()
            .map(|(p, _, _)| p.to_string_lossy().to_string())
            .collect();
        assert!(paths.contains(&"a.md".to_string()));
        assert!(paths.contains(&"b.md".to_string()));
        assert!(!paths.contains(&"c.md".to_string()));
    }

    #[test]
    fn incremental_update_keeps_search_working() {
        let mut index = idx();
        index.add_file(Path::new("d.md"), "Karpathy persistent wiki compilation");
        let hits = index.search("persistent wiki", 5);
        assert!(hits.iter().any(|(p, _, _)| p.ends_with("d.md")));
        index.remove_file(Path::new("d.md"));
        let hits = index.search("persistent wiki", 5);
        assert!(hits.iter().all(|(p, _, _)| !p.ends_with("d.md")));
    }
}
