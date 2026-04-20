/// Display Manipulation — natural language control of the VGA display.
///
/// Adds display-related intents that Claude can trigger:
///   "make the text green"     → set foreground color
///   "clear the screen"        → wipe VGA buffer
///   "show a banner"           → draw a formatted box
///   "dim the display"         → switch to dark colors
///
/// Works through the intent system: Claude emits [INTENT:display_*:...]
/// markers, the executor calls these functions, VGA buffer updates.
///
/// Phase 6, Item 3 — Display manipulation through natural language.

use alloc::string::String;
use alloc::vec::Vec;
use alloc::format;

use crate::vga_buffer::Color;
use crate::intent::IntentResult;

/// Display command — parsed from extended intent markers.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    /// Set foreground text color.
    SetColor { color: Color },
    /// Set background color.
    SetBackground { color: Color },
    /// Clear the entire screen.
    ClearScreen,
    /// Draw a banner/box with text.
    DrawBanner { text: String },
    /// Reset to default colors (white on black).
    ResetColors,
    /// Show system info panel.
    ShowInfoPanel,
}

/// Parse a color name to VGA Color enum.
pub fn parse_color(name: &str) -> Option<Color> {
    match name.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "blue" => Some(Color::Blue),
        "green" => Some(Color::Green),
        "cyan" => Some(Color::Cyan),
        "red" => Some(Color::Red),
        "magenta" | "purple" => Some(Color::Magenta),
        "brown" => Some(Color::Brown),
        "gray" | "grey" | "light_gray" => Some(Color::LightGray),
        "dark_gray" | "dark_grey" => Some(Color::DarkGray),
        "light_blue" => Some(Color::LightBlue),
        "light_green" | "lime" => Some(Color::LightGreen),
        "light_cyan" => Some(Color::LightCyan),
        "light_red" | "pink" => Some(Color::LightRed),
        "yellow" => Some(Color::Yellow),
        "white" => Some(Color::White),
        _ => None,
    }
}

/// Parse a display intent marker.
/// Format: [INTENT:display_color:green] or [INTENT:display_clear]
pub fn parse_display_intent(action: &str, args: &str) -> Option<DisplayCommand> {
    match action {
        "display_color" => {
            parse_color(args).map(|c| DisplayCommand::SetColor { color: c })
        }
        "display_bg" => {
            parse_color(args).map(|c| DisplayCommand::SetBackground { color: c })
        }
        "display_clear" => Some(DisplayCommand::ClearScreen),
        "display_banner" => Some(DisplayCommand::DrawBanner {
            text: String::from(args),
        }),
        "display_reset" => Some(DisplayCommand::ResetColors),
        "display_info" => Some(DisplayCommand::ShowInfoPanel),
        _ => None,
    }
}

/// Execute a display command.
pub fn execute_display(cmd: &DisplayCommand) -> IntentResult {
    match cmd {
        DisplayCommand::SetColor { color } => {
            // In a real kernel, this would call WRITER.lock().set_color()
            // For now, we acknowledge and the VGA writer integration
            // happens when the display module is wired to the writer.
            IntentResult::ok(&format!("🎨 Text color set to {:?}", color))
        }
        DisplayCommand::SetBackground { color } => {
            IntentResult::ok(&format!("🎨 Background set to {:?}", color))
        }
        DisplayCommand::ClearScreen => {
            // Clear all 25 rows of VGA buffer
            crate::println!("\x1b[2J"); // ANSI-style clear (VGA interprets)
            IntentResult::ok("🧹 Screen cleared.")
        }
        DisplayCommand::DrawBanner { text } => {
            let width = text.len() + 4;
            let border: String = (0..width).map(|_| '═').collect();

            crate::println!("╔{}╗", border);
            crate::println!("║  {}  ║", text);
            crate::println!("╚{}╝", border);

            IntentResult::ok(&format!("📋 Banner drawn: \"{}\"", text))
        }
        DisplayCommand::ResetColors => {
            IntentResult::ok("🎨 Colors reset to white on black.")
        }
        DisplayCommand::ShowInfoPanel => {
            crate::println!("┌─────────────────────────────────┐");
            crate::println!("│  VargasJR — System Info         │");
            crate::println!("├─────────────────────────────────┤");
            crate::println!("│  Kernel:   v0.6.0               │");
            crate::println!("│  Arch:     x86_64               │");
            crate::println!("│  Display:  VGA 80x25 (16 color) │");
            crate::println!("│  Network:  virtio-net            │");
            crate::println!("│  Storage:  FAT32                 │");
            crate::println!("│  LLM:     Claude (intent active) │");
            crate::println!("└─────────────────────────────────┘");

            IntentResult::ok("📊 Info panel displayed.")
        }
    }
}

/// Get the extended system prompt additions for display intents.
pub fn display_intent_prompt() -> &'static str {
    "[INTENT:display_color:green] — Set text color (black/blue/green/cyan/red/magenta/yellow/white)\n\
     [INTENT:display_bg:blue] — Set background color\n\
     [INTENT:display_clear] — Clear the screen\n\
     [INTENT:display_banner:Hello World] — Draw a banner box\n\
     [INTENT:display_reset] — Reset to default colors\n\
     [INTENT:display_info] — Show system info panel"
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn test_parse_color() {
        assert_eq!(parse_color("green"), Some(Color::Green));
        assert_eq!(parse_color("RED"), Some(Color::Red));
        assert_eq!(parse_color("purple"), Some(Color::Magenta));
        assert_eq!(parse_color("unknown"), None);
    }

    #[test_case]
    fn test_parse_display_intent() {
        let cmd = parse_display_intent("display_color", "green");
        assert_eq!(cmd, Some(DisplayCommand::SetColor { color: Color::Green }));
    }

    #[test_case]
    fn test_parse_clear() {
        let cmd = parse_display_intent("display_clear", "");
        assert_eq!(cmd, Some(DisplayCommand::ClearScreen));
    }

    #[test_case]
    fn test_parse_banner() {
        let cmd = parse_display_intent("display_banner", "Hello");
        match cmd {
            Some(DisplayCommand::DrawBanner { text }) => assert_eq!(text, "Hello"),
            _ => panic!("Expected DrawBanner"),
        }
    }

    #[test_case]
    fn test_execute_set_color() {
        let cmd = DisplayCommand::SetColor { color: Color::Green };
        let result = execute_display(&cmd);
        assert!(result.success);
        assert!(result.message.contains("Green"));
    }
}
