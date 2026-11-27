use std::io::{self, BufRead, Write};

/// Writes a JSON-RPC 2.0 payload framed with `Content-Length`.
pub fn write_lsp_message(writer: &mut impl Write, payload: &str) -> io::Result<()> {
    let bytes = payload.as_bytes();
    write!(writer, "Content-Length: {}\r\n\r\n", bytes.len())?;
    writer.write_all(bytes)?;
    writer.flush()
}

/// Reads a single `Content-Length`-framed JSON-RPC payload. Returns `Ok(None)` on EOF.
pub fn read_lsp_message(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut header_line = String::new();

    loop {
        header_line.clear();
        let n = reader.read_line(&mut header_line)?;
        if n == 0 {
            return Ok(None);
        }
        let trimmed = header_line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }

    let Some(len) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Missing Content-Length header in LSP message",
        ));
    };

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    String::from_utf8(body)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn lsp_framing_roundtrip() {
        let mut buf = Vec::new();
        let msg = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        write_lsp_message(&mut buf, msg).unwrap();

        let mut cursor = Cursor::new(buf);
        let decoded = read_lsp_message(&mut cursor).unwrap().unwrap();
        assert_eq!(decoded, msg);
        assert!(read_lsp_message(&mut cursor).unwrap().is_none());
    }

    #[test]
    fn handles_multiple_utf8_messages_and_rejects_missing_header() {
        let mut buf = Vec::new();
        let m1 = r#"{"msg":"Xin chào Draco 🚀"}"#;
        let m2 = r#"{"id":2,"result":null}"#;
        write_lsp_message(&mut buf, m1).unwrap();
        write_lsp_message(&mut buf, m2).unwrap();

        let mut cursor = Cursor::new(buf);
        assert_eq!(read_lsp_message(&mut cursor).unwrap().unwrap(), m1);
        assert_eq!(read_lsp_message(&mut cursor).unwrap().unwrap(), m2);
        assert!(read_lsp_message(&mut cursor).unwrap().is_none());

        let mut bad = Cursor::new(b"X-Other: 1\r\n\r\n{}".to_vec());
        assert!(read_lsp_message(&mut bad).is_err());
    }
}
