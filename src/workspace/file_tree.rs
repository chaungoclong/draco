use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub expanded: bool,
    pub children: Option<Vec<FileNode>>,
}

impl FileNode {
    pub fn from_dir(path: PathBuf) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| path.display().to_string());
        let mut node = Self {
            path,
            name,
            is_dir: true,
            expanded: true,
            children: None,
        };
        node.load_children();
        node
    }

    pub fn load_children(&mut self) {
        if !self.is_dir {
            return;
        }
        let Ok(entries) = fs::read_dir(&self.path) else {
            self.children = Some(Vec::new());
            return;
        };
        let mut list = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == ".git" {
                continue;
            }
            let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            list.push(FileNode {
                path,
                name,
                is_dir,
                expanded: false,
                children: None,
            });
        }
        list.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });
        self.children = Some(list);
    }

    pub fn toggle(&mut self) {
        if !self.is_dir {
            return;
        }
        self.expanded = !self.expanded;
        if self.expanded && self.children.is_none() {
            self.load_children();
        }
    }

    pub fn refresh(&mut self) {
        if self.is_dir && self.expanded {
            self.load_children();
        }
    }
}

pub struct FileTree {
    pub root: PathBuf,
    pub root_node: FileNode,
}

impl FileTree {
    pub fn open(root: PathBuf) -> Self {
        let root_node = FileNode::from_dir(root.clone());
        Self { root, root_node }
    }

    pub fn refresh(&mut self) {
        self.root_node.refresh();
    }

    #[allow(dead_code)]
    pub fn relative_display(&self, path: &Path) -> String {
        path.strip_prefix(&self.root)
            .unwrap_or(path)
            .display()
            .to_string()
    }
}
