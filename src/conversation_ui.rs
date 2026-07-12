/// Scrollable Conversation UI — formatted message history on LAYER_CHAT.
///
/// Renders a scrollable conversation pane using the compositor, framebuffer,
/// and bitmap font modules. Messages are stored in a ring buffer; the visible
/// window scrolls through them with pixel-accurate positioning.
///
/// Layout (LAYER_CHAT: 80×42 pixels, font: 8×8 glyphs):
///   Text area: 10 cols × 5 rows (80÷8=10, 40÷8=5 — using 40 of 42 px)
///   Left margin: 0 px (full width)
///   Each message:
///     Row 1: role prefix  "[you]" | "[vargas-jr]" | "[sys]"
///     Rows 2+: text content, word-wrapped at 10 chars per line
///     Final row: blank separator
///
/// Scroll model:
///   scroll_offset — number of logical lines scrolled up from the bottom.
///   0 = newest messages visible (default, bottom-anchored).
///   Each tick of scroll_up()/scroll_down() moves by SCROLL_STEP lines.
///
/// Color coding:
///   [you]       — LightGreen  (user input)
///   [vargas-jr] — LightCyan   (assistant response)
///   [sys]       — Yellow      (system/kernel messages)
///
/// Integration:
///   ConversationUI owns the message store and renders to the compositor.
///   The boot_message() in main.rs can replace chat_view with this UI.
///   The intent system fires re-renders after each response.
///
/// Phase 8, Item 4 — Scrollable conversation UI with formatted responses.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::format;

use crate::vga_buffer::Color;
use crate::font::{self, GLYPH_W, GLYPH_H};
use crate::compositor::{Compositor, LAYER_CHAT};

// ─── Layout constants ─────────────────────────────────────────────────────────

/// Pixel width of the chat layer.
pub const CHAT_PX_W: usize = 80;
/// Pixel height of the chat layer.
pub const CHAT_PX_H: usize = 42;
/// Character columns available in the chat layer.
pub const CHAT_COLS: usize = CHAT_PX_W / GLYPH_W; // 10
/// Visible text rows in the chat layer.
pub const CHAT_ROWS: usize = CHAT_PX_H / GLYPH_H; // 5
/// Lines scrolled per scroll_up/scroll_down call.
pub const SCROLL_STEP: usize = 1;
/// Maximum messages held in the ring buffer before oldest are dropped.
pub const MAX_MESSAGES: usize = 64;

// ─── Message ─────────────────────────────────────────────────────────────────

/// Role of a conversation participant.
#[derive(Debug, Clone, PartialEq)]
pub enum Role {
    /// The human user.
    User,
    /// The assistant (VargasJR kernel AI).
    Assistant,
    /// Kernel/system notification.
    System,
}

impl Role {
    /// Short display prefix (fits in CHAT_COLS).
    pub fn prefix(&self) -> &'static str {
        match self {
            Role::User      => "[you]",
            Role::Assistant => "[jr]",
            Role::System    => "[sys]",
        }
    }

    /// Color for the prefix label.
    pub fn prefix_color(&self) -> Color {
        match self {
            Role::User      => Color::LightGreen,
            Role::Assistant => Color::LightCyan,
            Role::System    => Color::Yellow,
        }
    }

    /// Color for the message body text.
    pub fn body_color(&self) -> Color {
        match self {
            Role::User      => Color::White,
            Role::Assistant => Color::LightGray,
            Role::System    => Color::Brown,
        }
    }
}

/// A single conversation message.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: Role,
    pub text: String,
}

impl Message {
    pub fn user(text: &str) -> Self {
        Message { role: Role::User, text: String::from(text) }
    }

    pub fn assistant(text: &str) -> Self {
        Message { role: Role::Assistant, text: String::from(text) }
    }

    pub fn system(text: &str) -> Self {
        Message { role: Role::System, text: String::from(text) }
    }
}

// ─── Rendered line ────────────────────────────────────────────────────────────

/// A single rendered line — text + color, ready to paint.
#[derive(Clone)]
struct RenderedLine {
    text:  String,
    color: Color,
    /// True for blank separator lines (no text painted, just spacing).
    is_blank: bool,
}

impl RenderedLine {
    fn text(s: &str, color: Color) -> Self {
        RenderedLine { text: String::from(s), color, is_blank: false }
    }
    fn blank() -> Self {
        RenderedLine { text: String::new(), color: Color::Black, is_blank: true }
    }
}

// ─── ConversationUI ───────────────────────────────────────────────────────────

/// Scrollable conversation UI — manages messages and renders to LAYER_CHAT.
pub struct ConversationUI {
    /// Message history (oldest first).
    messages: Vec<Message>,
    /// Lines scrolled up from the bottom (0 = newest visible).
    scroll_offset: usize,
    /// Cached rendered lines (re-built when messages change).
    rendered: Vec<RenderedLine>,
    /// Whether rendered cache is stale.
    dirty: bool,
}

impl ConversationUI {
    /// Create a new empty conversation UI.
    pub fn new() -> Self {
        ConversationUI {
            messages: Vec::new(),
            scroll_offset: 0,
            rendered: Vec::new(),
            dirty: true,
        }
    }

    // ─── Message management ──────────────────────────────────────────

    /// Append a message. Drops the oldest if over MAX_MESSAGES.
    pub fn push(&mut self, msg: Message) {
        if self.messages.len() >= MAX_MESSAGES {
            self.messages.remove(0);
        }
        self.messages.push(msg);
        self.dirty = true;
        // Auto-scroll to bottom on new message
        self.scroll_offset = 0;
    }

    /// Number of messages in history.
    pub fn message_count(&self) -> usize { self.messages.len() }

    /// Clear all messages.
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.scroll_offset = 0;
        self.dirty = true;
    }

    // ─── Scrolling ───────────────────────────────────────────────────

    /// Scroll up (toward older messages) by SCROLL_STEP lines.
    pub fn scroll_up(&mut self) {
        let max_scroll = self.max_scroll_offset();
        if self.scroll_offset < max_scroll {
            self.scroll_offset = (self.scroll_offset + SCROLL_STEP).min(max_scroll);
        }
    }

    /// Scroll down (toward newer messages) by SCROLL_STEP lines.
    pub fn scroll_down(&mut self) {
        self.scroll_offset = self.scroll_offset.saturating_sub(SCROLL_STEP);
    }

    /// Jump to the bottom (newest messages).
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }

    /// Jump to the top (oldest messages).
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = self.max_scroll_offset();
    }

    /// Current scroll offset.
    pub fn scroll_offset(&self) -> usize { self.scroll_offset }

    /// Maximum scroll offset (lines above bottom that can be scrolled to).
    fn max_scroll_offset(&self) -> usize {
        let total = self.total_rendered_lines();
        if total > CHAT_ROWS { total - CHAT_ROWS } else { 0 }
    }

    /// Total rendered lines across all messages.
    fn total_rendered_lines(&self) -> usize {
        if self.dirty {
            // Estimate without rebuilding cache
            self.messages.iter()
                .map(|m| 1 + word_wrap_count(&m.text, CHAT_COLS) + 1)
                .sum()
        } else {
            self.rendered.len()
        }
    }

    // ─── Rendering ───────────────────────────────────────────────────

    /// Rebuild the rendered-lines cache from messages.
    fn rebuild_cache(&mut self) {
        self.rendered.clear();
        for msg in &self.messages {
            // Prefix line
            self.rendered.push(RenderedLine::text(
                msg.role.prefix(),
                msg.role.prefix_color(),
            ));
            // Body lines (word-wrapped)
            for line in word_wrap(&msg.text, CHAT_COLS) {
                self.rendered.push(RenderedLine::text(&line, msg.role.body_color()));
            }
            // Blank separator
            self.rendered.push(RenderedLine::blank());
        }
        self.dirty = false;
    }

    /// Render the visible window of the conversation onto `LAYER_CHAT`.
    ///
    /// Paints directly into the compositor's chat layer using the font module.
    /// Call `compositor.flush()` after this to push to the VGA display.
    pub fn render(&mut self, comp: &mut Compositor) {
        if self.dirty {
            self.rebuild_cache();
        }

        let total_lines = self.rendered.len();

        // Determine which lines are visible
        // Bottom-anchored: last CHAT_ROWS lines are shown when scroll_offset=0
        let visible_start = if total_lines > CHAT_ROWS {
            let bottom_start = total_lines - CHAT_ROWS;
            if self.scroll_offset <= bottom_start {
                bottom_start - self.scroll_offset
            } else {
                0
            }
        } else {
            0
        };
        let visible_end = (visible_start + CHAT_ROWS).min(total_lines);

        // Paint onto LAYER_CHAT
        if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
            // Clear the chat layer
            layer.fill(Color::Black);

            let mut row_px = 0usize;
            for line_idx in visible_start..visible_end {
                if row_px + GLYPH_H > CHAT_PX_H { break; }

                let line = &self.rendered[line_idx];
                if !line.is_blank {
                    paint_text_line(layer, 0, row_px, &line.text, line.color);
                }
                row_px += GLYPH_H;
            }
        }
    }

    /// Convenience: push a message and re-render immediately.
    pub fn push_and_render(&mut self, msg: Message, comp: &mut Compositor) {
        self.push(msg);
        self.render(comp);
    }

    // ─── Scroll indicator ────────────────────────────────────────────

    /// Paint a tiny scroll indicator in the top-right corner of the chat layer.
    /// Shows "↑" when scrolled up, nothing when at bottom.
    pub fn render_scroll_indicator(&self, comp: &mut Compositor) {
        if self.scroll_offset == 0 { return; }
        if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
            let indicator = format!("^{}", self.scroll_offset);
            let x = CHAT_PX_W.saturating_sub(indicator.len() * GLYPH_W);
            paint_text_line(layer, x, 0, &indicator, Color::DarkGray);
        }
    }

    // ─── Diagnostics ─────────────────────────────────────────────────

    pub fn describe(&self) -> String {
        format!(
            "ConversationUI: {} msgs, {} rendered lines, scroll={}/{}",
            self.messages.len(),
            if self.dirty { 0 } else { self.rendered.len() },
            self.scroll_offset,
            self.max_scroll_offset()
        )
    }
}

// ─── Text rendering helpers ───────────────────────────────────────────────────

/// Paint a single text line at pixel (x, y) in the compositor layer
/// using the bitmap font.
fn paint_text_line(
    layer: &mut crate::compositor::Layer,
    x: usize, y: usize,
    text: &str,
    color: Color,
) {
    let mut cx = x;
    for ch in text.chars() {
        if cx + GLYPH_W > CHAT_PX_W { break; }
        let glyph = font::glyph(ch as u8);
        for (row, &byte) in glyph.iter().enumerate() {
            for bit in 0..GLYPH_W {
                let set = (byte >> (7 - bit)) & 1 == 1;
                if set {
                    layer.set_pixel(cx + bit, y + row, color);
                }
            }
        }
        cx += GLYPH_W;
    }
}

/// Word-wrap `text` at `max_chars` per line. Returns a Vec of line strings.
/// Splits on spaces; long words are hard-broken at `max_chars`.
pub fn word_wrap(text: &str, max_chars: usize) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();
    if max_chars == 0 { return lines; }

    let mut current = String::new();
    for word in text.split_whitespace() {
        // Hard-break words longer than max_chars
        let mut w = word;
        while w.len() > max_chars {
            let (chunk, rest) = w.split_at(max_chars);
            if !current.is_empty() {
                lines.push(current.clone());
                current.clear();
            }
            lines.push(String::from(chunk));
            w = rest;
        }
        if w.is_empty() { continue; }

        let needed = if current.is_empty() { w.len() } else { current.len() + 1 + w.len() };
        if needed > max_chars && !current.is_empty() {
            lines.push(current.clone());
            current.clear();
        }
        if !current.is_empty() { current.push(' '); }
        current.push_str(w);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}

/// Count how many wrapped lines `text` produces at `max_chars` per line.
pub fn word_wrap_count(text: &str, max_chars: usize) -> usize {
    word_wrap(text, max_chars).len()
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── word_wrap tests ─────────────────────────────────────────────

    #[test_case]
    fn test_wrap_empty() {
        let lines = word_wrap("", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "");
    }

    #[test_case]
    fn test_wrap_fits_on_one_line() {
        let lines = word_wrap("hello", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hello");
    }

    #[test_case]
    fn test_wrap_two_words_fitting() {
        let lines = word_wrap("hi there", 10);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "hi there");
    }

    #[test_case]
    fn test_wrap_forces_new_line() {
        // "hello world" = 11 chars, doesn't fit in 10
        let lines = word_wrap("hello world", 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "hello");
        assert_eq!(lines[1], "world");
    }

    #[test_case]
    fn test_wrap_long_word_hard_break() {
        // "abcdefghijklmno" = 15 chars, break at 10
        let lines = word_wrap("abcdefghijklmno", 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "abcdefghij");
        assert_eq!(lines[1], "klmno");
    }

    #[test_case]
    fn test_wrap_multiple_lines() {
        let lines = word_wrap("one two three four five", 10);
        // "one two" = 7, "three" = 5 so "one two th" no — let's check:
        // "one" → current="one"
        // "two" → "one two"=7 fits
        // "three" → "one two three"=13>10, so push "one two", current="three"
        // "four" → "three four"=10 fits exactly
        // "five" → "three four five"=15>10, push "three four", current="five"
        // end: push "five"
        assert!(lines.len() >= 3);
    }

    #[test_case]
    fn test_wrap_count_matches_wrap() {
        let text = "hello world how are you doing today";
        assert_eq!(word_wrap_count(text, 10), word_wrap(text, 10).len());
    }

    // ─── Role tests ──────────────────────────────────────────────────

    #[test_case]
    fn test_role_prefix_user() {
        assert_eq!(Role::User.prefix(), "[you]");
    }

    #[test_case]
    fn test_role_prefix_assistant() {
        assert_eq!(Role::Assistant.prefix(), "[jr]");
    }

    #[test_case]
    fn test_role_prefix_system() {
        assert_eq!(Role::System.prefix(), "[sys]");
    }

    // ─── Message tests ───────────────────────────────────────────────

    #[test_case]
    fn test_message_constructors() {
        let u = Message::user("hi");
        assert_eq!(u.role, Role::User);
        let a = Message::assistant("hello");
        assert_eq!(a.role, Role::Assistant);
        let s = Message::system("boot");
        assert_eq!(s.role, Role::System);
    }

    // ─── ConversationUI tests ─────────────────────────────────────────

    #[test_case]
    fn test_new_empty() {
        let ui = ConversationUI::new();
        assert_eq!(ui.message_count(), 0);
        assert_eq!(ui.scroll_offset(), 0);
    }

    #[test_case]
    fn test_push_message() {
        let mut ui = ConversationUI::new();
        ui.push(Message::user("hello"));
        assert_eq!(ui.message_count(), 1);
    }

    #[test_case]
    fn test_push_resets_scroll() {
        let mut ui = ConversationUI::new();
        for i in 0..20 {
            ui.push(Message::system(&format!("msg {}", i)));
        }
        ui.scroll_to_top();
        assert!(ui.scroll_offset() > 0);
        ui.push(Message::user("new"));
        assert_eq!(ui.scroll_offset(), 0);
    }

    #[test_case]
    fn test_max_messages_cap() {
        let mut ui = ConversationUI::new();
        for i in 0..(MAX_MESSAGES + 5) {
            ui.push(Message::system(&format!("msg {}", i)));
        }
        assert_eq!(ui.message_count(), MAX_MESSAGES);
    }

    #[test_case]
    fn test_clear_messages() {
        let mut ui = ConversationUI::new();
        ui.push(Message::user("hi"));
        ui.push(Message::assistant("hello"));
        ui.clear_messages();
        assert_eq!(ui.message_count(), 0);
        assert_eq!(ui.scroll_offset(), 0);
    }

    #[test_case]
    fn test_scroll_up_increases_offset() {
        let mut ui = ConversationUI::new();
        // Fill with enough messages to scroll
        for i in 0..20 {
            ui.push(Message::system(&format!("message number {}", i)));
        }
        let initial = ui.scroll_offset();
        ui.scroll_up();
        assert!(ui.scroll_offset() > initial);
    }

    #[test_case]
    fn test_scroll_down_decreases_offset() {
        let mut ui = ConversationUI::new();
        for i in 0..20 {
            ui.push(Message::system(&format!("msg {}", i)));
        }
        ui.scroll_to_top();
        let top = ui.scroll_offset();
        ui.scroll_down();
        assert!(ui.scroll_offset() < top);
    }

    #[test_case]
    fn test_scroll_to_bottom() {
        let mut ui = ConversationUI::new();
        for i in 0..20 {
            ui.push(Message::system(&format!("msg {}", i)));
        }
        ui.scroll_to_top();
        ui.scroll_to_bottom();
        assert_eq!(ui.scroll_offset(), 0);
    }

    #[test_case]
    fn test_scroll_capped_at_zero_bottom() {
        let mut ui = ConversationUI::new();
        ui.push(Message::user("hi"));
        // Already at bottom, scroll down should stay at 0
        ui.scroll_down();
        assert_eq!(ui.scroll_offset(), 0);
    }

    #[test_case]
    fn test_describe() {
        let mut ui = ConversationUI::new();
        ui.push(Message::user("hello"));
        let _desc = ui.describe();
        assert_eq!(ui.messages.len(), 1);
    }

    #[test_case]
    fn test_chat_cols_constant() {
        assert_eq!(CHAT_COLS, 10);
    }

    #[test_case]
    fn test_chat_rows_constant() {
        assert_eq!(CHAT_ROWS, 5);
    }
}
