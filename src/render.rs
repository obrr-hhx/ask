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

#[derive(Default)]
pub struct MarkdownStreamState {
    buffer: String,
    in_code_block: bool,
}

impl Renderer {
    /// Create a new renderer
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Print a command with highlighting
    pub fn print_command(&self, command: &str) -> Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Green),
            Print(command),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Print an explanation
    pub fn _print_explanation(&self, explanation: &str) -> Result<()> {
        self._print_header("📝 Explanation")?;

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

    /// Print a section header
    fn _print_header(&self, title: &str) -> Result<()> {
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
        // Intentionally silent: user requested no decorative thinking output.
        Ok(())
    }

    /// Prompt user to run a command
    pub fn prompt_run_command(&self, command: &str) -> Result<bool> {
        self.prompt_confirm(&format!("Auto execute `{}`", command))
    }

    /// Prompt user for a y/N confirmation in blue
    pub fn prompt_confirm(&self, question: &str) -> Result<bool> {
        if !self.config.behavior.show_run_prompt {
            return Ok(false);
        }

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Blue),
            Print(question),
            Print("? [y/N]: "),
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

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Blue),
            Print("Copied to clipboard!"),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Render markdown-style output (simple implementation)
    pub fn render_markdown(&self, text: &str) -> Result<()> {
        let mut state = MarkdownStreamState::default();
        for line in text.lines() {
            self.render_markdown_line(line, &mut state.in_code_block)?;
        }
        self.finish_markdown_stream(&mut state)?;

        Ok(())
    }

    /// Render markdown incrementally for streaming output
    pub fn render_markdown_stream_chunk(
        &self,
        chunk: &str,
        state: &mut MarkdownStreamState,
    ) -> Result<()> {
        state.buffer.push_str(chunk);

        while let Some(newline_pos) = state.buffer.find('\n') {
            let line = state.buffer[..newline_pos].to_string();
            state.buffer.drain(..=newline_pos);
            self.render_markdown_line(&line, &mut state.in_code_block)?;
        }

        Ok(())
    }

    pub fn finish_markdown_stream(&self, state: &mut MarkdownStreamState) -> Result<()> {
        if !state.buffer.trim_end().is_empty() {
            let remaining = state.buffer.clone();
            state.buffer.clear();
            self.render_markdown_line(&remaining, &mut state.in_code_block)?;
        }
        if state.in_code_block {
            state.in_code_block = false;
            println!();
        }
        Ok(())
    }

    fn render_markdown_line(&self, line: &str, in_code_block: &mut bool) -> Result<()> {
        let trimmed = line.trim_end();
        let compact = trimmed.trim();

        if compact.starts_with("```") {
            *in_code_block = !*in_code_block;
            if !*in_code_block {
                println!();
            }
            return Ok(());
        }

        if *in_code_block {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::DarkGrey),
                Print("  "),
                Print(trimmed),
                Print("\n"),
                ResetColor
            )?;
            return Ok(());
        }

        if compact.is_empty() {
            println!();
            return Ok(());
        }

        if is_horizontal_rule(compact) {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::DarkGrey),
                Print("────────────────────────\n"),
                ResetColor
            )?;
            return Ok(());
        }

        if is_table_separator(compact) {
            return Ok(());
        }

        if let Some((level, content)) = parse_heading(compact) {
            let color = match level {
                1 => Color::Yellow,
                2 => Color::Cyan,
                3 => Color::Green,
                _ => Color::White,
            };
            execute!(
                io::stdout(),
                SetForegroundColor(color),
                Print("\n"),
                Print(strip_inline_markdown(content)),
                Print("\n"),
                ResetColor
            )?;
            return Ok(());
        }

        if compact.starts_with('>') {
            let quoted = compact.trim_start_matches('>').trim();
            execute!(
                io::stdout(),
                SetForegroundColor(Color::DarkGrey),
                Print("│ "),
                Print(strip_inline_markdown(quoted)),
                Print("\n"),
                ResetColor
            )?;
            return Ok(());
        }

        if let Some((index, content)) = parse_ordered_list(compact) {
            let normalized = strip_inline_markdown(content);
            if is_section_like(&normalized) {
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Cyan),
                    Print("\n"),
                    Print(format!("{}. {}", index, normalized)),
                    Print("\n"),
                    ResetColor
                )?;
            } else {
                println!("{}. {}", index, normalized);
            }
            return Ok(());
        }

        if let Some((content, raw_content)) = parse_unordered_list(compact) {
            if is_horizontal_rule(content)
                || content == "--"
                || content == "—"
                || content == "——"
                || is_horizontal_rule(raw_content)
            {
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::DarkGrey),
                    Print("────────────────────────\n"),
                    ResetColor
                )?;
                return Ok(());
            }

            let normalized = strip_inline_markdown(content);
            if is_section_like(&normalized) || is_bold_only_list_item(raw_content) {
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Cyan),
                    Print("\n"),
                    Print(normalized),
                    Print("\n"),
                    ResetColor
                )?;
            } else {
                println!("• {}", normalized);
            }
            return Ok(());
        }

        if compact.starts_with('|') && compact.ends_with('|') {
            let cells: Vec<&str> = compact
                .trim_matches('|')
                .split('|')
                .map(|part| part.trim())
                .collect();
            println!("| {} |", cells.join(" | "));
            return Ok(());
        }

        println!("{}", strip_inline_markdown(compact));
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

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let level = line.chars().take_while(|&c| c == '#').count();
    if level == 0 {
        return None;
    }
    Some((level, line[level..].trim()))
}

fn parse_unordered_list(line: &str) -> Option<(&str, &str)> {
    let bytes = line.as_bytes();
    if bytes.len() >= 2
        && (bytes[0] == b'-' || bytes[0] == b'*' || bytes[0] == b'+')
        && bytes[1] == b' '
    {
        let raw = &line[2..];
        return Some((raw.trim(), raw));
    }
    None
}

fn parse_ordered_list(line: &str) -> Option<(usize, &str)> {
    let bytes = line.as_bytes();
    let mut digit_end = 0;
    while digit_end < bytes.len() && bytes[digit_end].is_ascii_digit() {
        digit_end += 1;
    }

    if digit_end == 0
        || digit_end + 1 >= bytes.len()
        || bytes[digit_end] != b'.'
        || bytes[digit_end + 1] != b' '
    {
        return None;
    }

    let index = line[..digit_end].parse::<usize>().ok()?;
    Some((index, line[digit_end + 2..].trim()))
}

fn is_horizontal_rule(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 3 {
        return false;
    }

    let mut chars = compact.chars();
    let first = match chars.next() {
        Some(c) => c,
        None => return false,
    };

    if first != '-' && first != '*' && first != '_' {
        return false;
    }

    chars.all(|c| c == first)
}

fn is_table_separator(line: &str) -> bool {
    if !(line.starts_with('|') && line.ends_with('|')) {
        return false;
    }

    let parts: Vec<&str> = line.trim_matches('|').split('|').collect();
    if parts.is_empty() {
        return false;
    }

    parts.into_iter().all(|part| {
        let t = part.trim();
        !t.is_empty() && t.chars().all(|c| c == '-' || c == ':')
    })
}

fn strip_inline_markdown(input: &str) -> String {
    let mut s = input
        .replace("**", "")
        .replace("__", "")
        .replace("~~", "")
        .replace('`', "");

    if (s.starts_with('*') && s.ends_with('*') && s.len() > 1)
        || (s.starts_with('_') && s.ends_with('_') && s.len() > 1)
    {
        s = s[1..s.len() - 1].to_string();
    }

    s
}

fn is_bold_only_list_item(content: &str) -> bool {
    let t = content.trim();
    t.starts_with("**") && t.ends_with("**") && t.len() > 4
}

fn is_section_like(text: &str) -> bool {
    let t = text.trim();
    if t.is_empty() {
        return false;
    }

    // Headline-like short labels should not be rendered as bullets.
    if t.len() <= 42
        && !t.contains(':')
        && !t.contains('.')
        && t.chars().filter(|c| c.is_alphabetic()).count() > 0
    {
        return t.chars().next().map(|c| c.is_uppercase()).unwrap_or(false);
    }

    false
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

    #[test]
    fn test_parse_ordered_list() {
        assert_eq!(parse_ordered_list("1. first item"), Some((1, "first item")));
        assert_eq!(parse_ordered_list("10. tenth"), Some((10, "tenth")));
        assert_eq!(parse_ordered_list("not a list"), None);
    }

    #[test]
    fn test_parse_unordered_list() {
        assert_eq!(parse_unordered_list("- item").map(|(x, _)| x), Some("item"));
        assert_eq!(parse_unordered_list("* item").map(|(x, _)| x), Some("item"));
        assert_eq!(parse_unordered_list("text"), None);
    }

    #[test]
    fn test_is_horizontal_rule() {
        assert!(is_horizontal_rule("---"));
        assert!(is_horizontal_rule("***"));
        assert!(is_horizontal_rule("_ _ _"));
        assert!(!is_horizontal_rule("--"));
        assert!(!is_horizontal_rule("- item"));
    }

    #[test]
    fn test_strip_inline_markdown() {
        assert_eq!(
            strip_inline_markdown("**Bold** and `code` text"),
            "Bold and code text"
        );
        assert_eq!(strip_inline_markdown("*emphasis*"), "emphasis");
    }

    #[test]
    fn test_section_detection() {
        assert!(is_section_like("Command Overview"));
        assert!(!is_section_like("Purpose: lists files"));
        assert!(!is_section_like(
            "this is a long sentence that should be body text"
        ));
    }
}
