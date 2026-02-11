use anyhow::Result;
use crossterm::{
    execute,
    style::Print,
};
use std::io::{self, Write};

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
        self.renderer
            .print_success("Starting interactive chat mode. Type /exit or press Ctrl+D to quit.")?;

        let client = AIClient::new(&self.profile)?;
        let system_prompt =
            SystemPrompt::Chat.get_prompt(&self.system_info, &self.config.behavior.language);

        // Test connection first
        self.renderer.print_info("Connecting to AI provider...")?;
        match client.test_connection().await {
            Ok(_) => self.renderer.print_success("Connected!")?,
            Err(e) => {
                self.renderer
                    .print_error(&format!("Failed to connect: {}", e))?;
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
                    if query.is_empty() || is_exit_command(query) {
                        break;
                    }

                    // Process the query
                    if let Err(e) = self.process_query(&client, &system_prompt, query).await {
                        self.renderer.print_error(&format!("Error: {}", e))?;
                    }
                }
                Err(e) => {
                    self.renderer
                        .print_error(&format!("Failed to read input: {}", e))?;
                    break;
                }
            }
        }

        println!();
        self.renderer.print_success("Goodbye!")?;

        Ok(())
    }

    /// Process a single query and display the response
    async fn process_query(
        &self,
        client: &AIClient,
        system_prompt: &str,
        query: &str,
    ) -> Result<()> {
        // Show thinking indicator
        self.renderer.print_thinking()?;

        // Get full response
        self.get_full_response(client, system_prompt, query).await?;

        println!();

        Ok(())
    }

    /// Get full response and print at once
    async fn get_full_response(
        &self,
        client: &AIClient,
        system_prompt: &str,
        query: &str,
    ) -> Result<()> {
        // Get full response
        let response = client.chat(system_prompt, query).await?;

        // Clear the "Thinking..." line
        self.renderer.clear_line()?;

        // Print AI prefix
        execute!(
            io::stdout(),
            crossterm::style::SetForegroundColor(crossterm::style::Color::Cyan),
            Print("\nAI: "),
            crossterm::style::ResetColor
        )?;

        // Print the response
        self.renderer.render_markdown(&response)?;

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
