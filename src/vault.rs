use std::path::{Component, Path, PathBuf};

/// Directories skipped when no `.curatorignore` is present.
pub const DEFAULT_IGNORE_NAMES: &[&str] = &[".obsidian", ".trash", ".git", "node_modules"];

/// True when a vault-relative path is under the immutable `raw/` tree.
pub fn is_under_raw(rel: &str) -> bool {
    let normalized = rel.replace('\\', "/");
    let first = normalized
        .trim_start_matches('/')
        .split('/')
        .next()
        .unwrap_or("");
    first.eq_ignore_ascii_case("raw")
}

/// Reject absolute paths and `..` components, returning a normalized relative path.
pub fn normalize_rel(rel: &str) -> Result<PathBuf, String> {
    let trimmed = rel.trim();
    if trimmed.is_empty() {
        return Err("path must not be empty".into());
    }
    let path = Path::new(trimmed);
    if path.is_absolute() {
        return Err("absolute paths are not allowed".into());
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(name) => out.push(name),
            Component::ParentDir => return Err("path traversal (..) is not allowed".into()),
            Component::RootDir | Component::Prefix(_) => {
                return Err("absolute paths are not allowed".into());
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err("path must not be empty".into());
    }
    Ok(out)
}

/// Join `rel` onto `root` after sandbox normalization.
pub fn sandbox_join(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let normalized = normalize_rel(rel)?;
    Ok(root.join(normalized))
}

/// Walk markdown files under each vault root, honoring `.curatorignore` (gitignore syntax).
pub fn all_md_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in roots {
        let mut builder = ignore::WalkBuilder::new(root);
        builder.hidden(true);

        let ignore_file = root.join(".curatorignore");
        if ignore_file.exists() {
            builder.add_ignore(&ignore_file);
        } else {
            builder.filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                !DEFAULT_IGNORE_NAMES.iter().any(|skip| name == *skip)
            });
        }

        for entry in builder.build().flatten() {
            let path = entry.path();
            if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
                files.push(path.to_path_buf());
            }
        }
    }
    files
}

/// Relative path of `abs` against the first matching vault root.
pub fn relative_path(roots: &[PathBuf], abs: &Path) -> String {
    for root in roots {
        if let Ok(rel) = abs.strip_prefix(root) {
            return rel.to_string_lossy().replace('\\', "/");
        }
    }
    abs.to_string_lossy().replace('\\', "/")
}

/// Resolve a relative path against vault roots. Existing files win; new files use the first root.
pub fn resolve_path(roots: &[PathBuf], rel: &str) -> Result<PathBuf, String> {
    let normalized = normalize_rel(rel)?;
    for root in roots {
        let candidate = root.join(&normalized);
        if candidate.exists() {
            return Ok(candidate);
        }
    }
    let root = roots
        .first()
        .ok_or_else(|| "no vault root configured".to_string())?;
    sandbox_join(root, rel)
}

/// List files and directories in a vault subdirectory (or vault root).
pub fn list_dir(roots: &[PathBuf], directory: Option<&str>) -> Result<Vec<String>, String> {
    let mut entries = Vec::new();
    if directory.is_none() && roots.len() > 1 {
        for root in roots {
            entries.push(format!("vault:\t{}", root.display()));
        }
        return Ok(entries);
    }

    let rel = directory.unwrap_or("");
    let dir = if rel.is_empty() {
        roots
            .first()
            .cloned()
            .ok_or_else(|| "no vault root configured".to_string())?
    } else {
        resolve_path(roots, rel)?
    };

    if !dir.is_dir() {
        return Err(format!("{} is not a directory", dir.display()));
    }

    let mut names: Vec<_> = std::fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !DEFAULT_IGNORE_NAMES.contains(&name) && !name.starts_with('.')
        })
        .collect();
    names.sort();

    for path in names {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if path.is_dir() {
            entries.push(format!("{name}/"));
        } else {
            entries.push(name);
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn raw_paths_are_detected() {
        assert!(is_under_raw("raw/article.md"));
        assert!(is_under_raw("Raw/nested/file.md"));
        assert!(is_under_raw("raw"));
        assert!(!is_under_raw("wiki/raw-notes.md"));
        assert!(!is_under_raw("wiki/entities/Foo.md"));
    }

    #[test]
    fn rejects_path_traversal() {
        assert!(normalize_rel("../secret.md").is_err());
        assert!(normalize_rel("wiki/../../etc/passwd").is_err());
        assert!(normalize_rel("/etc/passwd").is_err());
        assert_eq!(
            normalize_rel("wiki/./Foo.md").unwrap(),
            PathBuf::from("wiki/Foo.md")
        );
    }

    #[test]
    fn sandbox_join_stays_inside_root() {
        let root = PathBuf::from("/tmp/vault");
        let joined = sandbox_join(&root, "wiki/Foo.md").unwrap();
        assert_eq!(joined, root.join("wiki/Foo.md"));
        assert!(sandbox_join(&root, "../outside.md").is_err());
    }

    #[test]
    fn write_ban_helper_covers_nested_raw() {
        assert!(is_under_raw("raw/papers/one.md"));
        assert!(!is_under_raw("index.md"));
    }

    #[test]
    fn lists_vault_contents() {
        let dir = tempdir().unwrap();
        std::fs::create_dir(dir.path().join("wiki")).unwrap();
        std::fs::write(dir.path().join("index.md"), "# index\n").unwrap();
        let roots = vec![dir.path().to_path_buf()];
        let listing = list_dir(&roots, None).unwrap();
        assert!(listing.iter().any(|e| e == "wiki/"));
        assert!(listing.iter().any(|e| e == "index.md"));
    }
}
