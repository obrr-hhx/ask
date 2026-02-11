use anyhow::Result;
use crossterm::{execute, style::Print};
use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::time::{Duration, sleep};

use crate::client::AIClient;
use crate::config::{Config, Profile};
use crate::prompt::SystemPrompt;
use crate::render::Renderer;
use crate::system::SystemInfo;

/// Interactive chat mode - REPL interface
pub struct ChatMode {
    config: Config,
    profile: Profile,
    system_info: SystemInfo,
    renderer: Renderer,
}

impl ChatMode {
    /// Create a new chat mode instance
    pub fn new(config: Config) -> Result<Self> {
        let profile = config.get_active_profile()?.clone();
        let system_info = SystemInfo::detect();
        let renderer = Renderer::new(config.clone());

        Ok(Self {
            config,
            profile,
            system_info,
            renderer,
        })
    }

    /// Run the interactive chat loop
    pub async fn run(&self) -> Result<()> {
        self.print_stdout_colored(
            crossterm::style::Color::Green,
            "Starting interactive chat mode. Type /exit or press Ctrl+D to quit.",
        )?;

        let client = AIClient::new(&self.profile)?;
        let system_prompt =
            SystemPrompt::Chat.get_prompt(&self.system_info, &self.config.behavior.language);
        let mut history: Vec<(String, String)> = Vec::new();

        // Test connection first
        self.print_stdout_colored(
            crossterm::style::Color::Blue,
            "Connecting to AI provider...",
        )?;
        match client.test_connection().await {
            Ok(_) => self.print_stdout_colored(crossterm::style::Color::Green, "Connected!")?,
            Err(e) => {
                self.print_stderr_colored(
                    crossterm::style::Color::Red,
                    &format!("Failed to connect: {}", e),
                )?;
                return Ok(());
            }
        }

        println!("\n{}", "─".repeat(50));

        // Main chat loop
        loop {
            // Print prompt
            execute!(
                io::stdout(),
                crossterm::style::SetForegroundColor(crossterm::style::Color::Green),
                Print("\nYou: "),
                crossterm::style::ResetColor
            )?;
            io::stdout().flush()?;

            // Read user input
            let mut input = String::new();
            match io::stdin().read_line(&mut input) {
                Ok(0) => {
                    // EOF (Ctrl+D)
                    break;
                }
                Ok(_) => {
                    let query = input.trim();

                    // Check for exit commands
                    if query.is_empty() {
                        continue;
                    }
                    if is_exit_command(query) {
                        break;
                    }

                    // Process the query
                    if let Err(e) = self
                        .process_query(&client, &system_prompt, &mut history, query)
                        .await
                    {
                        self.print_stderr_colored(crossterm::style::Color::Red, &format!("{}", e))?;
                    }
                }
                Err(e) => {
                    self.print_stderr_colored(
                        crossterm::style::Color::Red,
                        &format!("Failed to read input: {}", e),
                    )?;
                    break;
                }
            }
        }

        println!();
        self.print_stdout_colored(crossterm::style::Color::Green, "Goodbye!")?;

        Ok(())
    }

    /// Process a single query and display the response
    async fn process_query(
        &self,
        client: &AIClient,
        system_prompt: &str,
        history: &mut Vec<(String, String)>,
        query: &str,
    ) -> Result<()> {
        // Stream response
        let response = self
            .get_streaming_response(client, system_prompt, history, query)
            .await?;

        history.push(("user".to_string(), query.to_string()));
        history.push(("assistant".to_string(), response));

        println!();

        Ok(())
    }

    /// Stream response and render incrementally.
    async fn get_streaming_response(
        &self,
        client: &AIClient,
        system_prompt: &str,
        history: &[(String, String)],
        query: &str,
    ) -> Result<String> {
        let first_chunk = Arc::new(AtomicBool::new(false));
        let done = Arc::new(AtomicBool::new(false));
        let mut stream_state = crate::render::MarkdownStreamState::default();

        // Show AI loading indicator immediately.
        execute!(
            io::stdout(),
            crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
            Print("\nAI"),
            crossterm::style::ResetColor,
            Print(": ..."),
        )?;
        io::stdout().flush()?;

        // Animate dots until first token arrives.
        let first_chunk_for_spinner = first_chunk.clone();
        let done_for_spinner = done.clone();
        let spinner = tokio::spawn(async move {
            let frames = [".", "..", "..."];
            let mut idx = 0usize;
            loop {
                if first_chunk_for_spinner.load(Ordering::SeqCst)
                    || done_for_spinner.load(Ordering::SeqCst)
                {
                    break;
                }

                let dots = frames[idx % frames.len()];
                let _ = execute!(
                    io::stdout(),
                    crossterm::cursor::MoveToColumn(0),
                    crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                    crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
                    Print("AI"),
                    crossterm::style::ResetColor,
                    Print(format!(": {}", dots)),
                );
                let _ = io::stdout().flush();
                idx += 1;
                sleep(Duration::from_millis(220)).await;
            }
        });

        let first_chunk_for_cb = first_chunk.clone();
        let response = client
            .chat_stream_with_history(system_prompt, history, query, |chunk| {
                if !first_chunk_for_cb.swap(true, Ordering::SeqCst) {
                    execute!(
                        io::stdout(),
                        crossterm::cursor::MoveToColumn(0),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                        crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
                        Print("AI"),
                        crossterm::style::ResetColor,
                        Print(": "),
                    )?;
                    io::stdout().flush()?;
                }

                self.renderer
                    .render_markdown_stream_chunk(chunk, &mut stream_state)
            })
            .await;

        done.store(true, Ordering::SeqCst);
        let _ = spinner.await;

        let response = match response {
            Ok(resp) => resp,
            Err(stream_err) => {
                // If no stream token arrived, fallback to non-stream response.
                if !first_chunk.load(Ordering::SeqCst) {
                    execute!(
                        io::stdout(),
                        crossterm::cursor::MoveToColumn(0),
                        crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                    )?;
                    execute!(
                        io::stdout(),
                        crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
                        Print("AI"),
                        crossterm::style::ResetColor,
                        Print(": "),
                    )?;
                    io::stdout().flush()?;

                    let fallback = client
                        .chat_with_history(system_prompt, history, query)
                        .await?;
                    self.renderer.render_markdown(&fallback)?;
                    return Ok(fallback);
                }
                return Err(stream_err);
            }
        };

        if !first_chunk.load(Ordering::SeqCst) {
            execute!(
                io::stdout(),
                crossterm::cursor::MoveToColumn(0),
                crossterm::terminal::Clear(crossterm::terminal::ClearType::CurrentLine),
                crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
                Print("AI"),
                crossterm::style::ResetColor,
                Print(": "),
            )?;
            io::stdout().flush()?;
        }

        self.renderer.finish_markdown_stream(&mut stream_state)?;

        Ok(response)
    }

    fn print_stdout_colored(&self, color: crossterm::style::Color, message: &str) -> Result<()> {
        execute!(
            io::stdout(),
            crossterm::style::SetForegroundColor(color),
            Print(message),
            Print("\n"),
            crossterm::style::ResetColor
        )?;
        Ok(())
    }

    fn print_stderr_colored(&self, color: crossterm::style::Color, message: &str) -> Result<()> {
        execute!(
            io::stderr(),
            crossterm::style::SetForegroundColor(color),
            Print(message),
            Print("\n"),
            crossterm::style::ResetColor
        )?;
        Ok(())
    }
}

/// Check if the input is an exit command
fn is_exit_command(input: &str) -> bool {
    let input = input.trim().to_lowercase();
    matches!(input.as_str(), "/exit" | ":q" | ":quit" | "exit" | "quit")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_exit_command() {
        assert!(is_exit_command("/exit"));
        assert!(is_exit_command(":q"));
        assert!(is_exit_command(":quit"));
        assert!(is_exit_command("exit"));
        assert!(is_exit_command("quit"));
        assert!(is_exit_command("  /exit  "));
        assert!(is_exit_command("EXIT"));

        assert!(!is_exit_command("hello"));
        assert!(!is_exit_command(""));
        assert!(!is_exit_command("continue"));
    }
}
