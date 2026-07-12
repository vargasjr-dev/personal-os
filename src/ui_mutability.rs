/// UI Mutability — the assistant reshapes the interface.
///
/// The kernel's UI is not a fixed shell. The assistant can reconfigure
/// the display layout, mode, and density in real time — responding to
/// context, user preference, or its own judgment about what's useful.
///
/// UI Modes:
///   Chat       — conversational, [you] / [vargas-jr] turns (default)
///   Terminal   — classic command-line style, minimal decoration
///   Dashboard  — system panels, status overview, metrics layout
///   Focus      — distraction-free, just the response area
///   Code       — wide format optimized for code output, ruler visible
///
/// Layout Configs:
///   Full       — 80 columns, 25 rows (VGA default)
///   Split      — left nav panel + right content area
///   Compact    — 60 columns, preserve margins
///
/// Triggered via the intent system:
///   [INTENT:ui_mode:dashboard]   → switch to dashboard mode
///   [INTENT:ui_mode:terminal]    → switch to terminal mode
///   [INTENT:ui_mode:focus]       → switch to focus mode
///   [INTENT:ui_mode:code]        → switch to code mode
///   [INTENT:ui_mode:chat]        → return to chat mode
///   [INTENT:ui_layout:split]     → split layout
///   [INTENT:ui_layout:full]      → full-width layout
///   [INTENT:ui_density:compact]  → compact line spacing
///   [INTENT:ui_density:normal]   → normal line spacing
///
/// Phase 7, Item 5 — UI mutability — the assistant reshapes the interface.

use alloc::string::String;
use alloc::string::ToString;
use alloc::format;
use alloc::vec::Vec;

use crate::vga_buffer::Color;
use crate::intent::IntentResult;
use crate::context::SystemContext;

// ─── UI Mode ─────────────────────────────────────────────────────────────────

/// The active UI mode — determines how input/output is rendered.
#[derive(Debug, Clone, PartialEq)]
pub enum UiMode {
    /// Conversational chat — [you] / [vargas-jr] turns. Default.
    Chat,
    /// Classic command-line — `> ` prompt, raw output.
    Terminal,
    /// Dashboard — panels showing system status and metrics.
    Dashboard,
    /// Focus mode — minimal chrome, just the response.
    Focus,
    /// Code mode — optimized for code output, ruler at 80 chars.
    Code,
}

impl UiMode {
    pub fn name(&self) -> &'static str {
        match self {
            UiMode::Chat     => "chat",
            UiMode::Terminal => "terminal",
            UiMode::Dashboard => "dashboard",
            UiMode::Focus    => "focus",
            UiMode::Code     => "code",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            UiMode::Chat     => "Conversational — [you] / [vargas-jr] turns",
            UiMode::Terminal => "Classic command-line — minimal decoration",
            UiMode::Dashboard => "Panels with system status and metrics",
            UiMode::Focus    => "Distraction-free — just the response",
            UiMode::Code     => "Wide format — optimized for code output",
        }
    }

    pub fn prompt_prefix(&self) -> &'static str {
        match self {
            UiMode::Chat     => "[you] ",
            UiMode::Terminal => "> ",
            UiMode::Dashboard => "cmd> ",
            UiMode::Focus    => "  ",
            UiMode::Code     => "│ ",
        }
    }

    pub fn response_prefix(&self) -> &'static str {
        match self {
            UiMode::Chat     => "[vargas-jr] ",
            UiMode::Terminal => "",
            UiMode::Dashboard => "[sys] ",
            UiMode::Focus    => "",
            UiMode::Code     => "│ ",
        }
    }
}

// ─── Layout ──────────────────────────────────────────────────────────────────

/// The active layout — determines how screen space is divided.
#[derive(Debug, Clone, PartialEq)]
pub enum UiLayout {
    /// Full VGA width — 80 columns, all rows for content.
    Full,
    /// Split — left 20-char status sidebar + 60-char content area.
    Split,
    /// Compact — 60 columns centered, preserve side margins.
    Compact,
}

impl UiLayout {
    pub fn name(&self) -> &'static str {
        match self {
            UiLayout::Full    => "full",
            UiLayout::Split   => "split",
            UiLayout::Compact => "compact",
        }
    }

    /// Usable content columns in this layout.
    pub fn content_width(&self) -> usize {
        match self {
            UiLayout::Full    => 78,
            UiLayout::Split   => 58,
            UiLayout::Compact => 58,
        }
    }

    /// Left margin (columns to skip before content).
    pub fn left_margin(&self) -> usize {
        match self {
            UiLayout::Full    => 0,
            UiLayout::Split   => 21,
            UiLayout::Compact => 10,
        }
    }
}

// ─── Line Density ─────────────────────────────────────────────────────────────

/// How tightly lines are packed — affects blank line insertion.
#[derive(Debug, Clone, PartialEq)]
pub enum UiDensity {
    /// Normal — blank lines between turns.
    Normal,
    /// Compact — no blank lines, maximum content on screen.
    Compact,
}

impl UiDensity {
    pub fn name(&self) -> &'static str {
        match self {
            UiDensity::Normal  => "normal",
            UiDensity::Compact => "compact",
        }
    }
}

// ─── UI State ────────────────────────────────────────────────────────────────

/// Complete mutable UI state — the shape of the interface right now.
#[derive(Debug, Clone)]
pub struct UiState {
    pub mode:    UiMode,
    pub layout:  UiLayout,
    pub density: UiDensity,
    /// Stack of previous modes for "go back" support.
    pub history: Vec<UiMode>,
}

impl UiState {
    /// Default boot state — chat, full, normal.
    pub fn default() -> Self {
        UiState {
            mode:    UiMode::Chat,
            layout:  UiLayout::Full,
            density: UiDensity::Normal,
            history: Vec::new(),
        }
    }

    /// Switch to a new mode, pushing the current to history.
    pub fn set_mode(&mut self, new_mode: UiMode) {
        let prev = self.mode.clone();
        self.history.push(prev);
        if self.history.len() > 8 {
            self.history.remove(0); // Cap history depth
        }
        self.mode = new_mode;
    }

    /// Pop back to the previous mode.
    pub fn go_back(&mut self) -> bool {
        if let Some(prev) = self.history.pop() {
            self.mode = prev;
            true
        } else {
            false
        }
    }

    /// Describe the current state for display.
    pub fn describe(&self) -> String {
        format!(
            "Mode: {} | Layout: {} | Density: {}",
            self.mode.name(),
            self.layout.name(),
            self.density.name()
        )
    }
}

// ─── Intent Parsing ──────────────────────────────────────────────────────────

/// Intent actions this module handles.
pub enum UiIntent {
    SetMode(UiMode),
    SetLayout(UiLayout),
    SetDensity(UiDensity),
    GoBack,
    ShowStatus,
}

/// Parse a ui_* intent marker into a UiIntent.
pub fn parse_ui_intent(action: &str, args: &str) -> Option<UiIntent> {
    match action {
        "ui_mode" => {
            let mode = match args {
                "chat"      => UiMode::Chat,
                "terminal"  => UiMode::Terminal,
                "dashboard" => UiMode::Dashboard,
                "focus"     => UiMode::Focus,
                "code"      => UiMode::Code,
                _           => return None,
            };
            Some(UiIntent::SetMode(mode))
        }
        "ui_layout" => {
            let layout = match args {
                "full"    => UiLayout::Full,
                "split"   => UiLayout::Split,
                "compact" => UiLayout::Compact,
                _         => return None,
            };
            Some(UiIntent::SetLayout(layout))
        }
        "ui_density" => {
            let density = match args {
                "normal"  => UiDensity::Normal,
                "compact" => UiDensity::Compact,
                _         => return None,
            };
            Some(UiIntent::SetDensity(density))
        }
        "ui_back"   => Some(UiIntent::GoBack),
        "ui_status" => Some(UiIntent::ShowStatus),
        _           => None,
    }
}

// ─── Intent Execution ────────────────────────────────────────────────────────

/// Apply a parsed UiIntent to the mutable state.
/// Returns an IntentResult for the assistant to relay.
pub fn execute_ui_intent(intent: UiIntent, state: &mut UiState) -> IntentResult {
    match intent {
        UiIntent::SetMode(new_mode) => {
            let prev_name = state.mode.name();
            let desc     = new_mode.description();
            let new_name = new_mode.name();

            state.set_mode(new_mode.clone());
            render_mode_transition(prev_name, new_name);

            IntentResult::ok(&format!(
                "🖥️  Switched to {} mode — {}",
                new_name, desc
            ))
        }

        UiIntent::SetLayout(layout) => {
            let name = layout.name();
            state.layout = layout;
            IntentResult::ok(&format!("📐 Layout set to {}", name))
        }

        UiIntent::SetDensity(density) => {
            let name = density.name();
            state.density = density;
            IntentResult::ok(&format!("📏 Line density set to {}", name))
        }

        UiIntent::GoBack => {
            if state.go_back() {
                IntentResult::ok(&format!(
                    "↩️  Returned to {} mode",
                    state.mode.name()
                ))
            } else {
                IntentResult::err("No previous mode to return to.")
            }
        }

        UiIntent::ShowStatus => {
            let description = state.describe();
            crate::println!("┌─────────────────────────────────────┐");
            crate::println!("│  UI State                           │");
            crate::println!("├─────────────────────────────────────┤");
            crate::println!("│  Mode:    {:26} │", state.mode.name());
            crate::println!("│  Layout:  {:26} │", state.layout.name());
            crate::println!("│  Density: {:26} │", state.density.name());
            crate::println!("│  Prompt:  {:26} │", state.mode.prompt_prefix().trim());
            crate::println!("└─────────────────────────────────────┘");
            IntentResult::ok(&description)
        }
    }
}

// ─── Rendering ───────────────────────────────────────────────────────────────

/// Render the visual cue for a mode transition.
fn render_mode_transition(from: &str, to: &str) {
    crate::println!();
    crate::println!("┄┄┄ switching: {} → {} ┄┄┄", from, to);
    crate::println!();
}

/// Render the dashboard mode — system panels on screen.
pub fn render_dashboard(ctx: &SystemContext) {
    crate::println!("╔══════════════════════════════════════════════════════════════════╗");
    crate::println!("║                   VargasJR — Dashboard                          ║");
    crate::println!("╠══════════════════════╦═══════════════════════════════════════════╣");
    crate::println!("║  System              ║  Activity                                ║");
    crate::println!("║  Arch:    x86_64     ║  Tasks:   {}                             ║",
        ctx.running_tasks);
    crate::println!("║  Kernel:  v0.7.0     ║  Files:   {}                             ║",
        ctx.open_files);
    crate::println!("║  Net:     up         ║  Memory:  ok                             ║");
    crate::println!("╠══════════════════════╩═══════════════════════════════════════════╣");
    crate::println!("║  Conversation                                                    ║");
    crate::println!("║  Turns: {}   Context: {}%                                       ║",
        ctx.conversation_turns, ctx.context_fill_percent);
    crate::println!("╚══════════════════════════════════════════════════════════════════╝");
}

/// Render the focus-mode header — minimal, just a thin separator.
pub fn render_focus_header() {
    crate::println!("─────────────────────────────────────────────────");
}

/// Render the code-mode ruler — 80-char guide line.
pub fn render_code_ruler() {
    crate::println!("         1111111111222222222233333333334444444444555555555566666666667777777780");
    crate::println!("1234567890123456789012345678901234567890123456789012345678901234567890123456789");
}

// ─── System Prompt Additions ──────────────────────────────────────────────────

/// Get the extended system prompt additions for UI mutability intents.
pub fn ui_mutability_prompt() -> &'static str {
    "[INTENT:ui_mode:chat]      — Switch to conversational chat mode (default)\n\
     [INTENT:ui_mode:terminal]  — Switch to terminal / command-line mode\n\
     [INTENT:ui_mode:dashboard] — Switch to dashboard with system panels\n\
     [INTENT:ui_mode:focus]     — Switch to focus mode (minimal chrome)\n\
     [INTENT:ui_mode:code]      — Switch to code-optimized mode with ruler\n\
     [INTENT:ui_layout:full]    — Use full 80-column layout\n\
     [INTENT:ui_layout:split]   — Use split layout (sidebar + content)\n\
     [INTENT:ui_layout:compact] — Use compact 60-column layout\n\
     [INTENT:ui_density:normal] — Normal line spacing between turns\n\
     [INTENT:ui_density:compact] — Compact spacing, maximum content\n\
     [INTENT:ui_back]           — Return to previous mode\n\
     [INTENT:ui_status]         — Show current UI state"
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_default_ui_state() {
        let state = UiState::default();
        assert_eq!(state.mode, UiMode::Chat);
        assert_eq!(state.layout, UiLayout::Full);
        assert_eq!(state.density, UiDensity::Normal);
        assert!(state.history.is_empty());
    }

    #[test_case]
    fn test_mode_switch_pushes_history() {
        let mut state = UiState::default();
        state.set_mode(UiMode::Terminal);
        assert_eq!(state.mode, UiMode::Terminal);
        assert_eq!(state.history.len(), 1);
        assert_eq!(state.history[0], UiMode::Chat);
    }

    #[test_case]
    fn test_go_back_pops_history() {
        let mut state = UiState::default();
        state.set_mode(UiMode::Terminal);
        state.set_mode(UiMode::Dashboard);
        assert_eq!(state.mode, UiMode::Dashboard);
        let went_back = state.go_back();
        assert!(went_back);
        assert_eq!(state.mode, UiMode::Terminal);
    }

    #[test_case]
    fn test_go_back_empty_history() {
        let mut state = UiState::default();
        let went_back = state.go_back();
        assert!(!went_back);
    }

    #[test_case]
    fn test_history_capped_at_8() {
        let mut state = UiState::default();
        let modes = [
            UiMode::Terminal, UiMode::Dashboard, UiMode::Focus,
            UiMode::Code, UiMode::Chat, UiMode::Terminal,
            UiMode::Dashboard, UiMode::Focus, UiMode::Code,
        ];
        for mode in modes {
            state.set_mode(mode);
        }
        assert!(state.history.len() <= 8);
    }

    #[test_case]
    fn test_parse_ui_mode_intent_chat() {
        let intent = parse_ui_intent("ui_mode", "chat");
        match intent {
            Some(UiIntent::SetMode(UiMode::Chat)) => {},
            _ => panic!("Expected SetMode(Chat)"),
        }
    }

    #[test_case]
    fn test_parse_ui_mode_intent_terminal() {
        let intent = parse_ui_intent("ui_mode", "terminal");
        match intent {
            Some(UiIntent::SetMode(UiMode::Terminal)) => {},
            _ => panic!("Expected SetMode(Terminal)"),
        }
    }

    #[test_case]
    fn test_parse_ui_mode_intent_dashboard() {
        let intent = parse_ui_intent("ui_mode", "dashboard");
        match intent {
            Some(UiIntent::SetMode(UiMode::Dashboard)) => {},
            _ => panic!("Expected SetMode(Dashboard)"),
        }
    }

    #[test_case]
    fn test_parse_ui_mode_intent_unknown() {
        let intent = parse_ui_intent("ui_mode", "holodeck");
        assert!(intent.is_none());
    }

    #[test_case]
    fn test_parse_layout_split() {
        let intent = parse_ui_intent("ui_layout", "split");
        match intent {
            Some(UiIntent::SetLayout(UiLayout::Split)) => {},
            _ => panic!("Expected SetLayout(Split)"),
        }
    }

    #[test_case]
    fn test_parse_density_compact() {
        let intent = parse_ui_intent("ui_density", "compact");
        match intent {
            Some(UiIntent::SetDensity(UiDensity::Compact)) => {},
            _ => panic!("Expected SetDensity(Compact)"),
        }
    }

    #[test_case]
    fn test_parse_back_intent() {
        let intent = parse_ui_intent("ui_back", "");
        match intent {
            Some(UiIntent::GoBack) => {},
            _ => panic!("Expected GoBack"),
        }
    }

    #[test_case]
    fn test_parse_status_intent() {
        let intent = parse_ui_intent("ui_status", "");
        match intent {
            Some(UiIntent::ShowStatus) => {},
            _ => panic!("Expected ShowStatus"),
        }
    }

    #[test_case]
    fn test_content_width_full() {
        assert_eq!(UiLayout::Full.content_width(), 78);
    }

    #[test_case]
    fn test_content_width_split() {
        assert!(UiLayout::Split.content_width() < UiLayout::Full.content_width());
    }

    #[test_case]
    fn test_describe_state() {
        let state = UiState::default();
        let _desc = state.describe();
        assert!(state.history.is_empty());
    }

    #[test_case]
    fn test_mode_names() {
        assert_eq!(UiMode::Chat.name(), "chat");
        assert_eq!(UiMode::Terminal.name(), "terminal");
        assert_eq!(UiMode::Dashboard.name(), "dashboard");
        assert_eq!(UiMode::Focus.name(), "focus");
        assert_eq!(UiMode::Code.name(), "code");
    }

    #[test_case]
    fn test_prompt_prefixes() {
        assert!(UiMode::Chat.prompt_prefix().contains("you"));
        assert!(UiMode::Terminal.prompt_prefix().contains(">"));
    }
}
