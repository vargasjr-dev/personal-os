/// Compositor — layer-based window system for the framebuffer.
///
/// The compositor manages named pixel layers that are blended in Z-order
/// onto the output framebuffer. Each layer is an independently-paintable
/// pixel buffer with its own position, size, and visibility.
///
/// Layer compositing rules:
///   - Layers are drawn back-to-front (lowest Z first).
///   - Color::Black in a non-background layer is treated as transparent
///     (the pixel below shows through). Background layers are always opaque.
///   - The final composited image is flushed to the VGA buffer.
///
/// Pre-defined OS layers (by Z order):
///   LAYER_BACKGROUND  (z=0)  — full-screen solid backdrop
///   LAYER_CHAT        (z=1)  — conversation pane (main content)
///   LAYER_STATUS_BAR  (z=2)  — status strip at bottom of screen
///   LAYER_OVERLAY     (z=3)  — modal dialogs, notifications, cursor
///
/// Usage:
///   let mut comp = Compositor::new();
///   comp.init_os_layers();                     // create the 4 OS layers
///
///   // Paint into the chat layer
///   if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
///       layer.fill(Color::DarkGray);
///       layer.draw_rect(1, 1, layer.width()-2, layer.height()-2, Color::White);
///   }
///
///   comp.composite();   // merge all layers → output framebuffer
///   comp.flush();        // output framebuffer → VGA
///
/// Foundation for Phase 8 items 4–5:
///   item 4 — scrollable conversation UI lives in LAYER_CHAT
///   item 5 — system status bar lives in LAYER_STATUS_BAR
///
/// Phase 8, Item 3 — Basic compositor.

use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::format;

use crate::vga_buffer::Color;
use crate::framebuffer::{Framebuffer, FB_WIDTH, FB_HEIGHT};

// ─── Pre-defined OS layer IDs ─────────────────────────────────────────────────

pub const LAYER_BACKGROUND: usize = 0;
pub const LAYER_CHAT:        usize = 1;
pub const LAYER_STATUS_BAR:  usize = 2;
pub const LAYER_OVERLAY:     usize = 3;

/// Maximum layers the compositor tracks.
const MAX_LAYERS: usize = 8;

// ─── Layer ────────────────────────────────────────────────────────────────────

/// A compositable pixel surface.
pub struct Layer {
    /// Unique layer ID (assigned at creation).
    pub id: usize,
    /// Human-readable name.
    pub name: String,
    /// Pixel x of top-left corner on the output canvas.
    pub x: usize,
    /// Pixel y of top-left corner on the output canvas.
    pub y: usize,
    /// Width in pixels.
    pub width: usize,
    /// Height in pixels.
    pub height: usize,
    /// Stacking order — lower = further back.
    pub z: u8,
    /// Whether this layer is rendered during compositing.
    pub visible: bool,
    /// Whether this is the background (always opaque — no transparency).
    pub is_background: bool,
    /// Pixel buffer: `pixels[row * width + col]`.
    pixels: Vec<Color>,
    /// Whether the pixel buffer has been modified since last composite.
    pub dirty: bool,
}

impl Layer {
    fn new(id: usize, name: &str, x: usize, y: usize,
           width: usize, height: usize, z: u8, is_background: bool) -> Self {
        let size = width * height;
        let mut pixels = Vec::with_capacity(size);
        for _ in 0..size {
            pixels.push(Color::Black);
        }
        Layer {
            id, name: String::from(name),
            x, y, width, height, z,
            visible: true,
            is_background,
            pixels,
            dirty: true,
        }
    }

    // ─── Pixel access ────────────────────────────────────────────────

    /// Set a pixel in this layer's local coordinate space.
    pub fn set_pixel(&mut self, x: usize, y: usize, color: Color) {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x] = color;
            self.dirty = true;
        }
    }

    /// Read a pixel in local coordinates.
    pub fn get_pixel(&self, x: usize, y: usize) -> Color {
        if x < self.width && y < self.height {
            self.pixels[y * self.width + x]
        } else {
            Color::Black
        }
    }

    // ─── Drawing primitives ──────────────────────────────────────────

    /// Fill the entire layer with one color.
    pub fn fill(&mut self, color: Color) {
        for px in self.pixels.iter_mut() {
            *px = color;
        }
        self.dirty = true;
    }

    /// Fill a rectangle in local coordinates.
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        for dy in 0..h {
            for dx in 0..w {
                self.set_pixel(x + dx, y + dy, color);
            }
        }
    }

    /// Draw a rectangle outline in local coordinates.
    pub fn draw_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: Color) {
        if w == 0 || h == 0 { return; }
        for dx in 0..w { self.set_pixel(x + dx, y, color); }
        for dx in 0..w { self.set_pixel(x + dx, y + h - 1, color); }
        for dy in 0..h { self.set_pixel(x, y + dy, color); }
        for dy in 0..h { self.set_pixel(x + w - 1, y + dy, color); }
    }

    /// Draw a horizontal line.
    pub fn draw_hline(&mut self, x0: usize, x1: usize, y: usize, color: Color) {
        let lo = x0.min(x1);
        let hi = x0.max(x1);
        for x in lo..=hi { self.set_pixel(x, y, color); }
    }

    /// Draw a vertical line.
    pub fn draw_vline(&mut self, x: usize, y0: usize, y1: usize, color: Color) {
        let lo = y0.min(y1);
        let hi = y0.max(y1);
        for y in lo..=hi { self.set_pixel(x, y, color); }
    }

    /// Blit another layer's pixels into this layer at offset (dx, dy).
    /// Transparent (Black) pixels in the source are skipped.
    pub fn blit(&mut self, src: &Layer, dx: usize, dy: usize) {
        for sy in 0..src.height {
            for sx in 0..src.width {
                let color = src.get_pixel(sx, sy);
                if color as u8 != Color::Black as u8 || src.is_background {
                    self.set_pixel(dx + sx, dy + sy, color);
                }
            }
        }
    }

    // ─── Accessors ───────────────────────────────────────────────────

    pub fn width(&self)  -> usize { self.width  }
    pub fn height(&self) -> usize { self.height }

    /// Pixel count (width × height).
    pub fn size(&self) -> usize { self.width * self.height }

    /// Summary string for debugging.
    pub fn describe(&self) -> String {
        format!(
            "Layer[{}] '{}' {}×{} @({},{}) z={} vis={} dirty={}",
            self.id, self.name,
            self.width, self.height,
            self.x, self.y,
            self.z, self.visible, self.dirty
        )
    }
}

// ─── Compositor ───────────────────────────────────────────────────────────────

/// Layer-based compositor — manages surfaces and blends them onto the output.
pub struct Compositor {
    /// All registered layers.
    layers: Vec<Layer>,
    /// Output framebuffer (composited result).
    output: Framebuffer,
    /// Whether any layer has changed since last composite.
    needs_composite: bool,
}

impl Compositor {
    /// Create an empty compositor with a blank output buffer.
    pub fn new() -> Self {
        Compositor {
            layers: Vec::new(),
            output: Framebuffer::new(),
            needs_composite: true,
        }
    }

    /// Create the 4 standard OS layers and return the compositor ready to use.
    ///
    /// Layout:
    ///   LAYER_BACKGROUND — full canvas (80×50)
    ///   LAYER_CHAT       — main content area (80×42), top of screen
    ///   LAYER_STATUS_BAR — 2-pixel status strip (80×8), bottom 8 rows
    ///   LAYER_OVERLAY    — full canvas (80×50), topmost; starts invisible
    pub fn with_os_layers() -> Self {
        let mut comp = Compositor::new();

        // Background: full canvas, deep blue
        let mut bg = Layer::new(LAYER_BACKGROUND, "background",
                                0, 0, FB_WIDTH, FB_HEIGHT, 0, true);
        bg.fill(Color::Blue);
        comp.layers.push(bg);

        // Chat pane: top 42 rows (42÷8 = ~5 text rows @ 8px glyphs)
        let mut chat = Layer::new(LAYER_CHAT, "chat",
                                  0, 0, FB_WIDTH, 42, 1, false);
        chat.fill(Color::Black);
        comp.layers.push(chat);

        // Status bar: bottom 8 rows
        let mut status = Layer::new(LAYER_STATUS_BAR, "status_bar",
                                    0, 42, FB_WIDTH, 8, 2, false);
        status.fill(Color::DarkGray);
        comp.layers.push(status);

        // Overlay: full canvas, initially hidden
        let mut overlay = Layer::new(LAYER_OVERLAY, "overlay",
                                     0, 0, FB_WIDTH, FB_HEIGHT, 3, false);
        overlay.visible = false;
        comp.layers.push(overlay);

        comp
    }

    // ─── Layer management ────────────────────────────────────────────

    /// Register a custom layer. Returns its index in the layers vec.
    pub fn add_layer(&mut self, name: &str, x: usize, y: usize,
                     width: usize, height: usize, z: u8) -> usize {
        if self.layers.len() >= MAX_LAYERS {
            return self.layers.len() - 1; // refuse silently at cap
        }
        let id = self.layers.len();
        self.layers.push(Layer::new(id, name, x, y, width, height, z, false));
        self.needs_composite = true;
        id
    }

    /// Get an immutable reference to a layer by ID.
    pub fn layer(&self, id: usize) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Get a mutable reference to a layer by ID.
    pub fn layer_mut(&mut self, id: usize) -> Option<&mut Layer> {
        self.needs_composite = true;
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Show or hide a layer.
    pub fn set_visible(&mut self, id: usize, visible: bool) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.visible = visible;
            layer.dirty = true;
        }
        self.needs_composite = true;
    }

    /// Move a layer to a new canvas position.
    pub fn move_layer(&mut self, id: usize, x: usize, y: usize) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.x = x;
            layer.y = y;
            layer.dirty = true;
        }
        self.needs_composite = true;
    }

    /// Change a layer's Z order and re-sort.
    pub fn set_z(&mut self, id: usize, z: u8) {
        if let Some(layer) = self.layers.iter_mut().find(|l| l.id == id) {
            layer.z = z;
        }
        self.needs_composite = true;
    }

    // ─── Compositing ─────────────────────────────────────────────────

    /// Composite all visible layers (back-to-front) onto the output framebuffer.
    /// Only recomposites if a layer is dirty or `needs_composite` is set.
    pub fn composite(&mut self) {
        if !self.needs_composite {
            return;
        }

        // Collect (z, index) pairs and sort by z ascending
        let mut order: Vec<(u8, usize)> = self.layers.iter()
            .enumerate()
            .filter(|(_, l)| l.visible)
            .map(|(i, l)| (l.z, i))
            .collect();
        order.sort_by_key(|&(z, _)| z);

        // Blit each layer onto the output in z-order
        for (_, idx) in &order {
            let layer = &self.layers[*idx];
            let lx = layer.x;
            let ly = layer.y;
            let is_bg = layer.is_background;

            for py in 0..layer.height {
                for px in 0..layer.width {
                    let color = layer.get_pixel(px, py);
                    // Transparent: non-background Black pixels are skipped
                    if color as u8 == Color::Black as u8 && !is_bg {
                        continue;
                    }
                    let ox = lx + px;
                    let oy = ly + py;
                    if ox < FB_WIDTH && oy < FB_HEIGHT {
                        self.output.set_pixel(ox, oy, color);
                    }
                }
            }
        }

        // Mark all layers clean
        for layer in self.layers.iter_mut() {
            layer.dirty = false;
        }
        self.needs_composite = false;
    }

    /// Flush the composited output to the VGA display.
    /// Calls `composite()` first if needed.
    pub fn flush(&mut self) {
        self.composite();
        self.output.flush();
    }

    /// Force a full recomposite + flush regardless of dirty state.
    pub fn redraw(&mut self) {
        self.needs_composite = true;
        for layer in self.layers.iter_mut() {
            layer.dirty = true;
        }
        self.output.flush_all();
        self.flush();
    }

    // ─── Diagnostics ─────────────────────────────────────────────────

    /// Number of registered layers.
    pub fn layer_count(&self) -> usize { self.layers.len() }

    /// Number of currently visible layers.
    pub fn visible_count(&self) -> usize {
        self.layers.iter().filter(|l| l.visible).count()
    }

    /// Number of dirty layers.
    pub fn dirty_count(&self) -> usize {
        self.layers.iter().filter(|l| l.dirty).count()
    }

    /// Describe all layers for serial debugging.
    pub fn describe(&self) -> String {
        let mut s = format!(
            "Compositor: {} layers, {} visible, {} dirty\n",
            self.layer_count(), self.visible_count(), self.dirty_count()
        );
        // Sort by z for readability
        let mut layers: Vec<&Layer> = self.layers.iter().collect();
        layers.sort_by_key(|l| l.z);
        for l in layers {
            s.push_str("  ");
            s.push_str(&l.describe());
            s.push('\n');
        }
        s
    }
}

// ─── Global compositor ───────────────────────────────────────────────────────

use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// Global OS compositor — initialized with the 4 standard OS layers.
    pub static ref COMPOSITOR: Mutex<Compositor> = Mutex::new(
        Compositor::with_os_layers()
    );
}

/// Convenience: paint the background layer with `color` and flush.
pub fn set_background(color: Color) {
    let mut comp = COMPOSITOR.lock();
    if let Some(bg) = comp.layer_mut(LAYER_BACKGROUND) {
        bg.fill(color);
    }
    comp.flush();
}

/// Convenience: show or hide the overlay layer.
pub fn set_overlay_visible(visible: bool) {
    let mut comp = COMPOSITOR.lock();
    comp.set_visible(LAYER_OVERLAY, visible);
    comp.flush();
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_compositor() -> Compositor {
        Compositor::with_os_layers()
    }

    #[test_case]
    fn test_with_os_layers_creates_four() {
        let comp = make_compositor();
        assert_eq!(comp.layer_count(), 4);
    }

    #[test_case]
    fn test_background_layer_exists() {
        let comp = make_compositor();
        let bg = comp.layer(LAYER_BACKGROUND).unwrap();
        assert_eq!(bg.name, "background");
        assert_eq!(bg.width, FB_WIDTH);
        assert_eq!(bg.height, FB_HEIGHT);
        assert!(bg.is_background);
    }

    #[test_case]
    fn test_status_bar_position() {
        let comp = make_compositor();
        let sb = comp.layer(LAYER_STATUS_BAR).unwrap();
        assert_eq!(sb.y, 42);
        assert_eq!(sb.height, 8);
        assert_eq!(sb.width, FB_WIDTH);
    }

    #[test_case]
    fn test_overlay_starts_invisible() {
        let comp = make_compositor();
        let ov = comp.layer(LAYER_OVERLAY).unwrap();
        assert!(!ov.visible);
    }

    #[test_case]
    fn test_set_visible() {
        let mut comp = make_compositor();
        comp.set_visible(LAYER_OVERLAY, true);
        assert!(comp.layer(LAYER_OVERLAY).unwrap().visible);
        comp.set_visible(LAYER_OVERLAY, false);
        assert!(!comp.layer(LAYER_OVERLAY).unwrap().visible);
    }

    #[test_case]
    fn test_visible_count() {
        let comp = make_compositor();
        // Background (visible) + Chat (visible) + Status (visible) + Overlay (hidden) = 3
        assert_eq!(comp.visible_count(), 3);
    }

    #[test_case]
    fn test_layer_set_get_pixel() {
        let mut comp = make_compositor();
        if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
            layer.set_pixel(5, 5, Color::Red);
            assert_eq!(layer.get_pixel(5, 5) as u8, Color::Red as u8);
        }
    }

    #[test_case]
    fn test_layer_fill() {
        let mut comp = make_compositor();
        if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
            layer.fill(Color::Green);
            assert_eq!(layer.get_pixel(0, 0) as u8, Color::Green as u8);
            assert_eq!(layer.get_pixel(10, 10) as u8, Color::Green as u8);
        }
    }

    #[test_case]
    fn test_layer_fill_rect() {
        let mut layer = Layer::new(0, "test", 0, 0, 20, 20, 0, false);
        layer.fill_rect(5, 5, 5, 5, Color::Cyan);
        assert_eq!(layer.get_pixel(5, 5) as u8, Color::Cyan as u8);
        assert_eq!(layer.get_pixel(9, 9) as u8, Color::Cyan as u8);
        assert_eq!(layer.get_pixel(4, 5) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_layer_draw_rect_outline() {
        let mut layer = Layer::new(0, "test", 0, 0, 20, 20, 0, false);
        layer.draw_rect(2, 2, 6, 6, Color::White);
        assert_eq!(layer.get_pixel(2, 2) as u8, Color::White as u8); // top-left
        assert_eq!(layer.get_pixel(7, 7) as u8, Color::White as u8); // bottom-right
        assert_eq!(layer.get_pixel(4, 4) as u8, Color::Black as u8); // interior
    }

    #[test_case]
    fn test_add_custom_layer() {
        let mut comp = make_compositor();
        let id = comp.add_layer("hud", 0, 0, 20, 10, 5);
        assert_eq!(comp.layer_count(), 5);
        assert!(comp.layer(id).is_some());
    }

    #[test_case]
    fn test_move_layer() {
        let mut comp = make_compositor();
        comp.move_layer(LAYER_CHAT, 10, 5);
        let chat = comp.layer(LAYER_CHAT).unwrap();
        assert_eq!(chat.x, 10);
        assert_eq!(chat.y, 5);
    }

    #[test_case]
    fn test_composite_does_not_panic() {
        let mut comp = make_compositor();
        comp.composite(); // should not panic
    }

    #[test_case]
    fn test_background_opaque_black_shows() {
        let mut comp = Compositor::new();
        let mut bg = Layer::new(0, "bg", 0, 0, 4, 4, 0, true);
        bg.fill(Color::Black); // background is always opaque
        comp.layers.push(bg);
        comp.needs_composite = true;
        comp.composite();
        // Background black pixel should be written to output
        assert_eq!(comp.output.get_pixel(0, 0) as u8, Color::Black as u8);
    }

    #[test_case]
    fn test_layer_z_ordering() {
        let mut comp = Compositor::new();
        // Layer z=1: blue full canvas
        let mut bottom = Layer::new(0, "bottom", 0, 0, 4, 4, 1, true);
        bottom.fill(Color::Blue);
        comp.layers.push(bottom);
        // Layer z=2: red 2×2 on top at (0,0)
        let mut top = Layer::new(1, "top", 0, 0, 2, 2, 2, false);
        top.fill(Color::Red);
        comp.layers.push(top);
        comp.needs_composite = true;
        comp.composite();
        // (0,0) should be Red (top layer)
        assert_eq!(comp.output.get_pixel(0, 0) as u8, Color::Red as u8);
        // (3,3) should be Blue (bottom layer, top layer doesn't cover it)
        assert_eq!(comp.output.get_pixel(3, 3) as u8, Color::Blue as u8);
    }

    #[test_case]
    fn test_layer_transparency() {
        let mut comp = Compositor::new();
        // Background: all white
        let mut bg = Layer::new(0, "bg", 0, 0, 4, 4, 0, true);
        bg.fill(Color::White);
        comp.layers.push(bg);
        // Top layer: only one non-transparent pixel at (1,1)
        let mut top = Layer::new(1, "top", 0, 0, 4, 4, 1, false);
        top.set_pixel(1, 1, Color::Red);
        // All other pixels remain Black = transparent
        comp.layers.push(top);
        comp.needs_composite = true;
        comp.composite();
        // (1,1) = Red (top layer)
        assert_eq!(comp.output.get_pixel(1, 1) as u8, Color::Red as u8);
        // (0,0) = White (background shows through transparent top)
        assert_eq!(comp.output.get_pixel(0, 0) as u8, Color::White as u8);
    }

    #[test_case]
    fn test_layer_describe() {
        let layer = Layer::new(0, "test", 5, 10, 20, 15, 2, false);
        let _desc = layer.describe();
        assert_eq!(layer.width, 20);
        assert_eq!(layer.height, 15);
    }

    #[test_case]
    fn test_compositor_describe() {
        let comp = make_compositor();
        let _desc = comp.describe();
        assert_eq!(comp.layers.len(), 4);
    }

    #[test_case]
    fn test_layer_size() {
        let layer = Layer::new(0, "t", 0, 0, 10, 5, 0, false);
        assert_eq!(layer.size(), 50);
    }

    #[test_case]
    fn test_dirty_count_after_mutation() {
        let mut comp = make_compositor();
        // After creation, all layers are dirty
        assert!(comp.dirty_count() > 0);
        comp.composite();
        assert_eq!(comp.dirty_count(), 0);
        // Mutate one layer
        if let Some(layer) = comp.layer_mut(LAYER_CHAT) {
            layer.set_pixel(0, 0, Color::Red);
        }
        assert_eq!(comp.dirty_count(), 1);
    }

    #[test_case]
    fn test_layer_blit() {
        let mut dst = Layer::new(0, "dst", 0, 0, 10, 10, 0, false);
        let mut src = Layer::new(1, "src", 0, 0, 3, 3, 1, false);
        src.fill(Color::Yellow);
        dst.blit(&src, 2, 2);
        assert_eq!(dst.get_pixel(2, 2) as u8, Color::Yellow as u8);
        assert_eq!(dst.get_pixel(4, 4) as u8, Color::Yellow as u8);
        assert_eq!(dst.get_pixel(0, 0) as u8, Color::Black as u8);
    }
}
