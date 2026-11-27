use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use ignore::WalkBuilder;
use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};
use nucleo_matcher::{Config, Matcher, Utf32Str};

/// Unicode-aware SIMD fuzzy path matcher powered by `nucleo-matcher` (Helix Editor).
pub fn fuzzy_score(pattern: &str, candidate: &str) -> Option<i32> {
    if pattern.is_empty() {
        return Some(0);
    }
    let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
    let pat = Pattern::parse(pattern, CaseMatching::Smart, Normalization::Smart);
    let mut buf = Vec::new();
    let haystack = Utf32Str::new(candidate, &mut buf);
    pat.score(haystack, &mut matcher).map(|s| s as i32)
}

#[derive(Clone)]
pub struct WorkspaceIndexer {
    files: Arc<Mutex<Vec<PathBuf>>>,
}

impl Default for WorkspaceIndexer {
    fn default() -> Self {
        Self {
            files: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl WorkspaceIndexer {
    pub fn start_index(&self, root: PathBuf, repaint: Arc<dyn Fn() + Send + Sync>) {
        let files_arc = Arc::clone(&self.files);
        thread::spawn(move || {
            let mut collected = Vec::new();
            let walker = WalkBuilder::new(&root)
                .hidden(false)
                .filter_entry(|e| e.file_name() != ".git")
                .build();

            for entry in walker.flatten() {
                if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    collected.push(entry.into_path());
                    if collected.len() >= 100_000 {
                        break;
                    }
                }
            }
            if let Ok(mut guard) = files_arc.lock() {
                *guard = collected;
            }
            repaint();
        });
    }

    pub fn query(&self, root: &Path, pattern: &str, limit: usize) -> Vec<(PathBuf, String, i32)> {
        let Ok(guard) = self.files.lock() else {
            return Vec::new();
        };

        if pattern.is_empty() {
            let mut out: Vec<(PathBuf, String, i32)> = guard
                .iter()
                .take(limit)
                .map(|p| {
                    let rel = p.strip_prefix(root).unwrap_or(p).display().to_string();
                    (p.clone(), rel, 0)
                })
                .collect();
            out.sort_by(|a, b| a.1.cmp(&b.1));
            return out;
        }

        let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
        let pat = Pattern::parse(pattern, CaseMatching::Smart, Normalization::Smart);
        let mut buf = Vec::new();

        let mut scored: Vec<(PathBuf, String, i32)> = guard
            .iter()
            .filter_map(|p| {
                let rel = p.strip_prefix(root).unwrap_or(p).display().to_string();
                let haystack = Utf32Str::new(&rel, &mut buf);
                pat.score(haystack, &mut matcher)
                    .map(|s| (p.clone(), rel, s as i32))
            })
            .collect();
        scored.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.1.cmp(&b.1)));
        scored.truncate(limit);
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fuzzy_ranks_filename_match_higher() {
        let s1 = fuzzy_score("buf", "src/core/buffer.rs").unwrap();
        let s2 = fuzzy_score("buf", "build/foo/utility/file.rs").unwrap();
        assert!(s1 > s2);
        assert!(fuzzy_score("zzz", "src/main.rs").is_none());
    }
}
