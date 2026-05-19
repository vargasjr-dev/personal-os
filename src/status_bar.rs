/// System Status Bar — persistent HUD on LAYER_STATUS_BAR.
///
/// The status bar occupies the bottom 8 pixel rows of the screen
/// (LAYER_STATUS_BAR: 80×8 px at y=42). With 8×8 glyphs this is
/// exactly one row of text across 10 character columns.
///
/// Layout — 10 columns divided into three segments:
///
///   Cols 0–2   [POS]  — OS label, always DarkGray bg / White fg
///   Col  3     [ ]    — separator pixel (LightGray divider line)
///   Cols 4–6   [net]  — network indicator, Green when up / Red when down
///   Col  7     [ ]    — separator
///   Cols 8–9   [ai]   — LLM backend indicator, Cyan when up / Brown when unknown
///
/// Each segment is a colored background zone with text rendered on top.
/// Vertical dividers are 1-pixel-wide LightGray lines.
///
/// Data fields updated between renders:
///   network_up    — true when virtio-net link is active
///   llm_up        — true when last Anthropic API call succeeded
///   tick_count    — kernel uptime in agent cycles (incremented by boot loop)
///   ui_mode_name  — current UI mode name from ui_mutability (≤3 chars)
///
/// Integration:
///   StatusBar::render(&mut self, comp: &mut Compositor) paints LAYER_STATUS_BAR.
///   Boot loop calls inc_tick() each cycle and re-renders.
///   Network/LLM state updates come from virtio_net and anthropic modules.
///
/// Phase 8, Item 5 — System status bar (time, network, LLM backend).

use alloc::string::String;
use alloc::string::ToString;
use alloc::format;

use crate::vga_buffer::Color;
use crate::font::{self, GLYPH_W, GLYPH_H};
use crate::compositor::{Compositor, LAYER_STATUS_BAR};

// ─── Layout constants ─────────────────────────────────────────────────────────

/// Pixel width of the status bar layer.
pub const BAR_PX_W: usize = 80;
/// Pixel height of the status bar layer (= 1 glyph row).
pub const BAR_PX_H: usize = 8;
/// Number of character columns.
pub const BAR_COLS: usize = BAR_PX_W / GLYPH_W; // 10

// Segment boundaries (in character columns)
const SEG_OS_START:  usize = 0;
const SEG_OS_END:    usize = 3;   // cols 0–2: OS label (3 chars)
const SEG_DIV1:      usize = 3;   // col 3: divider pixel (1 char wide)
const SEG_NET_START: usize = 4;
const SEG_NET_END:   usize = 7;   // cols 4–6: network (3 chars)
const SEG_DIV2:      usize = 7;   // col 7: divider pixel
const SEG_AI_START:  usize = 8;
const SEG_AI_END:    usize = 10;  // cols 8–9: AI/LLM (2 chars)

// ─── NetworkState ─────────────────────────────────────────────────────────────

/// Network link state.
#[derive(Debug, Clone, PartialEq)]
pub enum NetworkState {
    /// virtio-net link active.
    Up,
    /// No link / device not found.
    Down,
    /// Probing / unknown.
    Unknown,
}

impl NetworkState {
    pub fn label(&self) -> &'static str {
        match self {
            NetworkState::Up      => "N:U",
            NetworkState::Down    => "N:D",
            NetworkState::Unknown => "N:?",
        }
    }
    pub fn color(&self) -> Color {
        match self {
            NetworkState::Up      => Color::LightGreen,
            NetworkState::Down    => Color::LightRed,
            NetworkState::Unknown => Color::Brown,
        }
    }
}

// ─── LlmState ────────────────────────────────────────────────────────────────

/// LLM backend state (Anthropic API).
#[derive(Debug, Clone, PartialEq)]
pub enum LlmState {
    /// Last API call succeeded.
    Up,
    /// Last API call failed.
    Error,
    /// No API calls attempted yet.
    Idle,
}

impl LlmState {
    pub fn label(&self) -> &'static str {
        match self {
            LlmState::Up    => "AI",
            LlmState::Error => "A!",
            LlmState::Idle  => "A?",
        }
    }
    pub fn color(&self) -> Color {
        match self {
            LlmState::Up    => Color::LightCyan,
            LlmState::Error => Color::LightRed,
            LlmState::Idle  => Color::DarkGray,
        }
    }
}

// ─── StatusBar ────────────────────────────────────────────────────────────────

/// The system status bar — holds state and renders to LAYER_STATUS_BAR.
pub struct StatusBar {
    /// Network link state.
    pub network: NetworkState,
    /// LLM backend state.
    pub llm: LlmState,
    /// Kernel agent-cycle tick counter (uptime proxy).
    pub tick_count: u64,
    /// Current UI mode abbreviation (max 3 chars).
    pub ui_mode: String,
    /// Whether the bar needs a repaint.
    dirty: bool,
}

impl StatusBar {
    /// Create a new status bar with default (unknown/idle) state.
    pub fn new() -> Self {
        StatusBar {
            network:    NetworkState::Unknown,
            llm:        LlmState::Idle,
            tick_count: 0,
            ui_mode:    String::from("POS"),
            dirty:      true,
        }
    }

    // ─── State updates ───────────────────────────────────────────────

    /// Mark network as up.
    pub fn set_network_up(&mut self) {
        if self.network != NetworkState::Up {
            self.network = NetworkState::Up;
            self.dirty = true;
        }
    }

    /// Mark network as down.
    pub fn set_network_down(&mut self) {
        if self.network != NetworkState::Down {
            self.network = NetworkState::Down;
            self.dirty = true;
        }
    }

    /// Mark LLM backend as up (last call succeeded).
    pub fn set_llm_up(&mut self) {
        if self.llm != LlmState::Up {
            self.llm = LlmState::Up;
            self.dirty = true;
        }
    }

    /// Mark LLM backend as errored.
    pub fn set_llm_error(&mut self) {
        if self.llm != LlmState::Error {
            self.llm = LlmState::Error;
            self.dirty = true;
        }
    }

    /// Increment the tick counter (call each agent cycle).
    pub fn inc_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        self.dirty = true;
    }

    /// Set the UI mode abbreviation (truncated to 3 chars for display).
    pub fn set_ui_mode(&mut self, mode: &str) {
        let abbrev: String = mode.chars().take(3).collect();
        if abbrev != self.ui_mode {
            self.ui_mode = abbrev;
            self.dirty = true;
        }
    }

    /// Force a repaint next render call.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    // ─── Rendering ───────────────────────────────────────────────────

    /// Render the status bar onto LAYER_STATUS_BAR in the compositor.
    /// Only repaints if dirty. Call `comp.flush()` after to push to VGA.
    pub fn render(&mut self, comp: &mut Compositor) {
        if !self.dirty { return; }

        if let Some(layer) = comp.layer_mut(LAYER_STATUS_BAR) {
            // Base: fill entire bar with DarkGray
            layer.fill(Color::DarkGray);

            // ── OS segment (cols 0–2) ───────────────────────────────
            let os_label = self.ui_mode_padded();
            paint_segment(layer, SEG_OS_START, &os_label, Color::White, Color::DarkGray);

            // ── Divider 1 (col 3) ───────────────────────────────────
            paint_divider(layer, SEG_DIV1);

            // ── Network segment (cols 4–6) ──────────────────────────
            let net_label = self.network.label();
            let net_fg    = self.network.color();
            paint_segment(layer, SEG_NET_START, net_label, net_fg, Color::DarkGray);

            // ── Divider 2 (col 7) ───────────────────────────────────
            paint_divider(layer, SEG_DIV2);

            // ── AI segment (cols 8–9) ───────────────────────────────
            let ai_label = self.llm.label();
            let ai_fg    = self.llm.color();
            paint_segment(layer, SEG_AI_START, ai_label, ai_fg, Color::DarkGray);

            // ── Tick flash: dim bottom pixel row when tick is odd ───
            // Subtle "heartbeat" — every odd tick dims the bottom strip
            if self.tick_count % 2 == 1 {
                for x in 0..BAR_PX_W {
                    layer.set_pixel(x, BAR_PX_H - 1, Color::Black);
                }
            }
        }

        self.dirty = false;
    }

    /// Render unconditionally (force repaint).
    pub fn redraw(&mut self, comp: &mut Compositor) {
        self.dirty = true;
        self.render(comp);
    }

    // ─── Helpers ─────────────────────────────────────────────────────

    /// OS label padded/truncated to exactly SEG_OS_END chars.
    fn ui_mode_padded(&self) -> String {
        let len = self.ui_mode.len();
        let target = SEG_OS_END - SEG_OS_START; // 3
        if len >= target {
            self.ui_mode.chars().take(target).collect()
        } else {
            let mut s = self.ui_mode.clone();
            for _ in 0..(target - len) { s.push(' '); }
            s
        }
    }

    // ─── Diagnostics ─────────────────────────────────────────────────

    pub fn describe(&self) -> String {
        format!(
            "StatusBar: mode={} net={:?} llm={:?} tick={} dirty={}",
            self.ui_mode,
            self.network,
            self.llm,
            self.tick_count,
            self.dirty
        )
    }
}

// ─── Segment rendering helpers ────────────────────────────────────────────────

/// Paint a text string at column `start_col` in the status bar layer.
/// Text is rendered at y=0 using the bitmap font.
fn paint_segment(
    layer: &mut crate::compositor::Layer,
    start_col: usize,
    text: &str,
    fg: Color,
    bg: Color,
) {
    let x = start_col * GLYPH_W;

    // Fill background for the segment
    let char_count = text.chars().count();
    for i in 0..char_count {
        let cx = x + i * GLYPH_W;
        for py in 0..BAR_PX_H {
            for px in 0..GLYPH_W {
                if cx + px < BAR_PX_W {
                    layer.set_pixel(cx + px, py, bg);
                }
            }
        }
    }

    // Render glyphs
    let mut cx = x;
    for ch in text.chars() {
        if cx + GLYPH_W > BAR_PX_W { break; }
        let glyph = font::glyph(ch as u8);
        for (row, &byte) in glyph.iter().enumerate() {
            if row >= BAR_PX_H { break; }
            for bit in 0..GLYPH_W {
                let set = (byte >> (7 - bit)) & 1 == 1;
                if set && cx + bit < BAR_PX_W {
                    layer.set_pixel(cx + bit, row, fg);
                }
            }
        }
        cx += GLYPH_W;
    }
}

/// Paint a 1-column-wide vertical divider line at `col`.
fn paint_divider(layer: &mut crate::compositor::Layer, col: usize) {
    let x = col * GLYPH_W;
    // Full-height light gray line at left edge of this column
    for py in 0..BAR_PX_H {
        if x < BAR_PX_W {
            layer.set_pixel(x, py, Color::LightGray);
        }
    }
    // Fill rest of column with DarkGray (spacer)
    for px in 1..GLYPH_W {
        for py in 0..BAR_PX_H {
            if x + px < BAR_PX_W {
                layer.set_pixel(x + px, py, Color::DarkGray);
            }
        }
    }
}

// ─── Global status bar ───────────────────────────────────────────────────────

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Global system status bar instance.
    pub static ref STATUS_BAR: Mutex<StatusBar> = Mutex::new(StatusBar::new());
}

/// Convenience: update network state and re-render the bar.
pub fn notify_network_up(comp: &mut Compositor) {
    let mut bar = STATUS_BAR.lock();
    bar.set_network_up();
    bar.render(comp);
}

/// Convenience: update LLM state and re-render the bar.
pub fn notify_llm_up(comp: &mut Compositor) {
    let mut bar = STATUS_BAR.lock();
    bar.set_llm_up();
    bar.render(comp);
}

/// Convenience: tick the counter and re-render the bar.
pub fn tick(comp: &mut Compositor) {
    let mut bar = STATUS_BAR.lock();
    bar.inc_tick();
    bar.render(comp);
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bar() -> StatusBar { StatusBar::new() }

    #[test_case]
    fn test_new_defaults() {
        let bar = make_bar();
        assert_eq!(bar.network, NetworkState::Unknown);
        assert_eq!(bar.llm,     LlmState::Idle);
        assert_eq!(bar.tick_count, 0);
        assert!(bar.dirty);
    }

    #[test_case]
    fn test_set_network_up() {
        let mut bar = make_bar();
        bar.set_network_up();
        assert_eq!(bar.network, NetworkState::Up);
        assert!(bar.dirty);
    }

    #[test_case]
    fn test_set_network_down() {
        let mut bar = make_bar();
        bar.set_network_down();
        assert_eq!(bar.network, NetworkState::Down);
    }

    #[test_case]
    fn test_set_llm_up() {
        let mut bar = make_bar();
        bar.set_llm_up();
        assert_eq!(bar.llm, LlmState::Up);
        assert!(bar.dirty);
    }

    #[test_case]
    fn test_set_llm_error() {
        let mut bar = make_bar();
        bar.set_llm_error();
        assert_eq!(bar.llm, LlmState::Error);
    }

    #[test_case]
    fn test_inc_tick() {
        let mut bar = make_bar();
        bar.inc_tick();
        assert_eq!(bar.tick_count, 1);
        bar.inc_tick();
        assert_eq!(bar.tick_count, 2);
    }

    #[test_case]
    fn test_tick_wraps() {
        let mut bar = make_bar();
        bar.tick_count = u64::MAX;
        bar.inc_tick();
        assert_eq!(bar.tick_count, 0);
    }

    #[test_case]
    fn test_set_ui_mode() {
        let mut bar = make_bar();
        bar.set_ui_mode("terminal");
        assert_eq!(bar.ui_mode, "ter"); // truncated to 3
    }

    #[test_case]
    fn test_set_ui_mode_short() {
        let mut bar = make_bar();
        bar.set_ui_mode("os");
        assert_eq!(bar.ui_mode, "os");
    }

    #[test_case]
    fn test_ui_mode_padded_exact() {
        let mut bar = make_bar();
        bar.ui_mode = String::from("POS");
        let padded = bar.ui_mode_padded();
        assert_eq!(padded.len(), 3);
    }

    #[test_case]
    fn test_ui_mode_padded_short() {
        let mut bar = make_bar();
        bar.ui_mode = String::from("OS");
        let padded = bar.ui_mode_padded();
        assert_eq!(padded.len(), 3);
        assert!(padded.starts_with("OS"));
    }

    #[test_case]
    fn test_no_repaint_if_not_dirty() {
        let mut bar = make_bar();
        bar.dirty = false;
        // Setting same state should not dirty (Up→Up)
        bar.network = NetworkState::Up;
        bar.set_network_up();
        assert!(!bar.dirty);
    }

    #[test_case]
    fn test_network_label_up() {
        assert_eq!(NetworkState::Up.label(), "N:U");
    }

    #[test_case]
    fn test_network_label_down() {
        assert_eq!(NetworkState::Down.label(), "N:D");
    }

    #[test_case]
    fn test_network_label_unknown() {
        assert_eq!(NetworkState::Unknown.label(), "N:?");
    }

    #[test_case]
    fn test_llm_label_up() {
        assert_eq!(LlmState::Up.label(), "AI");
    }

    #[test_case]
    fn test_llm_label_error() {
        assert_eq!(LlmState::Error.label(), "A!");
    }

    #[test_case]
    fn test_llm_label_idle() {
        assert_eq!(LlmState::Idle.label(), "A?");
    }

    #[test_case]
    fn test_describe() {
        let bar = make_bar();
        let desc = bar.describe();
        assert!(desc.contains("StatusBar"));
        assert!(desc.contains("tick=0"));
    }

    #[test_case]
    fn test_bar_layout_constants() {
        assert_eq!(BAR_COLS, 10);
        assert_eq!(BAR_PX_W, 80);
        assert_eq!(BAR_PX_H, 8);
        assert_eq!(BAR_PX_H, GLYPH_H);
    }

    #[test_case]
    fn test_render_does_not_panic() {
        let mut bar = make_bar();
        let mut comp = crate::compositor::Compositor::with_os_layers();
        bar.render(&mut comp); // should not panic
        assert!(!bar.dirty);
    }

    #[test_case]
    fn test_render_clears_dirty() {
        let mut bar = make_bar();
        let mut comp = crate::compositor::Compositor::with_os_layers();
        assert!(bar.dirty);
        bar.render(&mut comp);
        assert!(!bar.dirty);
    }

    #[test_case]
    fn test_render_skips_if_not_dirty() {
        let mut bar = make_bar();
        let mut comp = crate::compositor::Compositor::with_os_layers();
        bar.render(&mut comp); // first render, clears dirty
        // Paint something into the layer manually
        if let Some(layer) = comp.layer_mut(crate::compositor::LAYER_STATUS_BAR) {
            layer.set_pixel(0, 0, Color::Magenta);
        }
        bar.render(&mut comp); // not dirty, should NOT overwrite
        // Magenta pixel should survive (render skipped)
        if let Some(layer) = comp.layer(crate::compositor::LAYER_STATUS_BAR) {
            assert_eq!(layer.get_pixel(0, 0) as u8, Color::Magenta as u8);
        }
    }

    #[test_case]
    fn test_mark_dirty_forces_repaint() {
        let mut bar = make_bar();
        let mut comp = crate::compositor::Compositor::with_os_layers();
        bar.render(&mut comp);
        assert!(!bar.dirty);
        bar.mark_dirty();
        assert!(bar.dirty);
    }
}
