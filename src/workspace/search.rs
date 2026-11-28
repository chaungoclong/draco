use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use ignore::WalkBuilder;
use regex::RegexBuilder;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceSearchOptions {
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub use_regex: bool,
    pub include_filter: String,
    pub exclude_filter: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMatch {
    pub path: PathBuf,
    pub rel_path: String,
    pub line: usize,
    pub col: usize,
    pub line_text: String,
    pub match_start_byte: usize,
    pub match_end_byte: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceReplaceSummary {
    pub files_changed: usize,
    pub replacements: usize,
    pub modified_paths: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
pub struct WorkspaceSearchSnapshot {
    pub matches: Vec<WorkspaceMatch>,
    pub truncated: bool,
    pub error: Option<String>,
    pub searching: bool,
}

/// Compiles query + options into a `regex::Regex`.
pub fn build_search_regex(
    query: &str,
    opts: &WorkspaceSearchOptions,
) -> Result<regex::Regex, String> {
    if query.is_empty() {
        return Err("Empty search query".to_string());
    }
    let mut pattern = if opts.use_regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    if opts.whole_word {
        pattern = format!(r"\b(?:{pattern})\b");
    }
    RegexBuilder::new(&pattern)
        .case_insensitive(!opts.case_sensitive)
        .build()
        .map_err(|e| e.to_string())
}

fn matches_filter_list(rel_path: &str, filter: &str, is_include: bool) -> bool {
    let tokens: Vec<&str> = filter
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if tokens.is_empty() {
        return is_include;
    }
    let rel_norm = rel_path.replace('\\', "/");
    let file_name = rel_norm.rsplit('/').next().unwrap_or(&rel_norm);

    tokens.iter().any(|pat| {
        if let Some(ext) = pat.strip_prefix("*.") {
            file_name.ends_with(&format!(".{ext}"))
        } else if pat.starts_with('*') && pat.ends_with('*') && pat.len() >= 2 {
            let sub = &pat[1..pat.len() - 1];
            rel_norm.contains(sub)
        } else {
            rel_norm.contains(pat) || file_name == *pat
        }
    })
}

fn is_probably_binary(bytes: &[u8]) -> bool {
    let check_len = bytes.len().min(4096);
    bytes[..check_len].contains(&0)
}

/// Synchronous workspace search respecting `.gitignore` and include/exclude filters.
pub fn search_workspace_sync(
    root: &Path,
    query: &str,
    opts: &WorkspaceSearchOptions,
    max_results: usize,
) -> Result<(Vec<WorkspaceMatch>, bool), String> {
    if query.is_empty() {
        return Ok((Vec::new(), false));
    }
    let re = build_search_regex(query, opts)?;
    let mut results = Vec::new();
    let mut truncated = false;

    let walker = WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|e| e.file_name() != ".git")
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if !opts.include_filter.trim().is_empty()
            && !matches_filter_list(&rel, &opts.include_filter, true)
        {
            continue;
        }
        if !opts.exclude_filter.trim().is_empty()
            && matches_filter_list(&rel, &opts.exclude_filter, false)
        {
            continue;
        }

        // Skip huge files (> 2MB) and binary files
        let Ok(meta) = fs::metadata(path) else {
            continue;
        };
        if meta.len() > 2 * 1024 * 1024 {
            continue;
        }
        let Ok(bytes) = fs::read(path) else {
            continue;
        };
        if is_probably_binary(&bytes) {
            continue;
        }
        let Ok(text) = String::from_utf8(bytes) else {
            continue;
        };

        for (line_idx, raw_line) in text.lines().enumerate() {
            for m in re.find_iter(raw_line) {
                let col = raw_line[..m.start()].chars().count();
                results.push(WorkspaceMatch {
                    path: path.to_path_buf(),
                    rel_path: rel.clone(),
                    line: line_idx,
                    col,
                    line_text: raw_line.to_string(),
                    match_start_byte: m.start(),
                    match_end_byte: m.end(),
                });
                if results.len() >= max_results {
                    truncated = true;
                    return Ok((results, truncated));
                }
            }
        }
    }

    Ok((results, truncated))
}

/// Replaces all matches across workspace files matching `.gitignore` and filters.
pub fn replace_in_workspace(
    root: &Path,
    query: &str,
    replacement: &str,
    opts: &WorkspaceSearchOptions,
) -> Result<WorkspaceReplaceSummary, String> {
    let re = build_search_regex(query, opts)?;
    let (matches, _) = search_workspace_sync(root, query, opts, 10_000)?;

    let mut unique_files: Vec<PathBuf> = Vec::new();
    for m in matches {
        if unique_files.last() != Some(&m.path) && !unique_files.contains(&m.path) {
            unique_files.push(m.path);
        }
    }

    let mut files_changed = 0usize;
    let mut replacements = 0usize;
    let mut modified_paths = Vec::new();

    for path in unique_files {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        let count = re.find_iter(&text).count();
        if count == 0 {
            continue;
        }
        let new_text = if opts.use_regex {
            re.replace_all(&text, replacement).into_owned()
        } else {
            re.replace_all(&text, regex::NoExpand(replacement)).into_owned()
        };
        if new_text != text && fs::write(&path, new_text).is_ok() {
            files_changed += 1;
            replacements += count;
            modified_paths.push(path);
        }
    }

    Ok(WorkspaceReplaceSummary {
        files_changed,
        replacements,
        modified_paths,
    })
}

/// Background workspace search coordinator using `std::thread` + generation cancellation.
#[derive(Default)]
pub struct WorkspaceSearcher {
    generation: Arc<AtomicU64>,
    snapshot: Arc<Mutex<WorkspaceSearchSnapshot>>,
    searching: Arc<AtomicBool>,
}

impl WorkspaceSearcher {
    pub fn snapshot(&self) -> WorkspaceSearchSnapshot {
        let mut snap = self
            .snapshot
            .lock()
            .map(|g| g.clone())
            .unwrap_or_default();
        snap.searching = self.searching.load(Ordering::Relaxed);
        snap
    }

    pub fn clear(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.searching.store(false, Ordering::SeqCst);
        if let Ok(mut guard) = self.snapshot.lock() {
            *guard = WorkspaceSearchSnapshot::default();
        }
    }

    pub fn start_search(
        &self,
        root: PathBuf,
        query: String,
        opts: WorkspaceSearchOptions,
        repaint: Arc<dyn Fn() + Send + Sync>,
    ) {
        let my_gen = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        if query.is_empty() {
            self.clear();
            repaint();
            return;
        }

        self.searching.store(true, Ordering::SeqCst);
        let gen_ref = Arc::clone(&self.generation);
        let snap_ref = Arc::clone(&self.snapshot);
        let searching_ref = Arc::clone(&self.searching);

        thread::spawn(move || {
            let outcome = search_workspace_sync(&root, &query, &opts, 500);
            if gen_ref.load(Ordering::SeqCst) != my_gen {
                return;
            }
            if let Ok(mut guard) = snap_ref.lock() {
                match outcome {
                    Ok((matches, truncated)) => {
                        guard.matches = matches;
                        guard.truncated = truncated;
                        guard.error = None;
                    }
                    Err(e) => {
                        guard.matches.clear();
                        guard.truncated = false;
                        guard.error = Some(e);
                    }
                }
            }
            searching_ref.store(false, Ordering::SeqCst);
            repaint();
        });
    }
}
