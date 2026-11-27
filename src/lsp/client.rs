use std::collections::HashMap;
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use serde_json::{json, Value};
use url::Url;

use crate::core::LanguageId;
use crate::lsp::transport::{read_lsp_message, write_lsp_message};

#[derive(Debug, Clone)]
pub struct DiagnosticItem {
    pub start_line: usize,
    pub start_col: usize,
    pub end_line: usize,
    pub end_col: usize,
    pub severity: u8, // 1=Error, 2=Warning, 3=Info, 4=Hint
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    pub label: String,
    pub detail: Option<String>,
    pub insert_text: String,
}

#[derive(Debug, Clone)]
pub enum LspEvent {
    Diagnostics {
        path: PathBuf,
        items: Vec<DiagnosticItem>,
    },
    Completion {
        items: Vec<CompletionCandidate>,
    },
    Hover {
        text: String,
    },
    Definition {
        path: PathBuf,
        line: usize,
        col: usize,
    },
    Status {
        lang: LanguageId,
        status: String,
    },
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
enum PendingKind {
    Initialize,
    Completion,
    Hover,
    Definition,
}

struct ServerHandle {
    tx_out: mpsc::Sender<String>,
    pending: Arc<Mutex<HashMap<i64, PendingKind>>>,
    _child: Child,
}

pub struct LspManager {
    root: Option<PathBuf>,
    servers: HashMap<LanguageId, ServerHandle>,
    status: HashMap<LanguageId, String>,
    next_id: AtomicI64,
    tx_events: mpsc::Sender<LspEvent>,
    rx_events: mpsc::Receiver<LspEvent>,
    repaint: Arc<dyn Fn() + Send + Sync>,
}

impl LspManager {
    pub fn new(repaint: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (tx_events, rx_events) = mpsc::channel();
        Self {
            root: None,
            servers: HashMap::new(),
            status: HashMap::new(),
            next_id: AtomicI64::new(1),
            tx_events,
            rx_events,
            repaint,
        }
    }

    pub fn set_root(&mut self, root: PathBuf) {
        self.root = Some(root);
    }

    pub fn status_for(&self, lang: LanguageId) -> Option<&str> {
        self.status.get(&lang).map(|s| s.as_str())
    }

    pub fn poll_events(&mut self) -> Vec<LspEvent> {
        let mut out = Vec::new();
        while let Ok(ev) = self.rx_events.try_recv() {
            if let LspEvent::Status { lang, ref status } = ev {
                self.status.insert(lang, status.clone());
            }
            out.push(ev);
        }
        out
    }

    fn server_command(lang: LanguageId) -> Option<(&'static str, &'static [&'static str])> {
        match lang {
            LanguageId::Rust => Some(("rust-analyzer", &[])),
            LanguageId::Python => Some(("pyright-langserver", &["--stdio"])),
            LanguageId::JavaScript | LanguageId::TypeScript => {
                Some(("typescript-language-server", &["--stdio"]))
            }
            LanguageId::C => Some(("clangd", &[])),
            LanguageId::Php => Some(("intelephense", &["--stdio"])),
            _ => None,
        }
    }

    pub fn ensure_server(&mut self, lang: LanguageId) -> bool {
        if self.servers.contains_key(&lang) {
            return true;
        }
        let Some((cmd, args)) = Self::server_command(lang) else {
            return false;
        };
        self.spawn_custom_server(lang, cmd, args)
    }

    pub fn spawn_custom_server(&mut self, lang: LanguageId, cmd: &str, args: &[&str]) -> bool {
        if self.servers.contains_key(&lang) {
            return true;
        }

        let mut command = Command::new(cmd);
        command
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());
        if let Some(root) = &self.root {
            command.current_dir(root);
        }

        let Ok(mut child) = command.spawn() else {
            self.status.insert(lang, format!("LSP: {cmd} not found"));
            return false;
        };

        let Some(stdin) = child.stdin.take() else {
            return false;
        };
        let Some(stdout) = child.stdout.take() else {
            return false;
        };

        let (tx_out, rx_out) = mpsc::channel::<String>();
        let pending = Arc::new(Mutex::new(HashMap::<i64, PendingKind>::new()));

        // Writer thread
        thread::spawn(move || {
            let mut writer = BufWriter::new(stdin);
            while let Ok(msg) = rx_out.recv() {
                if write_lsp_message(&mut writer, &msg).is_err() {
                    break;
                }
            }
        });

        // Reader thread
        let pending_reader = Arc::clone(&pending);
        let tx_ev = self.tx_events.clone();
        let tx_out_reader = tx_out.clone();
        let repaint = Arc::clone(&self.repaint);
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            while let Ok(Some(raw)) = read_lsp_message(&mut reader) {
                if let Ok(val) = serde_json::from_str::<Value>(&raw) {
                    handle_incoming(
                        lang,
                        &val,
                        &pending_reader,
                        &tx_ev,
                        &tx_out_reader,
                    );
                    repaint();
                }
            }
            let _ = tx_ev.send(LspEvent::Status {
                lang,
                status: "LSP: stopped".to_string(),
            });
            repaint();
        });

        self.status.insert(lang, format!("LSP: {cmd} starting…"));
        self.servers.insert(
            lang,
            ServerHandle {
                tx_out,
                pending,
                _child: child,
            },
        );

        // Send initialize
        let root_uri = self
            .root
            .as_ref()
            .and_then(|p| Url::from_file_path(p).ok())
            .map(|u| u.to_string());
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let init_msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "completion": { "completionItem": { "snippetSupport": false } },
                        "hover": { "contentFormat": ["plaintext", "markdown"] },
                        "publishDiagnostics": {}
                    }
                }
            }
        });
        self.send_request(lang, id, PendingKind::Initialize, init_msg);
        true
    }

    fn send_request(&self, lang: LanguageId, id: i64, kind: PendingKind, payload: Value) {
        if let Some(srv) = self.servers.get(&lang) {
            if let Ok(mut p) = srv.pending.lock() {
                p.insert(id, kind);
            }
            let _ = srv.tx_out.send(payload.to_string());
        }
    }

    fn send_notification(&self, lang: LanguageId, payload: Value) {
        if let Some(srv) = self.servers.get(&lang) {
            let _ = srv.tx_out.send(payload.to_string());
        }
    }

    pub fn did_open(&mut self, lang: LanguageId, path: &Path, version: i32, text: &str) {
        if !self.ensure_server(lang) {
            return;
        }
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri.to_string(),
                    "languageId": lang.lsp_language_id(),
                    "version": version,
                    "text": text
                }
            }
        });
        self.send_notification(lang, msg);
    }

    pub fn did_change(&self, lang: LanguageId, path: &Path, version: i32, text: &str) {
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri.to_string(), "version": version },
                "contentChanges": [{ "text": text }]
            }
        });
        self.send_notification(lang, msg);
    }

    pub fn did_save(&self, lang: LanguageId, path: &Path) {
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didSave",
            "params": { "textDocument": { "uri": uri.to_string() } }
        });
        self.send_notification(lang, msg);
    }

    pub fn request_completion(&self, lang: LanguageId, path: &Path, line: usize, col: usize) {
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": col }
            }
        });
        self.send_request(lang, id, PendingKind::Completion, msg);
    }

    pub fn request_definition(&self, lang: LanguageId, path: &Path, line: usize, col: usize) {
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": col }
            }
        });
        self.send_request(lang, id, PendingKind::Definition, msg);
    }

    #[allow(dead_code)]
    pub fn request_hover(&self, lang: LanguageId, path: &Path, line: usize, col: usize) {
        let Ok(uri) = Url::from_file_path(path) else {
            return;
        };
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let msg = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri.to_string() },
                "position": { "line": line, "character": col }
            }
        });
        self.send_request(lang, id, PendingKind::Hover, msg);
    }
}

fn handle_incoming(
    lang: LanguageId,
    val: &Value,
    pending: &Arc<Mutex<HashMap<i64, PendingKind>>>,
    tx_ev: &mpsc::Sender<LspEvent>,
    tx_out: &mpsc::Sender<String>,
) {
    // Notification from server
    if let Some(method) = val.get("method").and_then(|m| m.as_str()) {
        if method == "textDocument/publishDiagnostics" {
            if let Some(params) = val.get("params") {
                if let Some(uri_str) = params.get("uri").and_then(|u| u.as_str()) {
                    if let Ok(path) = Url::parse(uri_str).and_then(|u| u.to_file_path().map_err(|_| url::ParseError::EmptyHost)) {
                        let mut items = Vec::new();
                        if let Some(arr) = params.get("diagnostics").and_then(|d| d.as_array()) {
                            for d in arr {
                                let start = &d["range"]["start"];
                                let end = &d["range"]["end"];
                                items.push(DiagnosticItem {
                                    start_line: start["line"].as_u64().unwrap_or(0) as usize,
                                    start_col: start["character"].as_u64().unwrap_or(0) as usize,
                                    end_line: end["line"].as_u64().unwrap_or(0) as usize,
                                    end_col: end["character"].as_u64().unwrap_or(0) as usize,
                                    severity: d["severity"].as_u64().unwrap_or(1) as u8,
                                    message: d["message"].as_str().unwrap_or("").to_string(),
                                });
                            }
                        }
                        let _ = tx_ev.send(LspEvent::Diagnostics { path, items });
                    }
                }
            }
        }
        return;
    }

    // Response to our request
    let Some(id) = val.get("id").and_then(|i| i.as_i64()) else {
        return;
    };
    let kind = pending.lock().ok().and_then(|mut m| m.remove(&id));
    let Some(kind) = kind else {
        return;
    };
    let result = &val["result"];

    match kind {
        PendingKind::Initialize => {
            let notif = json!({
                "jsonrpc": "2.0",
                "method": "initialized",
                "params": {}
            });
            let _ = tx_out.send(notif.to_string());
            let _ = tx_ev.send(LspEvent::Status {
                lang,
                status: "LSP: Ready".to_string(),
            });
        }
        PendingKind::Completion => {
            let arr = result
                .as_array()
                .or_else(|| result.get("items").and_then(|i| i.as_array()));
            let mut items = Vec::new();
            if let Some(arr) = arr {
                for item in arr.iter().take(50) {
                    let label = item["label"].as_str().unwrap_or("").to_string();
                    if label.is_empty() {
                        continue;
                    }
                    let insert_text = item["insertText"]
                        .as_str()
                        .unwrap_or(&label)
                        .to_string();
                    let detail = item["detail"].as_str().map(|s| s.to_string());
                    items.push(CompletionCandidate {
                        label,
                        detail,
                        insert_text,
                    });
                }
            }
            let _ = tx_ev.send(LspEvent::Completion { items });
        }
        PendingKind::Definition => {
            let loc = if result.is_array() {
                result.as_array().and_then(|a| a.first())
            } else if result.is_object() {
                Some(result)
            } else {
                None
            };
            if let Some(loc) = loc {
                let uri_str = loc["uri"]
                    .as_str()
                    .or_else(|| loc["targetUri"].as_str());
                let range = if loc.get("range").is_some() {
                    &loc["range"]["start"]
                } else {
                    &loc["targetSelectionRange"]["start"]
                };
                if let Some(u) = uri_str {
                    if let Ok(path) = Url::parse(u).and_then(|url| url.to_file_path().map_err(|_| url::ParseError::EmptyHost)) {
                        let line = range["line"].as_u64().unwrap_or(0) as usize;
                        let col = range["character"].as_u64().unwrap_or(0) as usize;
                        let _ = tx_ev.send(LspEvent::Definition { path, line, col });
                    }
                }
            }
        }
        PendingKind::Hover => {
            let contents = &result["contents"];
            let text = if let Some(s) = contents.as_str() {
                s.to_string()
            } else if let Some(v) = contents.get("value").and_then(|v| v.as_str()) {
                v.to_string()
            } else {
                String::new()
            };
            if !text.is_empty() {
                let _ = tx_ev.send(LspEvent::Hover { text });
            }
        }
    }
}
