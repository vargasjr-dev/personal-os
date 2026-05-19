/// Bitmap Font Rendering — text on the framebuffer canvas.
///
/// Embeds the classic 8×8 CP437 bitmap font (ASCII 32–126).
/// Each glyph is 8 bytes: one per pixel row, MSB = leftmost pixel.
///
/// Character cell size: 8×8 pixels on the 80×50 logical framebuffer.
/// That gives a text grid of 10 columns × 6 rows on the pixel canvas
/// (80÷8=10, 50÷8=6), but the TextCanvas struct lets you place text
/// anywhere on the pixel grid with pixel-accurate positioning.
///
/// Text rendering API:
///   render_char(fb, x, y, ch, fg, bg)   — one glyph at pixel (x, y)
///   render_str(fb, x, y, s, fg, bg)     — string left-to-right
///   render_str_wrap(fb, x, y, w, s, ...) — word-wrap within width
///
/// TextCanvas — higher-level cursor-based text surface:
///   TextCanvas::new(x, y, cols, rows)   — define a text region
///   canvas.print(s, fg, bg)             — print at cursor, auto-wrap
///   canvas.println(s, fg, bg)           — print + newline
///   canvas.clear(bg)                    — fill region with bg color
///   canvas.goto(col, row)               — move cursor
///
/// Foundation for Phase 8 item 3 (compositor) and item 4 (scrollable UI).
///
/// Phase 8, Item 2 — Font rendering (bitmap fonts).

use alloc::string::String;
use alloc::vec::Vec;

use crate::vga_buffer::Color;
use crate::framebuffer::Framebuffer;

// ─── Font constants ───────────────────────────────────────────────────────────

/// Width of each glyph in pixels.
pub const GLYPH_W: usize = 8;
/// Height of each glyph in pixels.
pub const GLYPH_H: usize = 8;
/// First ASCII code in the font table.
const FONT_FIRST: u8 = 0x20; // space
/// Last ASCII code in the font table.
const FONT_LAST: u8 = 0x7E;  // ~

// ─── Glyph data ──────────────────────────────────────────────────────────────

/// 8×8 bitmap font — classic IBM VGA style, ASCII 0x20–0x7E.
/// Each entry is 8 bytes: one per row, MSB = left pixel.
/// Index 0 = space (0x20), index 94 = tilde (0x7E).
static FONT8X8: [[u8; 8]; 95] = [
    // 0x20  ' '
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 0x21  '!'
    [0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x18, 0x00],
    // 0x22  '"'
    [0x66, 0x66, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 0x23  '#'
    [0x36, 0x36, 0x7F, 0x36, 0x7F, 0x36, 0x36, 0x00],
    // 0x24  '$'
    [0x18, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x18, 0x00],
    // 0x25  '%'
    [0x62, 0x66, 0x0C, 0x18, 0x30, 0x66, 0x46, 0x00],
    // 0x26  '&'
    [0x3C, 0x66, 0x3C, 0x38, 0x67, 0x66, 0x3F, 0x00],
    // 0x27  '\''
    [0x18, 0x18, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 0x28  '('
    [0x0C, 0x18, 0x30, 0x30, 0x30, 0x18, 0x0C, 0x00],
    // 0x29  ')'
    [0x30, 0x18, 0x0C, 0x0C, 0x0C, 0x18, 0x30, 0x00],
    // 0x2A  '*'
    [0x00, 0x66, 0x3C, 0xFF, 0x3C, 0x66, 0x00, 0x00],
    // 0x2B  '+'
    [0x00, 0x18, 0x18, 0x7E, 0x18, 0x18, 0x00, 0x00],
    // 0x2C  ','
    [0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x30, 0x00],
    // 0x2D  '-'
    [0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00],
    // 0x2E  '.'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00],
    // 0x2F  '/'
    [0x00, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x00, 0x00],
    // 0x30  '0'
    [0x3C, 0x66, 0x6E, 0x76, 0x66, 0x66, 0x3C, 0x00],
    // 0x31  '1'
    [0x18, 0x38, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
    // 0x32  '2'
    [0x3C, 0x66, 0x06, 0x0C, 0x18, 0x30, 0x7E, 0x00],
    // 0x33  '3'
    [0x3C, 0x66, 0x06, 0x1C, 0x06, 0x66, 0x3C, 0x00],
    // 0x34  '4'
    [0x0C, 0x1C, 0x3C, 0x6C, 0x7E, 0x0C, 0x0C, 0x00],
    // 0x35  '5'
    [0x7E, 0x60, 0x7C, 0x06, 0x06, 0x66, 0x3C, 0x00],
    // 0x36  '6'
    [0x1C, 0x30, 0x60, 0x7C, 0x66, 0x66, 0x3C, 0x00],
    // 0x37  '7'
    [0x7E, 0x06, 0x0C, 0x18, 0x18, 0x18, 0x18, 0x00],
    // 0x38  '8'
    [0x3C, 0x66, 0x66, 0x3C, 0x66, 0x66, 0x3C, 0x00],
    // 0x39  '9'
    [0x3C, 0x66, 0x66, 0x3E, 0x06, 0x0C, 0x38, 0x00],
    // 0x3A  ':'
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x00, 0x00],
    // 0x3B  ';'
    [0x00, 0x18, 0x18, 0x00, 0x18, 0x18, 0x30, 0x00],
    // 0x3C  '<'
    [0x0C, 0x18, 0x30, 0x60, 0x30, 0x18, 0x0C, 0x00],
    // 0x3D  '='
    [0x00, 0x00, 0x7E, 0x00, 0x7E, 0x00, 0x00, 0x00],
    // 0x3E  '>'
    [0x60, 0x30, 0x18, 0x0C, 0x18, 0x30, 0x60, 0x00],
    // 0x3F  '?'
    [0x3C, 0x66, 0x0C, 0x18, 0x18, 0x00, 0x18, 0x00],
    // 0x40  '@'
    [0x3E, 0x63, 0x6F, 0x69, 0x6F, 0x60, 0x3C, 0x00],
    // 0x41  'A'
    [0x18, 0x3C, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
    // 0x42  'B'
    [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
    // 0x43  'C'
    [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
    // 0x44  'D'
    [0x78, 0x6C, 0x66, 0x66, 0x66, 0x6C, 0x78, 0x00],
    // 0x45  'E'
    [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x7E, 0x00],
    // 0x46  'F'
    [0x7E, 0x60, 0x60, 0x78, 0x60, 0x60, 0x60, 0x00],
    // 0x47  'G'
    [0x3C, 0x66, 0x60, 0x6E, 0x66, 0x66, 0x3C, 0x00],
    // 0x48  'H'
    [0x66, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x66, 0x00],
    // 0x49  'I'
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00],
    // 0x4A  'J'
    [0x06, 0x06, 0x06, 0x06, 0x06, 0x66, 0x3C, 0x00],
    // 0x4B  'K'
    [0x66, 0x6C, 0x78, 0x70, 0x78, 0x6C, 0x66, 0x00],
    // 0x4C  'L'
    [0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x7E, 0x00],
    // 0x4D  'M'
    [0x63, 0x77, 0x7F, 0x6B, 0x63, 0x63, 0x63, 0x00],
    // 0x4E  'N'
    [0x66, 0x76, 0x7E, 0x7E, 0x6E, 0x66, 0x66, 0x00],
    // 0x4F  'O'
    [0x3C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
    // 0x50  'P'
    [0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x00],
    // 0x51  'Q'
    [0x3C, 0x66, 0x66, 0x66, 0x6E, 0x3C, 0x06, 0x00],
    // 0x52  'R'
    [0x7C, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x00],
    // 0x53  'S'
    [0x3C, 0x66, 0x60, 0x3C, 0x06, 0x66, 0x3C, 0x00],
    // 0x54  'T'
    [0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00],
    // 0x55  'U'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x00],
    // 0x56  'V'
    [0x66, 0x66, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
    // 0x57  'W'
    [0x63, 0x63, 0x63, 0x6B, 0x7F, 0x77, 0x63, 0x00],
    // 0x58  'X'
    [0x66, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x66, 0x00],
    // 0x59  'Y'
    [0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x00],
    // 0x5A  'Z'
    [0x7E, 0x06, 0x0C, 0x18, 0x30, 0x60, 0x7E, 0x00],
    // 0x5B  '['
    [0x3C, 0x30, 0x30, 0x30, 0x30, 0x30, 0x3C, 0x00],
    // 0x5C  '\'
    [0x00, 0x60, 0x30, 0x18, 0x0C, 0x06, 0x00, 0x00],
    // 0x5D  ']'
    [0x3C, 0x0C, 0x0C, 0x0C, 0x0C, 0x0C, 0x3C, 0x00],
    // 0x5E  '^'
    [0x18, 0x3C, 0x66, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 0x5F  '_'
    [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xFF],
    // 0x60  '`'
    [0x30, 0x18, 0x0C, 0x00, 0x00, 0x00, 0x00, 0x00],
    // 0x61  'a'
    [0x00, 0x00, 0x3C, 0x06, 0x3E, 0x66, 0x3E, 0x00],
    // 0x62  'b'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x7C, 0x00],
    // 0x63  'c'
    [0x00, 0x00, 0x3C, 0x60, 0x60, 0x60, 0x3C, 0x00],
    // 0x64  'd'
    [0x06, 0x06, 0x3E, 0x66, 0x66, 0x66, 0x3E, 0x00],
    // 0x65  'e'
    [0x00, 0x00, 0x3C, 0x66, 0x7E, 0x60, 0x3C, 0x00],
    // 0x66  'f'
    [0x1C, 0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x00],
    // 0x67  'g'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x3C],
    // 0x68  'h'
    [0x60, 0x60, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00],
    // 0x69  'i'
    [0x18, 0x00, 0x38, 0x18, 0x18, 0x18, 0x3C, 0x00],
    // 0x6A  'j'
    [0x06, 0x00, 0x0E, 0x06, 0x06, 0x06, 0x66, 0x3C],
    // 0x6B  'k'
    [0x60, 0x60, 0x66, 0x6C, 0x78, 0x6C, 0x66, 0x00],
    // 0x6C  'l'
    [0x38, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00],
    // 0x6D  'm'
    [0x00, 0x00, 0x36, 0x7F, 0x6B, 0x63, 0x63, 0x00],
    // 0x6E  'n'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x00],
    // 0x6F  'o'
    [0x00, 0x00, 0x3C, 0x66, 0x66, 0x66, 0x3C, 0x00],
    // 0x70  'p'
    [0x00, 0x00, 0x7C, 0x66, 0x66, 0x7C, 0x60, 0x60],
    // 0x71  'q'
    [0x00, 0x00, 0x3E, 0x66, 0x66, 0x3E, 0x06, 0x06],
    // 0x72  'r'
    [0x00, 0x00, 0x6C, 0x76, 0x60, 0x60, 0x60, 0x00],
    // 0x73  's'
    [0x00, 0x00, 0x3E, 0x60, 0x3C, 0x06, 0x7C, 0x00],
    // 0x74  't'
    [0x30, 0x30, 0x7C, 0x30, 0x30, 0x30, 0x1C, 0x00],
    // 0x75  'u'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x66, 0x3E, 0x00],
    // 0x76  'v'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x00],
    // 0x77  'w'
    [0x00, 0x00, 0x63, 0x6B, 0x7F, 0x36, 0x36, 0x00],
    // 0x78  'x'
    [0x00, 0x00, 0x66, 0x3C, 0x18, 0x3C, 0x66, 0x00],
    // 0x79  'y'
    [0x00, 0x00, 0x66, 0x66, 0x66, 0x3E, 0x06, 0x3C],
    // 0x7A  'z'
    [0x00, 0x00, 0x7E, 0x0C, 0x18, 0x30, 0x7E, 0x00],
    // 0x7B  '{'
    [0x0E, 0x18, 0x18, 0x70, 0x18, 0x18, 0x0E, 0x00],
    // 0x7C  '|'
    [0x18, 0x18, 0x18, 0x00, 0x18, 0x18, 0x18, 0x00],
    // 0x7D  '}'
    [0x70, 0x18, 0x18, 0x0E, 0x18, 0x18, 0x70, 0x00],
    // 0x7E  '~'
    [0x00, 0x00, 0x32, 0x4C, 0x00, 0x00, 0x00, 0x00],
];

/// Fallback glyph for characters outside the font table (solid 5×7 box).
static GLYPH_FALLBACK: [u8; 8] = [0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x7E, 0x00];

// ─── Glyph lookup ────────────────────────────────────────────────────────────

/// Return the 8-byte glyph data for an ASCII character.
/// Falls back to a solid box for unmapped codepoints.
pub fn glyph(ch: u8) -> &'static [u8; 8] {
    if ch >= FONT_FIRST && ch <= FONT_LAST {
        &FONT8X8[(ch - FONT_FIRST) as usize]
    } else {
        &GLYPH_FALLBACK
    }
}

// ─── Rendering primitives ────────────────────────────────────────────────────

/// Render a single character at pixel position (x, y).
///
/// The glyph is 8×8 pixels. `fg` is the ink color; `bg` is the
/// background (use `Color::Black` for transparent-feel over dark canvas).
pub fn render_char(fb: &mut Framebuffer, x: usize, y: usize, ch: char,
                   fg: Color, bg: Color) {
    let g = glyph(ch as u8);
    fb.draw_bitmap(x, y, GLYPH_W, g, fg, bg);
}

/// Render a string starting at pixel position (x, y), left-to-right.
/// Characters advance by GLYPH_W pixels. No wrapping.
pub fn render_str(fb: &mut Framebuffer, x: usize, y: usize, s: &str,
                  fg: Color, bg: Color) {
    let mut cx = x;
    for ch in s.chars() {
        if cx + GLYPH_W > crate::framebuffer::FB_WIDTH { break; }
        render_char(fb, cx, y, ch, fg, bg);
        cx += GLYPH_W;
    }
}

/// Render a string with word-wrap within a pixel width `max_w`.
/// Returns the y coordinate after the last rendered line.
pub fn render_str_wrap(fb: &mut Framebuffer, x: usize, y: usize,
                       max_w: usize, s: &str,
                       fg: Color, bg: Color) -> usize {
    let mut cx = x;
    let mut cy = y;
    for ch in s.chars() {
        if ch == '\n' {
            cx = x;
            cy += GLYPH_H;
            continue;
        }
        if cx + GLYPH_W > x + max_w {
            cx = x;
            cy += GLYPH_H;
        }
        if cy + GLYPH_H > crate::framebuffer::FB_HEIGHT { break; }
        render_char(fb, cx, cy, ch, fg, bg);
        cx += GLYPH_W;
    }
    cy
}

// ─── TextCanvas ──────────────────────────────────────────────────────────────

/// A cursor-based text surface mapped onto a region of the framebuffer.
///
/// Provides a familiar print/println API for rendering text into a
/// defined rectangular area (measured in character cells).
///
/// Character cell = GLYPH_W × GLYPH_H pixels (8×8).
pub struct TextCanvas {
    /// Pixel x of the top-left corner.
    px: usize,
    /// Pixel y of the top-left corner.
    py: usize,
    /// Width in character columns.
    cols: usize,
    /// Height in character rows.
    rows: usize,
    /// Current cursor column (0-indexed).
    cursor_col: usize,
    /// Current cursor row (0-indexed).
    cursor_row: usize,
}

impl TextCanvas {
    /// Create a new TextCanvas anchored at pixel (px, py), sized in cells.
    pub fn new(px: usize, py: usize, cols: usize, rows: usize) -> Self {
        TextCanvas { px, py, cols, rows, cursor_col: 0, cursor_row: 0 }
    }

    /// Move the cursor to a specific cell position.
    pub fn goto(&mut self, col: usize, row: usize) {
        self.cursor_col = col.min(self.cols.saturating_sub(1));
        self.cursor_row = row.min(self.rows.saturating_sub(1));
    }

    /// Fill the entire canvas background with `bg` color.
    pub fn clear(&self, fb: &mut Framebuffer, bg: Color) {
        fb.fill_rect(self.px, self.py,
                     self.cols * GLYPH_W,
                     self.rows * GLYPH_H,
                     bg);
    }

    /// Print a string at the cursor position, advancing the cursor.
    /// Auto-wraps at the right edge; stops at the bottom edge.
    pub fn print(&mut self, fb: &mut Framebuffer, s: &str, fg: Color, bg: Color) {
        for ch in s.chars() {
            if ch == '\n' {
                self.newline();
                continue;
            }
            if self.cursor_col >= self.cols {
                self.newline();
            }
            if self.cursor_row >= self.rows { break; }

            let px = self.px + self.cursor_col * GLYPH_W;
            let py = self.py + self.cursor_row * GLYPH_H;
            render_char(fb, px, py, ch, fg, bg);
            self.cursor_col += 1;
        }
    }

    /// Print a string followed by a newline.
    pub fn println(&mut self, fb: &mut Framebuffer, s: &str, fg: Color, bg: Color) {
        self.print(fb, s, fg, bg);
        self.newline();
    }

    /// Print a right-aligned string on the current row.
    pub fn print_right(&mut self, fb: &mut Framebuffer, s: &str, fg: Color, bg: Color) {
        let len = s.chars().count();
        if len <= self.cols {
            self.cursor_col = self.cols - len;
        }
        self.print(fb, s, fg, bg);
    }

    /// Advance to the start of the next row.
    fn newline(&mut self) {
        self.cursor_col = 0;
        self.cursor_row += 1;
    }

    /// Current cursor column.
    pub fn col(&self) -> usize { self.cursor_col }
    /// Current cursor row.
    pub fn row(&self) -> usize { self.cursor_row }
    /// Canvas width in columns.
    pub fn cols(&self) -> usize { self.cols }
    /// Canvas height in rows.
    pub fn rows(&self) -> usize { self.rows }
}

// ─── Convenience: render on the global framebuffer ───────────────────────────

/// Render a string directly to the global framebuffer, flush immediately.
pub fn fb_print(x: usize, y: usize, s: &str, fg: Color, bg: Color) {
    let mut fb = crate::framebuffer::FRAMEBUFFER.lock();
    render_str(&mut fb, x, y, s, fg, bg);
    fb.flush();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_glyph_space_all_zero() {
        let g = glyph(b' ');
        assert!(g.iter().all(|&b| b == 0));
    }

    #[test_case]
    fn test_glyph_digit_zero_not_empty() {
        let g = glyph(b'0');
        assert!(g.iter().any(|&b| b != 0));
    }

    #[test_case]
    fn test_glyph_capital_a_not_empty() {
        let g = glyph(b'A');
        assert!(g.iter().any(|&b| b != 0));
    }

    #[test_case]
    fn test_glyph_fallback_for_control() {
        // ASCII 0x01 is below FONT_FIRST — should return fallback
        let g = glyph(0x01);
        assert_eq!(g as *const _, &GLYPH_FALLBACK as *const _);
    }

    #[test_case]
    fn test_glyph_tilde() {
        let g = glyph(b'~');
        // Should not panic and should have some data
        let _ = g;
    }

    #[test_case]
    fn test_glyph_full_table_accessible() {
        // Every ASCII char 0x20..=0x7E must return without panic
        for ch in 0x20u8..=0x7Eu8 {
            let _ = glyph(ch);
        }
    }

    #[test_case]
    fn test_glyph_h_has_horizontal_bar() {
        // 'H' row 3 (0-indexed) should be 0x7E (all bits set = horizontal bar)
        let g = glyph(b'H');
        assert_eq!(g[3], 0x7E);
    }

    #[test_case]
    fn test_render_char_sets_pixels() {
        use crate::framebuffer::Framebuffer;
        let mut fb = Framebuffer::new();
        // Render '0' at (0,0) — first row is 0x3C = 00111100
        render_char(&mut fb, 0, 0, '0', Color::White, Color::Black);
        // Bit 2 of row 0 (0x3C = 00111100) → pixel (2,0) should be White
        assert_eq!(fb.get_pixel(2, 0) as u8, Color::White as u8);
        // Bit 0 of row 0 → pixel (0,0) should be Black
        assert_eq!(fb.get_pixel(0, 0) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_render_str_advances_x() {
        use crate::framebuffer::Framebuffer;
        let mut fb = Framebuffer::new();
        render_str(&mut fb, 0, 0, "AB", Color::White, Color::Black);
        // Second char 'B' starts at x=8; its first row is 0x7C = 01111100
        // Bit 1 → pixel (9, 0) should be White
        assert_eq!(fb.get_pixel(9, 0) as u8, Color::White as u8);
    }

    #[test_case]
    fn test_text_canvas_new() {
        let canvas = TextCanvas::new(0, 0, 10, 6);
        assert_eq!(canvas.cols(), 10);
        assert_eq!(canvas.rows(), 6);
        assert_eq!(canvas.col(), 0);
        assert_eq!(canvas.row(), 0);
    }

    #[test_case]
    fn test_text_canvas_goto() {
        let mut canvas = TextCanvas::new(0, 0, 10, 6);
        canvas.goto(3, 2);
        assert_eq!(canvas.col(), 3);
        assert_eq!(canvas.row(), 2);
    }

    #[test_case]
    fn test_text_canvas_goto_clamps() {
        let mut canvas = TextCanvas::new(0, 0, 5, 3);
        canvas.goto(100, 100);
        assert!(canvas.col() < 5);
        assert!(canvas.row() < 3);
    }

    #[test_case]
    fn test_text_canvas_print_advances_cursor() {
        use crate::framebuffer::Framebuffer;
        let mut fb = Framebuffer::new();
        let mut canvas = TextCanvas::new(0, 0, 10, 6);
        canvas.print(&mut fb, "Hi", Color::White, Color::Black);
        assert_eq!(canvas.col(), 2);
        assert_eq!(canvas.row(), 0);
    }

    #[test_case]
    fn test_text_canvas_newline() {
        use crate::framebuffer::Framebuffer;
        let mut fb = Framebuffer::new();
        let mut canvas = TextCanvas::new(0, 0, 10, 6);
        canvas.println(&mut fb, "Hi", Color::White, Color::Black);
        assert_eq!(canvas.col(), 0);
        assert_eq!(canvas.row(), 1);
    }

    #[test_case]
    fn test_glyph_size() {
        assert_eq!(GLYPH_W, 8);
        assert_eq!(GLYPH_H, 8);
    }

    #[test_case]
    fn test_font_table_size() {
        // Should cover ASCII 0x20–0x7E = 95 characters
        assert_eq!(FONT8X8.len(), 95);
    }
}
