use vte::{Params, Perform};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TermColor {
    #[default]
    Default,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermCell {
    pub ch: char,
    pub fg: TermColor,
    pub bg: TermColor,
    pub bold: bool,
    pub underline: bool,
}

impl Default for TermCell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: TermColor::Default,
            bg: TermColor::Default,
            bold: false,
            underline: false,
        }
    }
}

pub struct TerminalGrid {
    pub cols: usize,
    pub rows: usize,
    pub cells: Vec<Vec<TermCell>>,
    pub scrollback: Vec<Vec<TermCell>>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    cur_fg: TermColor,
    cur_bg: TermColor,
    cur_bold: bool,
    cur_underline: bool,
}

impl TerminalGrid {
    pub fn new(cols: usize, rows: usize) -> Self {
        let cols = cols.max(10);
        let rows = rows.max(4);
        Self {
            cols,
            rows,
            cells: vec![vec![TermCell::default(); cols]; rows],
            scrollback: Vec::new(),
            cursor_row: 0,
            cursor_col: 0,
            cur_fg: TermColor::Default,
            cur_bg: TermColor::Default,
            cur_bold: false,
            cur_underline: false,
        }
    }

    pub fn resize(&mut self, cols: usize, rows: usize) {
        let cols = cols.max(10);
        let rows = rows.max(4);
        if cols == self.cols && rows == self.rows {
            return;
        }
        for row in &mut self.cells {
            row.resize(cols, TermCell::default());
        }
        self.cells.resize(rows, vec![TermCell::default(); cols]);
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(rows - 1);
        self.cursor_col = self.cursor_col.min(cols - 1);
    }

    fn scroll_up(&mut self) {
        if !self.cells.is_empty() {
            let top = self.cells.remove(0);
            self.scrollback.push(top);
            if self.scrollback.len() > 2000 {
                self.scrollback.remove(0);
            }
            self.cells.push(vec![TermCell::default(); self.cols]);
        }
    }

    fn newline(&mut self) {
        if self.cursor_row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor_row += 1;
        }
    }

    pub fn row_text(&self, row: usize) -> String {
        self.cells
            .get(row)
            .map(|r| r.iter().map(|c| c.ch).collect::<String>())
            .unwrap_or_default()
    }
}

impl Perform for TerminalGrid {
    fn print(&mut self, c: char) {
        if self.cursor_col >= self.cols {
            self.cursor_col = 0;
            self.newline();
        }
        if self.cursor_row < self.rows && self.cursor_col < self.cols {
            self.cells[self.cursor_row][self.cursor_col] = TermCell {
                ch: c,
                fg: self.cur_fg,
                bg: self.cur_bg,
                bold: self.cur_bold,
                underline: self.cur_underline,
            };
            self.cursor_col += 1;
        }
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => self.cursor_col = 0,
            b'\n' | 0x0b | 0x0c => self.newline(),
            0x08 => self.cursor_col = self.cursor_col.saturating_sub(1),
            b'\t' => {
                let next = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next.min(self.cols - 1);
            }
            _ => {}
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let p: Vec<u16> = params
            .iter()
            .map(|sub| sub.first().copied().unwrap_or(0))
            .collect();

        match action {
            'A' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_row = (self.cursor_row + n).min(self.rows - 1);
            }
            'C' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_col = (self.cursor_col + n).min(self.cols - 1);
            }
            'D' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            'H' | 'f' => {
                let row = p.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = p.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                self.cursor_row = row.min(self.rows - 1);
                self.cursor_col = col.min(self.cols - 1);
            }
            'J' => {
                let mode = p.first().copied().unwrap_or(0);
                match mode {
                    2 | 3 => {
                        for row in &mut self.cells {
                            row.fill(TermCell::default());
                        }
                    }
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.cells[self.cursor_row][c] = TermCell::default();
                        }
                        for r in (self.cursor_row + 1)..self.rows {
                            self.cells[r].fill(TermCell::default());
                        }
                    }
                    _ => {}
                }
            }
            'K' => {
                let mode = p.first().copied().unwrap_or(0);
                match mode {
                    0 => {
                        for c in self.cursor_col..self.cols {
                            self.cells[self.cursor_row][c] = TermCell::default();
                        }
                    }
                    1 => {
                        for c in 0..=self.cursor_col.min(self.cols - 1) {
                            self.cells[self.cursor_row][c] = TermCell::default();
                        }
                    }
                    2 => self.cells[self.cursor_row].fill(TermCell::default()),
                    _ => {}
                }
            }
            'm' => {
                if p.is_empty() {
                    self.reset_sgr();
                    return;
                }
                let mut i = 0;
                while i < p.len() {
                    match p[i] {
                        0 => self.reset_sgr(),
                        1 => self.cur_bold = true,
                        4 => self.cur_underline = true,
                        22 => self.cur_bold = false,
                        24 => self.cur_underline = false,
                        30..=37 => self.cur_fg = TermColor::Indexed((p[i] - 30) as u8),
                        39 => self.cur_fg = TermColor::Default,
                        40..=47 => self.cur_bg = TermColor::Indexed((p[i] - 40) as u8),
                        49 => self.cur_bg = TermColor::Default,
                        90..=97 => self.cur_fg = TermColor::Indexed((p[i] - 90 + 8) as u8),
                        100..=107 => self.cur_bg = TermColor::Indexed((p[i] - 100 + 8) as u8),
                        38 | 48 => {
                            let is_fg = p[i] == 38;
                            if i + 2 < p.len() && p[i + 1] == 5 {
                                let col = TermColor::Indexed(p[i + 2] as u8);
                                if is_fg { self.cur_fg = col; } else { self.cur_bg = col; }
                                i += 2;
                            } else if i + 4 < p.len() && p[i + 1] == 2 {
                                let col = TermColor::Rgb(p[i + 2] as u8, p[i + 3] as u8, p[i + 4] as u8);
                                if is_fg { self.cur_fg = col; } else { self.cur_bg = col; }
                                i += 4;
                            }
                        }
                        _ => {}
                    }
                    i += 1;
                }
            }
            _ => {}
        }
    }
}

impl TerminalGrid {
    fn reset_sgr(&mut self) {
        self.cur_fg = TermColor::Default;
        self.cur_bg = TermColor::Default;
        self.cur_bold = false;
        self.cur_underline = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ansi_color_and_cursor_control() {
        let mut grid = TerminalGrid::new(40, 10);
        let mut parser = vte::Parser::new();
        parser.advance(&mut grid, b"\x1b[32mHello\x1b[0m\r\nDraco");
        assert!(grid.row_text(0).starts_with("Hello"));
        assert_eq!(grid.cells[0][0].fg, TermColor::Indexed(2));
        assert!(grid.row_text(1).starts_with("Draco"));
        assert_eq!(grid.cells[1][0].fg, TermColor::Default);
    }

    #[test]
    fn ansi_256_truecolor_erase_scrollback_and_resize() {
        let mut grid = TerminalGrid::new(20, 4);
        let mut parser = vte::Parser::new();

        // 256-color fg + truecolor bg + bold
        parser.advance(&mut grid, b"\x1b[1;38;5;208;48;2;10;20;30mA\x1b[0m");
        assert_eq!(grid.cells[0][0].ch, 'A');
        assert_eq!(grid.cells[0][0].fg, TermColor::Indexed(208));
        assert_eq!(grid.cells[0][0].bg, TermColor::Rgb(10, 20, 30));
        assert!(grid.cells[0][0].bold);

        // Cursor positioning + erase line
        parser.advance(&mut grid, b"\x1b[2;1Hhello world\x1b[2;6H\x1b[0K");
        assert_eq!(grid.row_text(1).trim_end(), "hello");

        // Trigger scrollback by pushing more lines than grid rows
        parser.advance(&mut grid, b"\r\nL1\r\nL2\r\nL3\r\nL4\r\nL5");
        assert!(!grid.scrollback.is_empty());

        // Clear screen
        parser.advance(&mut grid, b"\x1b[2J\x1b[1;1Hclean");
        assert!(grid.row_text(0).starts_with("clean"));

        // Resize
        grid.resize(30, 8);
        assert_eq!(grid.cols, 30);
        assert_eq!(grid.rows, 8);
    }
}
