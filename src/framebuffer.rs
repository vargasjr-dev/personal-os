/// Framebuffer Graphics Driver — pixel-level graphics over VGA text mode.
///
/// VGA text mode gives us an 80×25 grid of character cells, each with a
/// foreground and background color (16-color palette). Using CP437 half-block
/// characters we can address an 80×50 pixel canvas:
///
///   0xDF (▀) — top half colored by foreground, bottom by background
///   0xDC (▄) — bottom half colored by foreground, top by background
///   0xDB (█) — full block (both halves = foreground color)
///   0x20 ( ) — empty (both halves = background color)
///
/// Each text cell (col, row) maps to two vertical pixels:
///   pixel (x, y*2)   → top half of cell (col=x, row=y)
///   pixel (x, y*2+1) → bottom half of cell (col=x, row=y)
///
/// This gives us an 80×50 logical pixel canvas with 16 colors.
///
/// The framebuffer maintains a pixel buffer in RAM and flushes to the VGA
/// buffer via `write_raw_cell`. The driver tracks dirty cells for efficient
/// partial redraws.
///
/// Primitives provided:
///   - set_pixel / get_pixel
///   - clear
///   - flush (RAM → VGA)
///   - draw_hline / draw_vline / draw_line (Bresenham)
///   - draw_rect / fill_rect
///   - draw_circle (midpoint algorithm)
///   - draw_bitmap (1bpp sprites)
///
/// This is the foundation for Phase 8 items 2–6:
///   item 2 — font rendering builds on draw_bitmap
///   item 3 — compositor uses fill_rect + layering
///   item 4 — scrollable UI uses the full canvas
///
/// Phase 8, Item 1 — Framebuffer graphics driver.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

use crate::vga_buffer::{Color, write_raw_cell};

// ─── Constants ───────────────────────────────────────────────────────────────

/// Logical pixel width (= VGA columns).
pub const FB_WIDTH: usize = 80;
/// Logical pixel height (= VGA rows × 2, using half-block chars).
pub const FB_HEIGHT: usize = 50;
/// Pixel aliases used by mouse.rs for cursor bounds checking.
pub const SCREEN_PX_W: usize = FB_WIDTH;
pub const SCREEN_PX_H: usize = FB_HEIGHT;
/// VGA text rows.
const VGA_ROWS: usize = 25;
/// VGA text columns.
const VGA_COLS: usize = 80;

// CP437 half-block bytes
const CHAR_TOP_HALF:    u8 = 0xDF; // ▀  — fg=top,  bg=bottom
const CHAR_BOTTOM_HALF: u8 = 0xDC; // ▄  — fg=bottom, bg=top
const CHAR_FULL_BLOCK:  u8 = 0xDB; // █  — fg=both
const CHAR_SPACE:       u8 = 0x20; // ' ' — bg=both

// ─── Framebuffer ─────────────────────────────────────────────────────────────

/// The pixel framebuffer — 80×50 pixels, 16-color palette.
pub struct Framebuffer {
    /// Pixel color buffer. `pixels[y][x]` = color at (x, y).
    /// Color 0 (Black) = "off" by convention.
    pixels: [[Color; FB_WIDTH]; FB_HEIGHT],
    /// Dirty flags per VGA text cell — true when pixel data changed
    /// since last flush. `dirty[row][col]` covers pixels (col, row*2)
    /// and (col, row*2+1).
    dirty: [[bool; VGA_COLS]; VGA_ROWS],
}

impl Framebuffer {
    /// Create a new framebuffer, all pixels black, all cells dirty.
    pub const fn new() -> Self {
        Framebuffer {
            pixels: [[Color::Black; FB_WIDTH]; FB_HEIGHT],
            dirty:  [[true; VGA_COLS]; VGA_ROWS],
        }
    }

    // ─── Pixel access ────────────────────────────────────────────────

    /// Set a pixel. Marks the containing VGA cell dirty.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < FB_WIDTH && y < FB_HEIGHT {
            self.pixels[y][x] = color;
            self.dirty[y / 2][x] = true;
        }
    }

    /// Read a pixel's current color.
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < FB_WIDTH && y < FB_HEIGHT {
            self.pixels[y][x]
        } else {
            Color::Black
        }
    }

    // ─── Fill ────────────────────────────────────────────────────────

    /// Fill the entire canvas with one color. Marks all cells dirty.
    pub fn clear(&mut self, color: Color) {
        for row in &mut self.pixels {
            for px in row.iter_mut() {
                *px = color;
            }
        }
        for row in &mut self.dirty {
            for d in row.iter_mut() {
                *d = true;
            }
        }
    }

    // ─── Flush ───────────────────────────────────────────────────────

    /// Flush dirty cells from the pixel buffer to the VGA text buffer.
    ///
    /// Each VGA cell covers pixels (x, row*2) and (x, row*2+1).
    /// We pick the appropriate CP437 half-block char and colors.
    pub fn flush(&mut self) {
        for row in 0..VGA_ROWS {
            for col in 0..VGA_COLS {
                if !self.dirty[row][col] {
                    continue;
                }
                let top    = self.pixels[row * 2][col];
                let bottom = self.pixels[row * 2 + 1][col];

                let (byte, fg, bg) = cell_for_pixels(top, bottom);
                write_raw_cell(row, col, byte, fg, bg);
                self.dirty[row][col] = false;
            }
        }
    }

    /// Force-flush all cells regardless of dirty state.
    pub fn flush_all(&mut self) {
        for row in &mut self.dirty {
            for d in row.iter_mut() {
                *d = true;
            }
        }
        self.flush();
    }

    // ─── Drawing primitives ──────────────────────────────────────────

    /// Draw a horizontal line from (x0, y) to (x1, y).
    pub fn draw_hline(&mut self, x0: usize, x1: usize, y: usize, color: Color) {
        let lo = x0.min(x1);
        let hi = x0.max(x1);
        for x in lo..=hi {
            self.set_pixel(x, y, color);
        }
    }

    /// Draw a vertical line from (x, y0) to (x, y1).
    pub fn draw_vline(&mut self, x: usize, y0: usize, y1: usize, color: Color) {
        let lo = y0.min(y1);
        let hi = y0.max(y1);
        for y in lo..=hi {
            self.set_pixel(x, y, color);
        }
    }

    /// Draw a line between two points using Bresenham's algorithm.
    pub fn draw_line(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, color: Color) {
        // Use signed arithmetic for Bresenham
        let mut x0 = x0 as i32;
        let mut y0 = y0 as i32;
        let x1 = x1 as i32;
        let y1 = y1 as i32;

        let dx =  (x1 - x0).abs();
        let dy = -(y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as usize, y0 as usize, color);
            }
            if x0 == x1 && y0 == y1 { break; }
            let e2 = 2 * err;
            if e2 >= dy { err += dy; x0 += sx; }
            if e2 <= dx { err += dx; y0 += sy; }
        }
    }

    /// Draw a rectangle outline.
    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if w == 0 || h == 0 { return; }
        self.draw_hline(x, x + w - 1, y, color);
        self.draw_hline(x, x + w - 1, y + h - 1, color);
        self.draw_vline(x, y, y + h - 1, color);
        self.draw_vline(x + w - 1, y, y + h - 1, color);
    }

    /// Fill a solid rectangle.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for dy in 0..h {
            self.draw_hline(x, x + w.saturating_sub(1), y + dy, color);
        }
    }

    /// Draw a circle outline using the midpoint circle algorithm.
    pub fn draw_circle(&mut self, cx: usize, cy: usize, r: usize, color: Color) {
        let cx = cx as i32;
        let cy = cy as i32;
        let mut x = r as i32;
        let mut y = 0i32;
        let mut err = 0i32;

        while x >= y {
            self.plot_circle_points(cx, cy, x, y, color);
            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    fn plot_circle_points(&mut self, cx: i32, cy: i32, x: i32, y: i32, color: Color) {
        let points = [
            (cx + x, cy + y), (cx - x, cy + y),
            (cx + x, cy - y), (cx - x, cy - y),
            (cx + y, cy + x), (cx - y, cy + x),
            (cx + y, cy - x), (cx - y, cy - x),
        ];
        for (px, py) in points {
            if px >= 0 && py >= 0 && (px as usize) < FB_WIDTH && (py as usize) < FB_HEIGHT {
                self.set_pixel(px as usize, py as usize, color);
            }
        }
    }

    /// Draw a 1bpp bitmap at (x, y). Each row is a `u8` bitmask (MSB = left).
    /// `1` bits draw `fg`, `0` bits draw `bg` (use Color::Black to leave clear).
    pub fn draw_bitmap(
        &mut self,
        x: usize,
        y: usize,
        width: usize,
        data: &[u8],
        fg: Color,
        bg: Color,
    ) {
        let height = data.len();
        for (row, &byte) in data.iter().enumerate() {
            for bit in 0..width.min(8) {
                let set = (byte >> (7 - bit)) & 1 == 1;
                let color = if set { fg } else { bg };
                self.set_pixel(x + bit, y + row, color);
            }
        }
        let _ = height; // suppress unused warning
    }

    // ─── Compositing helpers ─────────────────────────────────────────

    /// Fill a region with a two-color checkerboard pattern.
    /// Useful for "transparent" backgrounds and debugging.
    pub fn fill_checkerboard(&mut self, x: usize, y: usize, w: usize, h: usize,
                              c0: Color, c1: Color) {
        for dy in 0..h {
            for dx in 0..w {
                let color = if (dx + dy) % 2 == 0 { c0 } else { c1 };
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Draw a simple progress bar at (x, y) with width `w`, filled `pct` (0–100).
    pub fn draw_progress_bar(&mut self, x: usize, y: usize, w: usize,
                              pct: usize, fg: Color, bg: Color, border: Color) {
        let fill = (w * pct.min(100)) / 100;
        self.draw_rect(x, y, w, 3, border);
        if fill > 2 {
            self.fill_rect(x + 1, y + 1, fill - 2, 1, fg);
        }
        if w - fill > 2 {
            self.fill_rect(x + fill, y + 1, w - fill - 1, 1, bg);
        }
    }

    // ─── Diagnostics ─────────────────────────────────────────────────

    /// Return the number of dirty cells awaiting flush.
    pub fn dirty_count(&self) -> usize {
        self.dirty.iter().flat_map(|row| row.iter()).filter(|&&d| d).count()
    }

    /// Describe the framebuffer state (for serial debugging).
    pub fn describe(&self) -> String {
        format!(
            "Framebuffer {}×{} px ({}×{} cells), {} dirty",
            FB_WIDTH, FB_HEIGHT,
            VGA_COLS, VGA_ROWS,
            self.dirty_count()
        )
    }
}

// ─── Cell encoding ───────────────────────────────────────────────────────────

/// Given top-pixel color and bottom-pixel color, choose the CP437 char
/// and fg/bg colors that best represent the pair.
///
/// Rules:
///   top == bottom == Black  → space,       fg=Black,  bg=Black
///   top == bottom != Black  → full block,  fg=color,  bg=color
///   top != Black, bottom == Black → ▀ (0xDF), fg=top, bg=Black
///   top == Black, bottom != Black → ▄ (0xDC), fg=bottom, bg=Black
///   top != bottom, both set → ▀ (0xDF), fg=top, bg=bottom
fn cell_for_pixels(top: Color, bottom: Color) -> (u8, Color, Color) {
    match (top, bottom) {
        // Both black → blank space
        (Color::Black, Color::Black) => (CHAR_SPACE, Color::Black, Color::Black),
        // Same non-black → solid block
        (t, b) if t as u8 == b as u8 => (CHAR_FULL_BLOCK, t, t),
        // Top set, bottom black → ▀
        (t, Color::Black) => (CHAR_TOP_HALF, t, Color::Black),
        // Top black, bottom set → ▄
        (Color::Black, b) => (CHAR_BOTTOM_HALF, b, Color::Black),
        // Both set, different → ▀ (fg=top, bg=bottom)
        (t, b) => (CHAR_TOP_HALF, t, b),
    }
}

// ─── Global instance ─────────────────────────────────────────────────────────

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Global framebuffer instance — access via `FRAMEBUFFER.lock()`.
    pub static ref FRAMEBUFFER: Mutex<Framebuffer> = Mutex::new(Framebuffer::new());
}

/// Convenience: clear the global framebuffer and flush.
pub fn fb_clear(color: Color) {
    let mut fb = FRAMEBUFFER.lock();
    fb.clear(color);
    fb.flush();
}

/// Convenience: set a pixel and immediately flush just its cell.
pub fn fb_set_pixel(x: usize, y: usize, color: Color) {
    let mut fb = FRAMEBUFFER.lock();
    fb.set_pixel(x, y, color);
    // Flush only the affected cell
    let row = y / 2;
    let col = x;
    let top    = fb.pixels[row * 2][col];
    let bottom = fb.pixels[row * 2 + 1][col];
    let (byte, fg, bg) = cell_for_pixels(top, bottom);
    drop(fb);
    write_raw_cell(row, col, byte, fg, bg);
}

// ─── Demo ────────────────────────────────────────────────────────────────────

/// Draw a demo scene to prove the framebuffer works.
///
/// Renders:
///   - Solid color border around the canvas
///   - Filled rectangle in the upper-left quadrant
///   - Diagonal lines crossing the center
///   - Circle in the lower-right quadrant
///   - Progress bar at the bottom
pub fn draw_demo() {
    let mut fb = FRAMEBUFFER.lock();
    fb.clear(Color::Black);

    // Canvas border
    fb.draw_rect(0, 0, FB_WIDTH, FB_HEIGHT, Color::White);

    // Filled rectangle — upper-left quadrant
    fb.fill_rect(2, 2, 20, 12, Color::Blue);
    fb.draw_rect(2, 2, 20, 12, Color::LightBlue);

    // Diagonal cross through the center
    fb.draw_line(0, 0, FB_WIDTH - 1, FB_HEIGHT - 1, Color::LightGreen);
    fb.draw_line(FB_WIDTH - 1, 0, 0, FB_HEIGHT - 1, Color::Green);

    // Circle — lower-right quadrant
    fb.draw_circle(60, 38, 10, Color::Yellow);

    // Progress bar — near bottom
    fb.draw_progress_bar(5, 45, 70, 65, Color::LightCyan, Color::DarkGray, Color::White);

    fb.flush();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fb() -> Framebuffer {
        Framebuffer::new()
    }

    #[test_case]
    fn test_new_all_black() {
        let fb = make_fb();
        assert_eq!(fb.get_pixel(0, 0) as u8, Color::Black as u8);
        assert_eq!(fb.get_pixel(79, 49) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_set_get_pixel() {
        let mut fb = make_fb();
        fb.set_pixel(10, 20, Color::Red);
        assert_eq!(fb.get_pixel(10, 20) as u8, Color::Red as u8);
    }

    #[test_case]
    fn test_out_of_bounds_silent() {
        let mut fb = make_fb();
        fb.set_pixel(100, 100, Color::Red); // should not panic
        assert_eq!(fb.get_pixel(100, 100) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_clear_sets_all() {
        let mut fb = make_fb();
        fb.clear(Color::Blue);
        assert_eq!(fb.get_pixel(0, 0) as u8, Color::Blue as u8);
        assert_eq!(fb.get_pixel(40, 25) as u8, Color::Blue as u8);
        assert_eq!(fb.get_pixel(79, 49) as u8, Color::Blue as u8);
    }

    #[test_case]
    fn test_dirty_on_set_pixel() {
        let mut fb = make_fb();
        // After new(), all cells are dirty
        assert_eq!(fb.dirty_count(), VGA_ROWS * VGA_COLS);
        // After clear and marking all dirty again (clear does this)
        fb.clear(Color::Black);
        assert_eq!(fb.dirty_count(), VGA_ROWS * VGA_COLS);
    }

    #[test_case]
    fn test_cell_encoding_both_black() {
        let (byte, fg, bg) = cell_for_pixels(Color::Black, Color::Black);
        assert_eq!(byte, CHAR_SPACE);
        assert_eq!(fg as u8, Color::Black as u8);
        assert_eq!(bg as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_cell_encoding_both_same() {
        let (byte, fg, _bg) = cell_for_pixels(Color::Red, Color::Red);
        assert_eq!(byte, CHAR_FULL_BLOCK);
        assert_eq!(fg as u8, Color::Red as u8);
    }

    #[test_case]
    fn test_cell_encoding_top_only() {
        let (byte, fg, bg) = cell_for_pixels(Color::Green, Color::Black);
        assert_eq!(byte, CHAR_TOP_HALF);
        assert_eq!(fg as u8, Color::Green as u8);
        assert_eq!(bg as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_cell_encoding_bottom_only() {
        let (byte, fg, bg) = cell_for_pixels(Color::Black, Color::Blue);
        assert_eq!(byte, CHAR_BOTTOM_HALF);
        assert_eq!(fg as u8, Color::Blue as u8);
        assert_eq!(bg as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_cell_encoding_different_colors() {
        let (byte, fg, bg) = cell_for_pixels(Color::Red, Color::Blue);
        assert_eq!(byte, CHAR_TOP_HALF);
        assert_eq!(fg as u8, Color::Red as u8);
        assert_eq!(bg as u8, Color::Blue as u8);
    }

    #[test_case]
    fn test_draw_hline() {
        let mut fb = make_fb();
        fb.draw_hline(5, 15, 10, Color::White);
        for x in 5..=15 {
            assert_eq!(fb.get_pixel(x, 10) as u8, Color::White as u8);
        }
        assert_eq!(fb.get_pixel(4, 10) as u8, Color::Black as u8);
        assert_eq!(fb.get_pixel(16, 10) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_draw_vline() {
        let mut fb = make_fb();
        fb.draw_vline(20, 5, 15, Color::Cyan);
        for y in 5..=15 {
            assert_eq!(fb.get_pixel(20, y) as u8, Color::Cyan as u8);
        }
    }

    #[test_case]
    fn test_fill_rect() {
        let mut fb = make_fb();
        fb.fill_rect(10, 10, 5, 5, Color::Yellow);
        for dy in 0..5 {
            for dx in 0..5 {
                assert_eq!(fb.get_pixel(10 + dx, 10 + dy) as u8, Color::Yellow as u8);
            }
        }
        // Outside should be untouched
        assert_eq!(fb.get_pixel(9, 10) as u8, Color::Black as u8);
        assert_eq!(fb.get_pixel(15, 10) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_draw_rect_outline_only() {
        let mut fb = make_fb();
        fb.draw_rect(5, 5, 10, 8, Color::Magenta);
        // Corners exist
        assert_eq!(fb.get_pixel(5, 5) as u8, Color::Magenta as u8);
        assert_eq!(fb.get_pixel(14, 12) as u8, Color::Magenta as u8);
        // Interior is empty
        assert_eq!(fb.get_pixel(8, 8) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_draw_bitmap() {
        let mut fb = make_fb();
        // A small 'L' shape: top pixel + left column
        // Row 0: 10000000 = 0x80
        // Row 1: 10000000 = 0x80
        // Row 2: 11000000 = 0xC0
        let bitmap = [0x80u8, 0x80, 0xC0];
        fb.draw_bitmap(0, 0, 2, &bitmap, Color::White, Color::Black);
        assert_eq!(fb.get_pixel(0, 0) as u8, Color::White as u8);
        assert_eq!(fb.get_pixel(1, 0) as u8, Color::Black as u8);
        assert_eq!(fb.get_pixel(0, 1) as u8, Color::White as u8);
        assert_eq!(fb.get_pixel(0, 2) as u8, Color::White as u8);
        assert_eq!(fb.get_pixel(1, 2) as u8, Color::White as u8);
    }

    #[test_case]
    fn test_describe() {
        let fb = make_fb();
        let desc = fb.describe();
        assert!(desc.contains("80"));
        assert!(desc.contains("50"));
    }

    #[test_case]
    fn test_fb_dimensions() {
        assert_eq!(FB_WIDTH, 80);
        assert_eq!(FB_HEIGHT, 50);
        assert_eq!(FB_HEIGHT, VGA_ROWS * 2);
    }
}
