use draco::terminal::grid::{TermColor, TerminalGrid};

#[test]
fn cursor_movement_clamping_tabs_and_backspace() {
    let mut grid = TerminalGrid::new(20, 6);
    let mut parser = vte::Parser::new();

    // Tab advances to col 8, backspace moves left 1
    parser.advance(&mut grid, b"ab\tX\x08Y");
    assert_eq!(grid.cells[0][0].ch, 'a');
    assert_eq!(grid.cells[0][1].ch, 'b');
    assert_eq!(grid.cells[0][8].ch, 'Y');

    // Cursor up/left cannot underflow below 0
    parser.advance(&mut grid, b"\x1b[99A\x1b[99D");
    assert_eq!(grid.cursor_row, 0);
    assert_eq!(grid.cursor_col, 0);

    // Cursor down/right clamps to grid bounds
    parser.advance(&mut grid, b"\x1b[99B\x1b[99C");
    assert_eq!(grid.cursor_row, 5);
    assert_eq!(grid.cursor_col, 19);
}

#[test]
fn line_wrap_erase_modes_and_scrollback_cap() {
    let mut grid = TerminalGrid::new(10, 4);
    let mut parser = vte::Parser::new();

    // Writing 12 chars wraps onto next line
    parser.advance(&mut grid, b"0123456789AB");
    assert_eq!(grid.row_text(0), "0123456789");
    assert!(grid.row_text(1).starts_with("AB"));

    // Erase entire line (2K)
    parser.advance(&mut grid, b"\x1b[1;1H\x1b[2K");
    assert_eq!(grid.row_text(0).trim(), "");

    // Bright ANSI colors (90..97, 100..107) and underline
    parser.advance(&mut grid, b"\x1b[4;91;102mZ\x1b[24;39;49m");
    assert_eq!(grid.cells[0][0].ch, 'Z');
    assert_eq!(grid.cells[0][0].fg, TermColor::Indexed(9));
    assert_eq!(grid.cells[0][0].bg, TermColor::Indexed(10));
    assert!(grid.cells[0][0].underline);

    // Scrollback is capped at 2000 lines
    for _ in 0..2050 {
        parser.advance(&mut grid, b"\n");
    }
    assert!(grid.scrollback.len() <= 2000);
}
