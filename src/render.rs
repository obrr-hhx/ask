use anyhow::{Context, Result};
use crossterm::{
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
};
use std::io::{self, Write};

use crate::config::Config;

/// Simple terminal output renderer
pub struct Renderer {
    config: Config,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Print a command with highlighting
    pub fn print_command(&self, command: &str) -> Result<()> {
        self.print_header("💡 Command")?;

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Green),
            Print("   "),
            Print(command),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Print an explanation
    pub fn _print_explanation(&self, explanation: &str) -> Result<()> {
        self.print_header("📝 Explanation")?;

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Yellow),
            Print("   "),
        )?;

        // Print explanation with proper wrapping
        for line in explanation.lines() {
            println!("   {}", line);
        }

        execute!(io::stdout(), ResetColor)?;

        Ok(())
    }

    /// Print a warning
    pub fn print_warning(&self, message: &str) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Red),
            Print("⚠️  Warning: "),
            ResetColor,
            Print(message),
            Print("\n"),
        )?;

        Ok(())
    }

    /// Print an info message
    pub fn print_info(&self, message: &str) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Blue),
            Print("ℹ️  Info: "),
            ResetColor,
            Print(message),
            Print("\n"),
        )?;

        Ok(())
    }

    /// Print a success message
    pub fn print_success(&self, message: &str) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Green),
            Print("✅ "),
            ResetColor,
            Print(message),
            Print("\n"),
        )?;

        Ok(())
    }

    /// Print an error message
    pub fn print_error(&self, message: &str) -> Result<()> {
        execute!(
            io::stderr(),
            SetForegroundColor(Color::Red),
            Print("❌ Error: "),
            ResetColor,
            Print(message),
            Print("\n"),
        )?;

        Ok(())
    }

    /// Print a section header
    fn print_header(&self, title: &str) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print(title),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Print a thinking/spinner state
    pub fn print_thinking(&self) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Magenta),
            Print("🤔 Thinking"),
            ResetColor,
            Print("..."),
            Print("\n"),
        )?;

        Ok(())
    }

    /// Prompt user to run a command
    pub fn prompt_run_command(&self, command: &str) -> Result<bool> {
        if !self.config.behavior.show_run_prompt {
            return Ok(false);
        }

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Cyan),
            Print("\n🏃 Run `"),
            Print(command),
            Print("`? [y/N]: "),
            ResetColor
        )?;

        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let response = input.trim().to_lowercase();
        Ok(response == "y" || response == "yes")
    }

    /// Copy text to clipboard (if feature enabled)
    #[cfg(feature = "clipboard")]
    pub fn _copy_to_clipboard(&self, text: &str) -> Result<()> {
        use arboard::Clipboard;

        let mut clipboard = Clipboard::new().context("Failed to access clipboard")?;

        clipboard
            .set_text(text)
            .context("Failed to copy to clipboard")?;

        self.print_info("Copied to clipboard!")?;

        Ok(())
    }

    /// Render markdown-style output (simple implementation)
    pub fn render_markdown(&self, text: &str) -> Result<()> {
        for line in text.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("```") {
                // Code block - skip markers, print content
                continue;
            } else if trimmed.starts_with('#') {
                // Headers
                let level = trimmed.chars().take_while(|&c| c == '#').count();
                let content = &trimmed[level..].trim();

                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Yellow),
                    Print("\n"),
                    Print("#".repeat(level)),
                    Print(" "),
                    Print(content),
                    Print("\n"),
                    ResetColor
                )?;
            } else if trimmed.starts_with('*') || trimmed.starts_with('-') {
                // List items
                execute!(
                    io::stdout(),
                    Print(" • "),
                    Print(&trimmed[1..].trim()),
                    Print("\n"),
                )?;
            } else if !trimmed.is_empty() {
                // Regular text
                println!(" {}", trimmed);
            } else {
                // Empty line
                println!();
            }
        }

        Ok(())
    }

    /// Print streaming content (partial response)
    pub fn _print_stream_chunk(&self, chunk: &str) -> Result<()> {
        print!("{}", chunk);
        io::stdout().flush()?;
        Ok(())
    }

    /// Clear current line (useful for replacing thinking indicator)
    pub fn clear_line(&self) -> Result<()> {
        execute!(
            io::stdout(),
            crossterm::cursor::MoveToColumn(0),
            crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_renderer_creation() {
        let config = Config::create_sample();
        let _renderer = Renderer::new(config);
        // Just verify it doesn't panic
    }
}
