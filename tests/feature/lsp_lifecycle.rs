use std::sync::Arc;
use std::time::{Duration, Instant};

use draco::core::LanguageId;
use draco::lsp::client::{LspEvent, LspManager};

use crate::common::TempDir;

#[test]
fn lsp_client_full_jsonrpc_lifecycle_with_mock_server() {
    if std::process::Command::new("python3")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }

    let tmp = TempDir::new("feat_mock_lsp");
    let target_file = tmp.write_file("demo.rs", "fn main() {\n    println!(\"hi\");\n}\n");
    let mock_script = tmp.write_file(
        "mock_lsp.py",
        r#"
import sys, json

def read_msg():
    headers = {}
    while True:
        line = sys.stdin.buffer.readline()
        if not line:
            return None
        if line in (b'\r\n', b'\n'):
            break
        k, v = line.decode('utf-8').split(':', 1)
        headers[k.strip().lower()] = v.strip()
    length = int(headers.get('content-length', '0'))
    body = sys.stdin.buffer.read(length)
    return json.loads(body.decode('utf-8'))

def send_msg(obj):
    raw = json.dumps(obj).encode('utf-8')
    sys.stdout.buffer.write(f"Content-Length: {len(raw)}\r\n\r\n".encode('ascii') + raw)
    sys.stdout.buffer.flush()

while True:
    msg = read_msg()
    if msg is None:
        break
    method = msg.get('method')
    req_id = msg.get('id')
    if method == 'initialize':
        send_msg({"jsonrpc": "2.0", "id": req_id, "result": {"capabilities": {}}})
    elif method == 'textDocument/didOpen':
        uri = msg['params']['textDocument']['uri']
        send_msg({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {
                "uri": uri,
                "diagnostics": [{
                    "range": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 7}},
                    "severity": 1,
                    "message": "mock syntax warning"
                }]
            }
        })
    elif method == 'textDocument/completion':
        send_msg({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": [
                {"label": "println!", "detail": "macro", "insertText": "println!()"},
                {"label": "push_str", "detail": "fn(&mut self, &str)", "insertText": "push_str"}
            ]
        })
    elif method == 'textDocument/definition':
        uri = msg['params']['textDocument']['uri']
        send_msg({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {
                "uri": uri,
                "range": {"start": {"line": 1, "character": 4}, "end": {"line": 1, "character": 11}}
            }
        })
    elif method == 'textDocument/hover':
        send_msg({
            "jsonrpc": "2.0",
            "id": req_id,
            "result": {"contents": {"kind": "plaintext", "value": "fn main()"}}
        })
"#,
    );

    let mut lsp = LspManager::new(Arc::new(|| {}));
    lsp.set_root(tmp.path().to_path_buf());
    assert!(lsp.spawn_custom_server(
        LanguageId::Rust,
        "python3",
        &[mock_script.to_str().unwrap()]
    ));

    lsp.did_open(LanguageId::Rust, &target_file, 1, "fn main() {}\n");
    lsp.request_completion(LanguageId::Rust, &target_file, 1, 4);
    lsp.request_definition(LanguageId::Rust, &target_file, 0, 3);
    lsp.request_hover(LanguageId::Rust, &target_file, 0, 3);

    let (mut ready, mut diag, mut comp, mut def, mut hover) = (false, false, false, false, false);
    let deadline = Instant::now() + Duration::from_secs(4);
    while Instant::now() < deadline && !(ready && diag && comp && def && hover) {
        for ev in lsp.poll_events() {
            match ev {
                LspEvent::Status { status, .. } if status.contains("Ready") => ready = true,
                LspEvent::Diagnostics { items, .. }
                    if items.len() == 1 && items[0].message == "mock syntax warning" =>
                {
                    diag = true;
                }
                LspEvent::Completion { items }
                    if items.len() == 2 && items[0].label == "println!" =>
                {
                    comp = true;
                }
                LspEvent::Definition { line: 1, col: 4, .. } => def = true,
                LspEvent::Hover { text } if text == "fn main()" => hover = true,
                _ => {}
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(ready && diag && comp && def && hover);
}
