use std::io::Cursor;

use draco::lsp::transport::{read_lsp_message, write_lsp_message};

#[test]
fn framing_handles_lf_only_and_extra_headers() {
    let body = r#"{"jsonrpc":"2.0","id":7,"result":true}"#;
    let raw = format!(
        "Content-Type: application/vscode-jsonrpc; charset=utf-8\nContent-Length: {}\n\n{}",
        body.len(),
        body
    );
    let mut cursor = Cursor::new(raw.into_bytes());
    let decoded = read_lsp_message(&mut cursor).unwrap().unwrap();
    assert_eq!(decoded, body);
}

#[test]
fn framing_preserves_multibyte_utf8_byte_length() {
    let mut buf = Vec::new();
    let payload = r#"{"message":"Lỗi cú pháp tại dòng 12 🚀"}"#;
    write_lsp_message(&mut buf, payload).unwrap();

    let mut cursor = Cursor::new(buf);
    assert_eq!(read_lsp_message(&mut cursor).unwrap().unwrap(), payload);
    assert!(read_lsp_message(&mut cursor).unwrap().is_none());
}

#[test]
fn framing_errors_on_truncated_body() {
    let raw = b"Content-Length: 500\r\n\r\nshort".to_vec();
    let mut cursor = Cursor::new(raw);
    assert!(read_lsp_message(&mut cursor).is_err());
}
