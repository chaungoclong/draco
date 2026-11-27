use std::path::{Path, PathBuf};
use std::time::Instant;

use eframe::egui::{self, Color32, CursorIcon, FontFamily, FontId, Pos2, Rect, RichText, Stroke, TextFormat, Vec2};
use pulldown_cmark::{CodeBlockKind, Event as MdEvent, HeadingLevel, Options, Parser, Tag, TagEnd};
use ropey::Rope;

use crate::core::buffer::Document;
use crate::core::LanguageId;
use crate::syntax::{HighlightSpan, SyntaxState};
use crate::theme::Theme;
use crate::ui::mermaid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarkdownViewMode {
    Editor,
    #[default]
    Split,
    Preview,
}

impl MarkdownViewMode {
    pub fn cycle(self) -> Self {
        match self {
            Self::Editor => Self::Split,
            Self::Split => Self::Preview,
            Self::Preview => Self::Editor,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewAction {
    OpenFile {
        path: PathBuf,
        anchor: Option<String>,
    },
    OpenExternalUrl(String),
    ScrollEditorToLine(f32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InlineSpan {
    Text(String),
    Bold(String),
    Italic(String),
    Code(String),
    Strike(String),
    Link { label: String, url: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TocEntry {
    pub level: u8,
    pub title: String,
    pub slug: String,
    pub ascii_slug: String,
}

#[derive(Debug, Clone)]
pub enum MdBlock {
    Heading {
        level: u8,
        slug: String,
        ascii_slug: String,
        spans: Vec<InlineSpan>,
    },
    Paragraph(Vec<InlineSpan>),
    CodeBlock {
        lang: String,
        code: String,
        spans: Vec<HighlightSpan>,
    },
    Mermaid {
        source: String,
    },
    Blockquote(Vec<InlineSpan>),
    BulletItem {
        indent: usize,
        spans: Vec<InlineSpan>,
    },
    OrderedItem {
        number: String,
        spans: Vec<InlineSpan>,
    },
    TaskItem {
        checked: bool,
        spans: Vec<InlineSpan>,
    },
    Table {
        headers: Vec<Vec<InlineSpan>>,
        rows: Vec<Vec<Vec<InlineSpan>>>,
    },
    HorizontalRule,
}

#[derive(Debug, Clone)]
pub struct MappedBlock {
    pub start_line: usize,
    pub end_line: usize,
    pub block: MdBlock,
}

#[derive(Debug, Clone, Copy)]
pub struct BlockLayoutPos {
    pub start_line: usize,
    pub end_line: usize,
    pub top_y: f32,
    pub bottom_y: f32,
}

/// Version-cached Markdown AST + Google Docs-style Outline + block-mapped scroll sync.
/// Reparses ONLY when `(doc.id, doc.version)` changes -> 0ms parse cost while scrolling.
pub struct MarkdownPreviewCache {
    cached_doc_id: u64,
    cached_version: i32,
    pub blocks: Vec<MdBlock>,
    pub mapped_blocks: Vec<MappedBlock>,
    pub headings: Vec<TocEntry>,
    pub show_toc: bool,
    pub scroll_sync: bool,
    pub scroll_to_anchor: Option<String>,
    // Scroll sync & active heading tracking + Outline flash highlight
    last_editor_top_line: f32,
    block_positions: Vec<BlockLayoutPos>,
    heading_y_positions: Vec<(String, f32)>,
    current_scroll_y: f32,
    pending_scroll_y: Option<f32>,
    pub highlighted_heading: Option<(String, Instant)>,
    pub selected_outline_slug: Option<String>,
}

impl Default for MarkdownPreviewCache {
    fn default() -> Self {
        Self {
            cached_doc_id: 0,
            cached_version: -1,
            blocks: Vec::new(),
            mapped_blocks: Vec::new(),
            headings: Vec::new(),
            show_toc: true,
            scroll_sync: true,
            scroll_to_anchor: None,
            last_editor_top_line: -1.0,
            block_positions: Vec::new(),
            heading_y_positions: Vec::new(),
            current_scroll_y: 0.0,
            pending_scroll_y: None,
            highlighted_heading: None,
            selected_outline_slug: None,
        }
    }
}

impl MarkdownPreviewCache {
    pub fn sync(&mut self, doc: &Document) {
        if self.cached_doc_id == doc.id
            && self.cached_version == doc.version
            && !self.blocks.is_empty()
        {
            return;
        }
        let text = doc.rope.to_string();
        let mapped = parse_markdown_mapped(&text, &doc.rope);
        self.blocks = mapped.iter().map(|m| m.block.clone()).collect();
        self.headings = self
            .blocks
            .iter()
            .filter_map(|b| match b {
                MdBlock::Heading {
                    level,
                    slug,
                    ascii_slug,
                    spans,
                } => Some(TocEntry {
                    level: *level,
                    title: inline_plain_text(spans),
                    slug: slug.clone(),
                    ascii_slug: ascii_slug.clone(),
                }),
                _ => None,
            })
            .collect();
        self.mapped_blocks = mapped;
        if self.cached_doc_id != doc.id {
            self.last_editor_top_line = -1.0;
            self.current_scroll_y = 0.0;
            self.pending_scroll_y = None;
            self.block_positions.clear();
            self.heading_y_positions.clear();
            self.selected_outline_slug = None;
        }
        self.cached_doc_id = doc.id;
        self.cached_version = doc.version;
    }

    pub fn cached_version(&self) -> i32 {
        self.cached_version
    }

    /// Updates preview scroll offset immediately & cleanly when the editor scrolls.
    pub fn notify_editor_scroll(&mut self, editor_top_line: f32) {
        if !self.scroll_sync {
            return;
        }
        if (editor_top_line - self.last_editor_top_line).abs() > 0.02 {
            self.last_editor_top_line = editor_top_line;
            self.selected_outline_slug = None;
            if let Some(target) = compute_preview_scroll_y(editor_top_line, &self.block_positions) {
                self.pending_scroll_y = Some(target);
            }
        }
    }

    fn active_heading_slug(&self) -> Option<&str> {
        if let Some(sel) = self.selected_outline_slug.as_deref() {
            return Some(sel);
        }
        let probe = self.current_scroll_y + 36.0;
        let mut active: Option<&str> = self.heading_y_positions.first().map(|(s, _)| s.as_str());
        for (slug, y) in &self.heading_y_positions {
            if *y <= probe {
                active = Some(slug.as_str());
            } else {
                break;
            }
        }
        active
    }
}

/// Piecewise-linear interpolation mapping an editor's fractional top line to the exact
/// preview Y pixel offset using recorded rendered block positions.
pub fn compute_preview_scroll_y(
    editor_top_line: f32,
    positions: &[BlockLayoutPos],
) -> Option<f32> {
    if positions.is_empty() {
        return None;
    }
    if editor_top_line <= 0.0 {
        return Some(0.0);
    }

    let first = positions[0];
    if editor_top_line < first.start_line as f32 {
        let denom = (first.start_line as f32).max(1.0);
        let t = (editor_top_line / denom).clamp(0.0, 1.0);
        return Some(first.top_y * t);
    }

    for i in 0..positions.len() {
        let cur = positions[i];
        let cur_start = cur.start_line as f32;
        let cur_end = (cur.end_line + 1) as f32;

        if editor_top_line >= cur_start && editor_top_line <= cur_end {
            let span = (cur_end - cur_start).max(1.0);
            let t = ((editor_top_line - cur_start) / span).clamp(0.0, 1.0);
            return Some(cur.top_y + t * (cur.bottom_y - cur.top_y));
        }

        if i + 1 < positions.len() {
            let next = positions[i + 1];
            let next_start = next.start_line as f32;
            if editor_top_line > cur_end && editor_top_line < next_start {
                let gap = (next_start - cur_end).max(1.0);
                let t = ((editor_top_line - cur_end) / gap).clamp(0.0, 1.0);
                return Some(cur.bottom_y + t * (next.top_y - cur.bottom_y));
            }
        }
    }

    Some(positions.last().unwrap().bottom_y.max(0.0))
}

/// Exact inverse of `compute_preview_scroll_y`: maps a preview Y pixel offset back to
/// the corresponding fractional editor line for bidirectional scroll sync.
pub fn compute_editor_line_from_preview_y(
    preview_y: f32,
    positions: &[BlockLayoutPos],
) -> Option<f32> {
    if positions.is_empty() {
        return None;
    }
    if preview_y <= 0.0 {
        return Some(0.0);
    }

    let first = positions[0];
    if preview_y < first.top_y {
        let t = (preview_y / first.top_y.max(1.0)).clamp(0.0, 1.0);
        return Some(t * first.start_line as f32);
    }

    for i in 0..positions.len() {
        let cur = positions[i];
        let cur_start = cur.start_line as f32;
        let cur_end = (cur.end_line + 1) as f32;

        if preview_y >= cur.top_y && preview_y <= cur.bottom_y {
            let h = (cur.bottom_y - cur.top_y).max(1.0);
            let t = ((preview_y - cur.top_y) / h).clamp(0.0, 1.0);
            return Some(cur_start + t * (cur_end - cur_start));
        }

        if i + 1 < positions.len() {
            let next = positions[i + 1];
            if preview_y > cur.bottom_y && preview_y < next.top_y {
                let gap_y = (next.top_y - cur.bottom_y).max(1.0);
                let t = ((preview_y - cur.bottom_y) / gap_y).clamp(0.0, 1.0);
                return Some(cur_end + t * (next.start_line as f32 - cur_end));
            }
        }
    }

    Some(positions.last().unwrap().end_line as f32)
}

pub fn inline_plain_text(spans: &[InlineSpan]) -> String {
    let mut s = String::new();
    for sp in spans {
        match sp {
            InlineSpan::Text(t)
            | InlineSpan::Bold(t)
            | InlineSpan::Italic(t)
            | InlineSpan::Code(t)
            | InlineSpan::Strike(t) => s.push_str(t),
            InlineSpan::Link { label, .. } => s.push_str(label),
        }
    }
    s
}

/// Generates a GitHub-style heading anchor slug (preserving Unicode letters like Vietnamese).
pub fn heading_slug(title: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in title.trim().to_lowercase().chars() {
        if c.is_alphanumeric() {
            out.push(c);
            prev_dash = false;
        } else if (c.is_whitespace() || c == '-') && !prev_dash && !out.is_empty() {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_end_matches('-').to_string()
}

/// Generates an ASCII-folded slug (maps Vietnamese diacritics to base ASCII letters).
pub fn heading_ascii_slug(title: &str) -> String {
    let folded: String = title.chars().map(fold_vietnamese_char).collect();
    heading_slug(&folded)
}

fn fold_vietnamese_char(c: char) -> char {
    match c {
        'à' | 'á' | 'ạ' | 'ả' | 'ã' | 'â' | 'ầ' | 'ấ' | 'ậ' | 'ẩ' | 'ẫ' | 'ă' | 'ằ' | 'ắ'
        | 'ặ' | 'ẳ' | 'ẵ' | 'À' | 'Á' | 'Ạ' | 'Ả' | 'Ã' | 'Â' | 'Ầ' | 'Ấ' | 'Ậ' | 'Ẩ' | 'Ẫ'
        | 'Ă' | 'Ằ' | 'Ắ' | 'Ặ' | 'Ẳ' | 'Ẵ' => 'a',
        'è' | 'é' | 'ẹ' | 'ẻ' | 'ẽ' | 'ê' | 'ề' | 'ế' | 'ệ' | 'ể' | 'ễ' | 'È' | 'É' | 'Ẹ'
        | 'Ẻ' | 'Ẽ' | 'Ê' | 'Ề' | 'Ế' | 'Ệ' | 'Ể' | 'Ễ' => 'e',
        'ì' | 'í' | 'ị' | 'ỉ' | 'ĩ' | 'Ì' | 'Í' | 'Ị' | 'Ỉ' | 'Ĩ' => 'i',
        'ò' | 'ó' | 'ọ' | 'ỏ' | 'õ' | 'ô' | 'ồ' | 'ố' | 'ộ' | 'ổ' | 'ỗ' | 'ơ' | 'ờ' | 'ớ'
        | 'ợ' | 'ở' | 'ỡ' | 'Ò' | 'Ó' | 'Ọ' | 'Ỏ' | 'Õ' | 'Ô' | 'Ồ' | 'Ố' | 'Ộ' | 'Ổ' | 'Ỗ'
        | 'Ơ' | 'Ờ' | 'Ớ' | 'Ợ' | 'Ở' | 'Ỡ' => 'o',
        'ù' | 'ú' | 'ụ' | 'ủ' | 'ũ' | 'ư' | 'ừ' | 'ứ' | 'ự' | 'ử' | 'ữ' | 'Ù' | 'Ú' | 'Ụ'
        | 'Ủ' | 'Ũ' | 'Ư' | 'Ừ' | 'Ứ' | 'Ự' | 'Ử' | 'Ữ' => 'u',
        'ỳ' | 'ý' | 'ỵ' | 'ỷ' | 'ỹ' | 'Ỳ' | 'Ý' | 'Ỵ' | 'Ỷ' | 'Ỹ' => 'y',
        'đ' | 'Đ' => 'd',
        other => other,
    }
}

pub fn anchor_matches(target: &str, slug: &str, ascii_slug: &str) -> bool {
    let norm = heading_slug(target);
    let norm_ascii = heading_ascii_slug(target);
    !norm.is_empty() && (norm == slug || norm == ascii_slug || norm_ascii == ascii_slug)
}

/// Resolves a Markdown link target relative to `current_doc_path` or `workspace_root`.
pub fn resolve_markdown_link(
    url: &str,
    current_doc_path: Option<&Path>,
    workspace_root: Option<&Path>,
) -> Option<PreviewAction> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed.starts_with("http://")
        || trimmed.starts_with("https://")
        || trimmed.starts_with("mailto:")
    {
        return Some(PreviewAction::OpenExternalUrl(trimmed.to_string()));
    }
    if trimmed.starts_with('#') {
        return None;
    }

    let (file_part, anchor) = match trimmed.split_once('#') {
        Some((f, a)) => (f, Some(a.to_string())),
        None => (trimmed, None),
    };

    let rel_path = Path::new(file_part);
    let candidate = if rel_path.is_absolute() {
        rel_path.to_path_buf()
    } else if let Some(base_dir) = current_doc_path.and_then(|p| p.parent()) {
        let p = base_dir.join(rel_path);
        if p.exists() {
            p
        } else if let Some(root) = workspace_root {
            root.join(rel_path)
        } else {
            p
        }
    } else if let Some(root) = workspace_root {
        root.join(rel_path)
    } else {
        rel_path.to_path_buf()
    };

    let final_path = std::fs::canonicalize(&candidate).unwrap_or(candidate);
    Some(PreviewAction::OpenFile {
        path: final_path,
        anchor,
    })
}

/// Parses CommonMark + GFM (tables, strikethrough, task lists) using `pulldown-cmark`.
pub fn parse_markdown(input: &str) -> Vec<MdBlock> {
    let rope = Rope::from_str(input);
    parse_markdown_mapped(input, &rope)
        .into_iter()
        .map(|m| m.block)
        .collect()
}

/// Parses CommonMark + GFM with exact source line ranges (`start_line..=end_line`) per block.
pub fn parse_markdown_mapped(input: &str, rope: &Rope) -> Vec<MappedBlock> {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(input, opts).into_offset_iter();
    let mut blocks: Vec<MappedBlock> = Vec::new();
    let max_byte = rope.len_bytes();
    let byte_to_line = |b: usize| -> usize {
        if max_byte == 0 {
            0
        } else {
            rope.byte_to_line(b.min(max_byte.saturating_sub(1)))
        }
    };

    let mut block_start_line: usize = 0;
    let mut in_heading: Option<u8> = None;
    let mut in_blockquote = false;
    let mut in_code_block: Option<(String, String)> = None;
    let mut list_stack: Vec<Option<u64>> = Vec::new();
    let mut item_task_checked: Option<bool> = None;

    let mut in_table = false;
    let mut in_table_head = false;
    let mut table_headers: Vec<Vec<InlineSpan>> = Vec::new();
    let mut table_rows: Vec<Vec<Vec<InlineSpan>>> = Vec::new();
    let mut current_row: Vec<Vec<InlineSpan>> = Vec::new();

    let mut inline_buf: Vec<InlineSpan> = Vec::new();
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut strike_depth = 0usize;
    let mut active_link: Option<(String, String)> = None;

    let push_text = |text: &str,
                     bold_depth: usize,
                     italic_depth: usize,
                     strike_depth: usize,
                     active_link: &mut Option<(String, String)>,
                     inline_buf: &mut Vec<InlineSpan>| {
        if let Some((_, label)) = active_link {
            label.push_str(text);
            return;
        }
        let s = text.to_string();
        if strike_depth > 0 {
            inline_buf.push(InlineSpan::Strike(s));
        } else if bold_depth > 0 {
            inline_buf.push(InlineSpan::Bold(s));
        } else if italic_depth > 0 {
            inline_buf.push(InlineSpan::Italic(s));
        } else {
            inline_buf.push(InlineSpan::Text(s));
        }
    };

    for (ev, range) in parser {
        let cur_start_line = byte_to_line(range.start);
        let cur_end_line = byte_to_line(range.end.saturating_sub(1).max(range.start));

        match ev {
            MdEvent::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    in_heading = Some(heading_level_to_u8(level));
                    block_start_line = cur_start_line;
                    inline_buf.clear();
                }
                Tag::Paragraph => {
                    if !in_blockquote && list_stack.is_empty() && !in_table {
                        block_start_line = cur_start_line;
                        inline_buf.clear();
                    }
                }
                Tag::BlockQuote(_) => {
                    in_blockquote = true;
                    block_start_line = cur_start_line;
                    inline_buf.clear();
                }
                Tag::CodeBlock(kind) => {
                    let lang = match kind {
                        CodeBlockKind::Fenced(info) => {
                            info.split_whitespace().next().unwrap_or("").to_string()
                        }
                        CodeBlockKind::Indented => String::new(),
                    };
                    block_start_line = cur_start_line;
                    in_code_block = Some((lang, String::new()));
                }
                Tag::List(start) => list_stack.push(start),
                Tag::Item => {
                    block_start_line = cur_start_line;
                    inline_buf.clear();
                    item_task_checked = None;
                }
                Tag::Table(_) => {
                    in_table = true;
                    block_start_line = cur_start_line;
                    table_headers.clear();
                    table_rows.clear();
                }
                Tag::TableHead => {
                    in_table_head = true;
                    current_row.clear();
                }
                Tag::TableRow => current_row.clear(),
                Tag::TableCell => inline_buf.clear(),
                Tag::Strong => bold_depth += 1,
                Tag::Emphasis => italic_depth += 1,
                Tag::Strikethrough => strike_depth += 1,
                Tag::Link { dest_url, .. } => {
                    active_link = Some((dest_url.to_string(), String::new()));
                }
                _ => {}
            },
            MdEvent::End(tag_end) => match tag_end {
                TagEnd::Heading(_) => {
                    if let Some(level) = in_heading.take() {
                        let spans = std::mem::take(&mut inline_buf);
                        let title = inline_plain_text(&spans);
                        blocks.push(MappedBlock {
                            start_line: block_start_line,
                            end_line: cur_end_line.max(block_start_line),
                            block: MdBlock::Heading {
                                level,
                                slug: heading_slug(&title),
                                ascii_slug: heading_ascii_slug(&title),
                                spans,
                            },
                        });
                    }
                }
                TagEnd::Paragraph => {
                    if !in_blockquote && list_stack.is_empty() && !in_table && !inline_buf.is_empty()
                    {
                        blocks.push(MappedBlock {
                            start_line: block_start_line,
                            end_line: cur_end_line.max(block_start_line),
                            block: MdBlock::Paragraph(std::mem::take(&mut inline_buf)),
                        });
                    }
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                    if !inline_buf.is_empty() {
                        blocks.push(MappedBlock {
                            start_line: block_start_line,
                            end_line: cur_end_line.max(block_start_line),
                            block: MdBlock::Blockquote(std::mem::take(&mut inline_buf)),
                        });
                    }
                }
                TagEnd::CodeBlock => {
                    if let Some((lang, mut code)) = in_code_block.take() {
                        if code.ends_with('\n') {
                            code.pop();
                        }
                        let block = if lang.eq_ignore_ascii_case("mermaid") {
                            MdBlock::Mermaid { source: code }
                        } else {
                            let spans = highlight_code_block(&lang, &code);
                            MdBlock::CodeBlock { lang, code, spans }
                        };
                        blocks.push(MappedBlock {
                            start_line: block_start_line,
                            end_line: cur_end_line.max(block_start_line),
                            block,
                        });
                    }
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                }
                TagEnd::Item => {
                    let spans = std::mem::take(&mut inline_buf);
                    let block = if let Some(checked) = item_task_checked.take() {
                        MdBlock::TaskItem { checked, spans }
                    } else if let Some(Some(num)) = list_stack.last_mut() {
                        let n = *num;
                        *num += 1;
                        MdBlock::OrderedItem {
                            number: n.to_string(),
                            spans,
                        }
                    } else {
                        let indent = list_stack.len().saturating_sub(1);
                        MdBlock::BulletItem { indent, spans }
                    };
                    blocks.push(MappedBlock {
                        start_line: block_start_line,
                        end_line: cur_end_line.max(block_start_line),
                        block,
                    });
                }
                TagEnd::TableCell => current_row.push(std::mem::take(&mut inline_buf)),
                TagEnd::TableHead => {
                    in_table_head = false;
                    table_headers = std::mem::take(&mut current_row);
                }
                TagEnd::TableRow => {
                    if !in_table_head {
                        table_rows.push(std::mem::take(&mut current_row));
                    }
                }
                TagEnd::Table => {
                    in_table = false;
                    blocks.push(MappedBlock {
                        start_line: block_start_line,
                        end_line: cur_end_line.max(block_start_line),
                        block: MdBlock::Table {
                            headers: std::mem::take(&mut table_headers),
                            rows: std::mem::take(&mut table_rows),
                        },
                    });
                }
                TagEnd::Strong => bold_depth = bold_depth.saturating_sub(1),
                TagEnd::Emphasis => italic_depth = italic_depth.saturating_sub(1),
                TagEnd::Strikethrough => strike_depth = strike_depth.saturating_sub(1),
                TagEnd::Link => {
                    if let Some((url, label)) = active_link.take() {
                        inline_buf.push(InlineSpan::Link { label, url });
                    }
                }
                _ => {}
            },
            MdEvent::Text(text) => {
                if let Some((_, ref mut code)) = in_code_block {
                    code.push_str(&text);
                } else {
                    push_text(
                        &text,
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &mut active_link,
                        &mut inline_buf,
                    );
                }
            }
            MdEvent::Code(code) => {
                if let Some((_, label)) = &mut active_link {
                    label.push_str(&code);
                } else {
                    inline_buf.push(InlineSpan::Code(code.to_string()));
                }
            }
            MdEvent::SoftBreak | MdEvent::HardBreak => {
                if let Some((_, ref mut code)) = in_code_block {
                    code.push('\n');
                } else {
                    push_text(
                        " ",
                        bold_depth,
                        italic_depth,
                        strike_depth,
                        &mut active_link,
                        &mut inline_buf,
                    );
                }
            }
            MdEvent::Rule => blocks.push(MappedBlock {
                start_line: cur_start_line,
                end_line: cur_end_line,
                block: MdBlock::HorizontalRule,
            }),
            MdEvent::TaskListMarker(checked) => item_task_checked = Some(checked),
            _ => {}
        }
    }

    blocks
}

fn heading_level_to_u8(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

pub fn parse_inline(s: &str) -> Vec<InlineSpan> {
    match parse_markdown(s).into_iter().next() {
        Some(MdBlock::Paragraph(spans)) => spans,
        _ => Vec::new(),
    }
}

fn highlight_code_block(lang_hint: &str, code: &str) -> Vec<HighlightSpan> {
    let lang = LanguageId::from_fence_name(lang_hint);
    if lang == LanguageId::PlainText || code.is_empty() {
        return Vec::new();
    }
    let rope = Rope::from_str(code);
    let mut syn = SyntaxState::new(lang);
    syn.sync(&rope, lang, 1, &[]);
    syn.highlights_in_range(&rope, 0..rope.len_bytes())
}

fn bold_font(ctx: &egui::Context, size: f32) -> FontId {
    let bold_fam = FontFamily::Name("draco_bold".into());
    let has_bold = ctx.fonts(|f| f.families().contains(&bold_fam));
    if has_bold {
        FontId::new(size, bold_fam)
    } else {
        FontId::proportional(size)
    }
}

/// PhpStorm-style compact segmented switcher: `Sync | Outline | Editor | Split | Preview`.
/// Always lays out strictly left-to-right even when placed inside a right-aligned parent.
pub fn show_markdown_mode_switcher(
    ui: &mut egui::Ui,
    mode: &mut MarkdownViewMode,
    show_toc: &mut bool,
    scroll_sync: &mut bool,
    theme: &Theme,
) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        if *mode == MarkdownViewMode::Split {
            let sync_text = RichText::new("Sync")
                .size(11.0)
                .color(if *scroll_sync {
                    theme.syn_string
                } else {
                    theme.fg_muted
                });
            if ui
                .selectable_label(*scroll_sync, sync_text)
                .on_hover_text("Bidirectional scroll sync (Editor ↔ Preview)")
                .clicked()
            {
                *scroll_sync = !*scroll_sync;
            }
        }
        if *mode != MarkdownViewMode::Editor {
            let toc_text = RichText::new("Outline")
                .size(11.0)
                .color(if *show_toc {
                    theme.accent
                } else {
                    theme.fg_muted
                });
            if ui
                .selectable_label(*show_toc, toc_text)
                .on_hover_text("Toggle floating Document Outline (Google Docs style)")
                .clicked()
            {
                *show_toc = !*show_toc;
            }
        }
        egui::Frame::NONE
            .fill(theme.bg_panel)
            .stroke(Stroke::new(1.0_f32, theme.border))
            .corner_radius(4.0)
            .inner_margin(egui::Margin::symmetric(3, 1))
            .show(ui, |ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (m, label) in [
                        (MarkdownViewMode::Editor, "Editor"),
                        (MarkdownViewMode::Split, "Split"),
                        (MarkdownViewMode::Preview, "Preview"),
                    ] {
                        let active = *mode == m;
                        let text = RichText::new(label)
                            .size(11.0)
                            .color(if active { theme.accent } else { theme.fg_muted });
                        if ui.selectable_label(active, text).clicked() {
                            *mode = m;
                        }
                    }
                });
            });
    });
}

/// Renders full-width, high-performance Markdown preview with a Google Docs-style
/// fixed, translucent, collapsible floating Outline overlay.
pub fn show_markdown_preview(
    ui: &mut egui::Ui,
    doc: &Document,
    workspace_root: Option<&Path>,
    cache: &mut MarkdownPreviewCache,
    theme: &Theme,
) -> Option<PreviewAction> {
    cache.sync(doc);

    let pane_rect = ui.available_rect_before_wrap();
    let mut action: Option<PreviewAction> = None;
    let mut clicked_url: Option<String> = None;
    let mut scroll_target = cache.scroll_to_anchor.clone();
    let mut scrolled = false;

    // Compute active heading flash fade (0.0..=1.0 over 950ms) — 0% CPU once faded out!
    let mut flash_state: Option<(String, f32)> = None;
    if let Some((slug, started)) = &cache.highlighted_heading {
        let elapsed = started.elapsed().as_secs_f32();
        if elapsed < 0.95 {
            let intensity = (1.0 - elapsed / 0.95).powi(2);
            flash_state = Some((slug.clone(), intensity));
            ui.ctx().request_repaint();
        } else {
            cache.highlighted_heading = None;
        }
    }

    // Resolve anchor jump directly from cached heading_y_positions for instant, lag-free scrolling
    if let Some(target) = cache.scroll_to_anchor.as_deref() {
        if let Some((_, y)) = cache.heading_y_positions.iter().find(|(slug, _)| {
            cache
                .headings
                .iter()
                .find(|h| &h.slug == slug)
                .map(|h| anchor_matches(target, &h.slug, &h.ascii_slug))
                .unwrap_or(false)
        }) {
            cache.pending_scroll_y = Some((*y - 8.0).max(0.0));
            cache.scroll_to_anchor = None;
        }
    }

    // Apply scroll target cleanly without fighting user wheel scroll
    let preview_is_hovered = ui.rect_contains_pointer(pane_rect);
    let forced_scroll_y = cache.pending_scroll_y.take();
    let prev_scroll_y = cache.current_scroll_y;
    let mut new_positions: Vec<BlockLayoutPos> = Vec::with_capacity(cache.mapped_blocks.len());
    let mut new_heading_ys: Vec<(String, f32)> = Vec::with_capacity(cache.headings.len());

    let reserve_right_for_toc =
        if cache.show_toc && cache.headings.len() >= 2 && pane_rect.width() > 680.0 {
            224.0_f32
        } else {
            0.0_f32
        };

    egui::Frame::NONE
        .fill(theme.bg_editor)
        .inner_margin(egui::Margin {
            left: 28,
            right: 28,
            top: 18,
            bottom: 36,
        })
        .show(ui, |ui| {
            let mut scroll_area = egui::ScrollArea::vertical()
                .id_salt(("draco_md_preview", doc.id))
                .auto_shrink([false, false]);
            if let Some(y) = forced_scroll_y {
                scroll_area = scroll_area.vertical_scroll_offset(y.max(0.0));
                cache.current_scroll_y = y.max(0.0);
            }

            let scroll_out = scroll_area.show(ui, |ui| {
                // Full responsive width (only reserves right lane when Outline is expanded on wide panes)
                let content_w = (ui.available_width() - reserve_right_for_toc).max(240.0);
                ui.set_min_width(content_w);
                ui.set_max_width(content_w);
                ui.spacing_mut().item_spacing.y = 10.0;
                let content_origin_y = ui.cursor().min.y;

                if cache.mapped_blocks.is_empty() {
                    ui.label(
                        RichText::new("Markdown Preview — start typing to see live output…")
                            .color(theme.fg_muted)
                            .italics(),
                    );
                    return;
                }

                for mapped in &cache.mapped_blocks {
                    let top_y = (ui.cursor().min.y - content_origin_y).max(0.0);
                    if let MdBlock::Heading { slug, .. } = &mapped.block {
                        new_heading_ys.push((slug.clone(), top_y));
                    }
                    render_block(
                        ui,
                        &mapped.block,
                        content_w,
                        theme,
                        &mut clicked_url,
                        scroll_target.as_deref(),
                        &mut scrolled,
                        flash_state.as_ref().map(|(s, a)| (s.as_str(), *a)),
                    );
                    let bottom_y = (ui.cursor().min.y - content_origin_y).max(top_y);
                    new_positions.push(BlockLayoutPos {
                        start_line: mapped.start_line,
                        end_line: mapped.end_line,
                        top_y,
                        bottom_y,
                    });
                }
            });

            if forced_scroll_y.is_none() {
                let actual_y = scroll_out.state.offset.y;
                if (actual_y - prev_scroll_y).abs() > 2.0 && preview_is_hovered {
                    cache.selected_outline_slug = None;
                }
                cache.current_scroll_y = actual_y;
                // Master/Follower Bidirectional Scroll Sync: ONLY drive Editor when pointer is over Preview!
                if cache.scroll_sync
                    && preview_is_hovered
                    && (actual_y - prev_scroll_y).abs() > 1.5
                    && scroll_target.is_none()
                {
                    if let Some(target_line) =
                        compute_editor_line_from_preview_y(actual_y, &cache.block_positions)
                    {
                        cache.last_editor_top_line = target_line;
                        action = Some(PreviewAction::ScrollEditorToLine(target_line));
                    }
                }
            }
        });

    if !new_positions.is_empty() {
        cache.block_positions = new_positions;
    }
    if !new_heading_ys.is_empty() {
        cache.heading_y_positions = new_heading_ys;
    }

    // Google Docs-style FIXED, TRANSLUCENT, COLLAPSIBLE floating Outline overlay
    if cache.headings.len() >= 2 && pane_rect.width() > 300.0 {
        let active_slug = cache.active_heading_slug().map(|s| s.to_string());
        let bg_glass = Color32::from_rgba_unmultiplied(
            theme.bg_popup.r(),
            theme.bg_popup.g(),
            theme.bg_popup.b(),
            212,
        );
        let border_glass = Color32::from_rgba_unmultiplied(
            theme.border.r(),
            theme.border.g(),
            theme.border.b(),
            170,
        );

        if cache.show_toc {
            let panel_w = 212.0_f32.min(pane_rect.width() * 0.42);
            let pos = Pos2::new(pane_rect.right() - panel_w - 14.0, pane_rect.top() + 12.0);
            let max_h = (pane_rect.height() - 36.0).clamp(120.0, 420.0);

            egui::Area::new(egui::Id::new(("draco_md_floating_toc", doc.id)))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE
                        .fill(bg_glass)
                        .stroke(Stroke::new(1.0_f32, border_glass))
                        .corner_radius(8.0)
                        .shadow(egui::Shadow {
                            offset: [0, 4],
                            blur: 14,
                            spread: 0,
                            color: Color32::from_black_alpha(55),
                        })
                        .inner_margin(egui::Margin::symmetric(10, 8))
                        .show(ui, |ui| {
                            ui.set_width(panel_w - 20.0);
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("OUTLINE")
                                        .size(10.5)
                                        .strong()
                                        .color(theme.fg_muted),
                                );
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui
                                            .add(
                                                egui::Button::new(
                                                    RichText::new("×")
                                                        .size(13.0)
                                                        .color(theme.fg_muted),
                                                )
                                                .frame(false),
                                            )
                                            .on_hover_text("Hide Outline")
                                            .clicked()
                                        {
                                            cache.show_toc = false;
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            egui::ScrollArea::vertical()
                                .id_salt(("draco_md_toc_scroll", doc.id))
                                .max_height(max_h)
                                .show(ui, |ui| {
                                    ui.spacing_mut().item_spacing.y = 3.0;
                                    for h in &cache.headings {
                                        let is_active =
                                            active_slug.as_deref() == Some(h.slug.as_str());
                                        let indent = h.level.saturating_sub(1) as f32 * 10.0;
                                        ui.horizontal(|ui| {
                                            ui.add_space(indent);
                                            if is_active {
                                                let r = ui.cursor();
                                                ui.painter().rect_filled(
                                                    Rect::from_min_size(
                                                        Pos2::new(r.min.x - 5.0, r.min.y + 2.0),
                                                        Vec2::new(2.0, 13.0),
                                                    ),
                                                    1.0,
                                                    theme.accent,
                                                );
                                            }
                                            let resp = ui
                                                .add(
                                                    egui::Button::new(
                                                        RichText::new(&h.title)
                                                            .size(12.0)
                                                            .color(if is_active {
                                                                theme.accent
                                                            } else {
                                                                theme.fg_text
                                                            }),
                                                    )
                                                    .frame(false)
                                                    .truncate(),
                                                )
                                                .on_hover_cursor(CursorIcon::PointingHand);
                                            if resp.clicked() {
                                                // Jump directly to heading Y offset + flash heading + sync editor line
                                                if let Some((_, hy)) = cache
                                                    .heading_y_positions
                                                    .iter()
                                                    .find(|(s, _)| s == &h.slug)
                                                {
                                                    cache.pending_scroll_y =
                                                        Some((*hy - 8.0).max(0.0));
                                                    if cache.scroll_sync {
                                                        if let Some(target_line) =
                                                            compute_editor_line_from_preview_y(
                                                                *hy,
                                                                &cache.block_positions,
                                                            )
                                                        {
                                                            cache.last_editor_top_line = target_line;
                                                            action = Some(
                                                                PreviewAction::ScrollEditorToLine(
                                                                    target_line,
                                                                ),
                                                            );
                                                        }
                                                    }
                                                } else {
                                                    scroll_target = Some(h.slug.clone());
                                                }
                                                cache.selected_outline_slug =
                                                    Some(h.slug.clone());
                                                cache.highlighted_heading =
                                                    Some((h.slug.clone(), Instant::now()));
                                                ui.ctx().request_repaint();
                                            }
                                        });
                                    }
                                });
                        });
                });
        } else {
            // Compact translucent pill button to reopen Outline
            let pos = Pos2::new(pane_rect.right() - 88.0, pane_rect.top() + 12.0);
            egui::Area::new(egui::Id::new(("draco_md_toc_pill", doc.id)))
                .fixed_pos(pos)
                .order(egui::Order::Foreground)
                .show(ui.ctx(), |ui| {
                    egui::Frame::NONE
                        .fill(bg_glass)
                        .stroke(Stroke::new(1.0_f32, border_glass))
                        .corner_radius(14.0)
                        .inner_margin(egui::Margin::symmetric(10, 4))
                        .show(ui, |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("☰ Outline")
                                            .size(11.5)
                                            .color(theme.fg_text),
                                    )
                                    .frame(false),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                cache.show_toc = true;
                            }
                        });
                });
        }
    }

    if scrolled {
        cache.scroll_to_anchor = None;
    } else if scroll_target != cache.scroll_to_anchor {
        cache.scroll_to_anchor = scroll_target;
    }

    if let Some(url) = clicked_url {
        if let Some(anchor) = url.strip_prefix('#') {
            cache.scroll_to_anchor = Some(anchor.to_string());
            cache.selected_outline_slug = Some(heading_slug(anchor));
            cache.highlighted_heading = Some((anchor.to_string(), Instant::now()));
            ui.ctx().request_repaint();
        } else {
            action = resolve_markdown_link(&url, doc.path.as_deref(), workspace_root);
        }
    }

    action
}

#[allow(clippy::too_many_arguments)]
fn render_block(
    ui: &mut egui::Ui,
    block: &MdBlock,
    content_w: f32,
    theme: &Theme,
    clicked_url: &mut Option<String>,
    scroll_target: Option<&str>,
    scrolled: &mut bool,
    flash: Option<(&str, f32)>,
) {
    match block {
        MdBlock::Heading {
            level,
            slug,
            ascii_slug,
            spans,
        } => {
            let size = match level {
                1 => 26.0,
                2 => 21.0,
                3 => 17.5,
                4 => 15.5,
                _ => 14.0,
            };
            ui.add_space(if *level <= 2 { 10.0 } else { 5.0 });
            let flash_alpha = flash
                .filter(|(target, _)| anchor_matches(target, slug, ascii_slug))
                .map(|(_, intensity)| intensity)
                .unwrap_or(0.0);

            let resp = if flash_alpha > 0.01 {
                let bg = Color32::from_rgba_unmultiplied(
                    theme.accent.r(),
                    theme.accent.g(),
                    theme.accent.b(),
                    (flash_alpha * 85.0) as u8,
                );
                let frame_out = egui::Frame::NONE
                    .fill(bg)
                    .corner_radius(5.0)
                    .inner_margin(egui::Margin::symmetric(8, 3))
                    .show(ui, |ui| {
                        ui.set_min_width((content_w - 16.0).max(120.0));
                        let r = ui.max_rect();
                        ui.painter().rect_filled(
                            Rect::from_min_size(
                                Pos2::new(r.min.x - 8.0, r.min.y - 3.0),
                                Vec2::new(3.5, r.height() + 6.0),
                            ),
                            2.0,
                            Color32::from_rgba_unmultiplied(
                                theme.accent.r(),
                                theme.accent.g(),
                                theme.accent.b(),
                                (flash_alpha * 255.0) as u8,
                            ),
                        );
                        render_inline_spans(ui, spans, size, true, theme, clicked_url)
                    });
                frame_out.inner
            } else {
                render_inline_spans(ui, spans, size, true, theme, clicked_url)
            };

            if let Some(target) = scroll_target {
                if !*scrolled && anchor_matches(target, slug, ascii_slug) {
                    resp.scroll_to_me(Some(egui::Align::TOP));
                    *scrolled = true;
                }
            }
            if *level <= 2 {
                let r = ui.allocate_exact_size(Vec2::new(content_w, 2.0), egui::Sense::hover()).0;
                let line_col = if *level == 1 {
                    theme.accent.linear_multiply(0.55)
                } else {
                    theme.border
                };
                ui.painter().line_segment(
                    [Pos2::new(r.left(), r.center().y), Pos2::new(r.right(), r.center().y)],
                    Stroke::new(if *level == 1 { 1.5_f32 } else { 1.0_f32 }, line_col),
                );
            }
        }
        MdBlock::Mermaid { source } => {
            mermaid::show_mermaid_diagram(ui, source, content_w, theme);
        }
        MdBlock::Paragraph(spans) => {
            render_inline_spans(ui, spans, 14.5, false, theme, clicked_url);
        }
        MdBlock::Blockquote(spans) => {
            egui::Frame::NONE
                .fill(theme.bg_panel)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(14, 10))
                .show(ui, |ui| {
                    ui.set_min_width((content_w - 28.0).max(120.0));
                    let rect = ui.max_rect();
                    ui.painter().rect_filled(
                        Rect::from_min_size(
                            Pos2::new(rect.min.x - 14.0, rect.min.y - 10.0),
                            Vec2::new(3.5, rect.height() + 20.0),
                        ),
                        2.0,
                        theme.accent,
                    );
                    render_inline_spans(ui, spans, 14.0, false, theme, clicked_url);
                });
        }
        MdBlock::CodeBlock { lang, code, spans } => {
            egui::Frame::NONE
                .fill(theme.bg_panel)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_min_width((content_w - 24.0).max(120.0));
                    ui.horizontal(|ui| {
                        let badge = if lang.is_empty() { "CODE" } else { lang.as_str() };
                        ui.label(
                            RichText::new(badge.to_uppercase())
                                .size(10.5)
                                .strong()
                                .color(theme.fg_muted),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Copy").size(11.0).color(theme.fg_muted),
                                    )
                                    .frame(false),
                                )
                                .on_hover_cursor(CursorIcon::PointingHand)
                                .clicked()
                            {
                                ui.ctx().copy_text(code.clone());
                            }
                        });
                    });
                    ui.separator();
                    // No nested ScrollArea -> silky smooth vertical wheel scrolling!
                    let job = build_highlighted_code_job(code, spans, theme);
                    ui.add(egui::Label::new(job).wrap());
                });
        }
        MdBlock::BulletItem { indent, spans } => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(*indent as f32 * 18.0 + 6.0);
                ui.label(RichText::new("•").color(theme.accent).size(15.0));
                render_inline_spans(ui, spans, 14.5, false, theme, clicked_url);
            });
        }
        MdBlock::OrderedItem { number, spans } => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(format!("{number}."))
                        .color(theme.accent)
                        .strong()
                        .size(14.0),
                );
                render_inline_spans(ui, spans, 14.5, false, theme, clicked_url);
            });
        }
        MdBlock::TaskItem { checked, spans } => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(6.0);
                let (mark, col) = if *checked {
                    ("☑", theme.syn_string)
                } else {
                    ("☐", theme.fg_muted)
                };
                ui.label(RichText::new(mark).color(col).size(15.0));
                render_inline_spans(ui, spans, 14.5, false, theme, clicked_url);
            });
        }
        MdBlock::Table { headers, rows } => {
            egui::Frame::NONE
                .fill(theme.bg_panel)
                .stroke(Stroke::new(1.0_f32, theme.border))
                .corner_radius(6.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.set_min_width((content_w - 16.0).max(160.0));
                    let cols = headers.len().max(1);
                    let col_w = ((content_w - 40.0) / cols as f32).max(90.0);
                    egui::Grid::new(("md_table", headers.len(), rows.len()))
                        .striped(true)
                        .min_col_width(col_w)
                        .spacing(Vec2::new(14.0, 8.0))
                        .show(ui, |ui| {
                            for h in headers {
                                render_inline_spans(ui, h, 13.5, true, theme, clicked_url);
                            }
                            ui.end_row();
                            for r in rows {
                                for cell in r {
                                    render_inline_spans(ui, cell, 13.5, false, theme, clicked_url);
                                }
                                ui.end_row();
                            }
                        });
                });
        }
        MdBlock::HorizontalRule => {
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
        }
    }
}

/// Renders inline spans; when links are present, renders each link as a clickable button.
fn render_inline_spans(
    ui: &mut egui::Ui,
    spans: &[InlineSpan],
    size: f32,
    is_heading: bool,
    theme: &Theme,
    clicked_url: &mut Option<String>,
) -> egui::Response {
    let has_links = spans
        .iter()
        .any(|s| matches!(s, InlineSpan::Link { .. }));
    if !has_links {
        let job = build_inline_job(ui.ctx(), spans, size, is_heading, theme);
        return ui.add(egui::Label::new(job).wrap());
    }

    let inner = ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let base_color = if is_heading {
            theme.syn_heading
        } else {
            theme.fg_text
        };
        let base_font = if is_heading {
            bold_font(ui.ctx(), size)
        } else {
            FontId::proportional(size)
        };

        for sp in spans {
            match sp {
                InlineSpan::Link { label, url } => {
                    let resp = ui
                        .add(
                            egui::Button::new(
                                RichText::new(label)
                                    .font(base_font.clone())
                                    .color(theme.accent)
                                    .underline(),
                            )
                            .frame(false),
                        )
                        .on_hover_cursor(CursorIcon::PointingHand)
                        .on_hover_text(url);
                    if resp.clicked() {
                        *clicked_url = Some(url.clone());
                    }
                }
                InlineSpan::Text(t) => {
                    for seg in split_words_preserve_spaces(t) {
                        ui.label(RichText::new(seg).font(base_font.clone()).color(base_color));
                    }
                }
                InlineSpan::Bold(t) => {
                    let bf = bold_font(ui.ctx(), size);
                    for seg in split_words_preserve_spaces(t) {
                        ui.label(RichText::new(seg).font(bf.clone()).color(theme.fg_text));
                    }
                }
                InlineSpan::Italic(t) => {
                    for seg in split_words_preserve_spaces(t) {
                        ui.label(
                            RichText::new(seg)
                                .font(base_font.clone())
                                .color(base_color)
                                .italics(),
                        );
                    }
                }
                InlineSpan::Strike(t) => {
                    for seg in split_words_preserve_spaces(t) {
                        ui.label(
                            RichText::new(seg)
                                .font(base_font.clone())
                                .color(theme.fg_muted)
                                .strikethrough(),
                        );
                    }
                }
                InlineSpan::Code(t) => {
                    ui.label(
                        RichText::new(format!(" {t} "))
                            .font(FontId::monospace((size - 1.5).max(11.5)))
                            .color(theme.syn_string)
                            .background_color(theme.bg_panel),
                    );
                }
            }
        }
    });
    inner.response
}

fn split_words_preserve_spaces(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b' ' {
            while i < bytes.len() && bytes[i] == b' ' {
                i += 1;
            }
            out.push(&s[start..i]);
            start = i;
        } else {
            let ch = s[i..].chars().next().unwrap();
            i += ch.len_utf8();
        }
    }
    if start < s.len() {
        out.push(&s[start..]);
    }
    out
}

fn build_inline_job(
    ctx: &egui::Context,
    spans: &[InlineSpan],
    size: f32,
    is_heading: bool,
    theme: &Theme,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let base_color = if is_heading {
        theme.syn_heading
    } else {
        theme.fg_text
    };
    let base_font = if is_heading {
        bold_font(ctx, size)
    } else {
        FontId::proportional(size)
    };

    for sp in spans {
        match sp {
            InlineSpan::Text(t) => {
                job.append(t, 0.0, TextFormat::simple(base_font.clone(), base_color));
            }
            InlineSpan::Bold(t) => {
                job.append(t, 0.0, TextFormat::simple(bold_font(ctx, size), theme.fg_text));
            }
            InlineSpan::Italic(t) => {
                let mut fmt = TextFormat::simple(base_font.clone(), base_color);
                fmt.italics = true;
                job.append(t, 0.0, fmt);
            }
            InlineSpan::Strike(t) => {
                let mut fmt = TextFormat::simple(base_font.clone(), theme.fg_muted);
                fmt.strikethrough = Stroke::new(1.0_f32, theme.fg_muted);
                job.append(t, 0.0, fmt);
            }
            InlineSpan::Code(t) => {
                let mut fmt =
                    TextFormat::simple(FontId::monospace((size - 1.5).max(11.5)), theme.syn_string);
                fmt.background = theme.bg_panel;
                job.append(&format!(" {t} "), 0.0, fmt);
            }
            InlineSpan::Link { label, .. } => {
                let mut fmt = TextFormat::simple(base_font.clone(), theme.accent);
                fmt.underline = Stroke::new(1.0_f32, theme.accent);
                job.append(label, 0.0, fmt);
            }
        }
    }
    job
}

fn build_highlighted_code_job(
    code: &str,
    spans: &[HighlightSpan],
    theme: &Theme,
) -> egui::text::LayoutJob {
    let font = FontId::monospace(13.0);
    let mut job = egui::text::LayoutJob::default();
    let mut sorted: Vec<(usize, usize, Color32)> = spans
        .iter()
        .map(|s| (s.byte_range.start, s.byte_range.end, theme.token_color(s.kind)))
        .collect();
    sorted.sort_by_key(|(s, e, _)| (*s, *e));

    let mut pos = 0usize;
    for (s, e, color) in sorted {
        let s = s.min(code.len());
        let e = e.min(code.len());
        if s > pos && code.is_char_boundary(pos) && code.is_char_boundary(s) {
            job.append(&code[pos..s], 0.0, TextFormat::simple(font.clone(), theme.fg_text));
            pos = s;
        }
        if e > pos && code.is_char_boundary(pos) && code.is_char_boundary(e) {
            job.append(&code[pos..e], 0.0, TextFormat::simple(font.clone(), color));
            pos = e;
        }
    }
    if pos < code.len() && code.is_char_boundary(pos) {
        job.append(&code[pos..], 0.0, TextFormat::simple(font, theme.fg_text));
    }
    job
}
