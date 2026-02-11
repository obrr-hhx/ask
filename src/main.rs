mod chat;
mod cli;
mod client;
mod config;
mod prompt;
mod render;
mod safety;
mod system;

use anyhow::{Context, Result};
use clap::Parser;
use cli::{Cli, Commands, ConfigCommands};
use config::Config;
use render::Renderer;
use serde::Deserialize;
use std::io::Read;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let mut cli = Cli::parse();

    // Support `ask -c` as shortcut for chat mode
    if cli.chat && cli.command.is_none() {
        cli.command = Some(Commands::Chat);
    }

    // Handle setup command early (before config required)
    if let Some(Commands::Setup) = cli.command {
        config::run_setup()?;
        return Ok(());
    }

    // Execute the appropriate command
    match cli.command {
        Some(Commands::Config { subcommand }) => {
            handle_config_command(subcommand, cli.profile)?;
            Ok(())
        }
        _ => {
            // For commands that need config, load it
            let config = match Config::load(cli.profile.as_deref()) {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!("Failed to load config: {}", e);
                    eprintln!("Error: Failed to load configuration.");
                    eprintln!("Run 'ask --setup' to create a configuration file.");
                    std::process::exit(1);
                }
            };

            info!(
                "Loaded configuration for profile: {}",
                config.active_profile()
            );
            let _renderer = Renderer::new(config.clone());

            match cli.command {
                Some(Commands::Explain { command }) => {
                    let context = read_piped_input()?;
                    let command_text = cli::get_command_string(&command);
                    let explain_target = if command_text.trim().is_empty() && context.is_some() {
                        "Explain this piped input".to_string()
                    } else {
                        command_text
                    };

                    if explain_target.trim().is_empty() {
                        eprintln!("Error: No command provided.");
                        eprintln!("Usage: ask explain <command>");
                        std::process::exit(1);
                    }

                    execute_explain(&config, &explain_target, context, false).await
                }
                Some(Commands::Chat) => {
                    let chat = chat::ChatMode::new(config)?;
                    chat.run().await
                }
                Some(Commands::Config { .. }) | Some(Commands::Setup) => {
                    // Already handled above
                    Ok(())
                }
                None => {
                    let context = read_piped_input()?;

                    // No subcommand provided - default to ask mode with the query
                    if cli.query.is_empty() && context.is_none() {
                        // If no query provided, show help
                        eprintln!("Error: No query provided.");
                        eprintln!("Usage: ask <query>");
                        eprintln!("Example: ask 'how to list files'");
                        eprintln!("\nOr use a subcommand:");
                        eprintln!("  ask explain <cmd>   Explain a command");
                        eprintln!("  ask chat           Start interactive chat");
                        eprintln!("  ask config         Configuration management");
                        eprintln!("  ask setup          Initial setup wizard");
                        std::process::exit(1);
                    }
                    let mode = if cli.explain {
                        crate::prompt::SystemPrompt::Explain
                    } else {
                        crate::prompt::SystemPrompt::Command
                    };
                    let query = if cli.query.is_empty() {
                        if cli.explain {
                            "Explain this piped input".to_string()
                        } else {
                            "Based on the piped input, suggest the best terminal command."
                                .to_string()
                        }
                    } else {
                        cli::get_query_string(&cli.query)
                    };

                    execute_command(&config, mode, &query, context, cli.brief, cli.copy).await
                }
            }
        }
    }
}

/// Handle configuration subcommands
fn handle_config_command(subcommand: ConfigCommands, profile: Option<String>) -> Result<()> {
    match subcommand {
        ConfigCommands::Show => {
            let config = Config::load(profile.as_deref())?;
            println!("{}", toml::to_string_pretty(&config)?);
        }
        ConfigCommands::Set {
            key,
            value,
            profile,
        } => {
            let profile_to_use = profile.as_deref().or_else(|| profile.as_deref());
            let mut config = if let Ok(cfg) = Config::load(profile.as_deref()) {
                cfg
            } else {
                // Create default config if none exists
                eprintln!("No configuration found. Creating a new one...");
                Config::create_sample()
            };

            config.update(&key, &value, profile_to_use)?;
            println!("✅ Updated {} = {}", key, value);
        }
        ConfigCommands::Edit { editor: _ } => {
            let config_path = Config::config_path()?;
            println!("Configuration file: {}", config_path.display());

            // Open with default editor
            if let Some(editor) = std::env::var("EDITOR").ok() {
                std::process::Command::new(editor)
                    .arg(&config_path)
                    .status()
                    .context("Failed to open editor")?;
            } else {
                eprintln!("$EDITOR environment variable not set");
                eprintln!("Please edit the file manually: {}", config_path.display());
            }
        }
        ConfigCommands::Path => {
            let config_path = Config::config_path()?;
            println!("{}", config_path.display());
        }
    }

    Ok(())
}

/// Execute an ask command
async fn execute_command(
    config: &Config,
    mode: crate::prompt::SystemPrompt,
    query: &str,
    context: Option<String>,
    _brief: bool,
    _copy: bool,
) -> Result<()> {
    use crate::client::AIClient;
    use crate::prompt::SystemPrompt;
    use crate::render::Renderer;
    use crate::system::SystemInfo;

    let profile = config.get_active_profile()?;
    let client = AIClient::new(profile)?;
    let system_info = SystemInfo::detect();
    let renderer = Renderer::new(config.clone());

    // Get system prompt
    let system_prompt_text = mode.get_prompt(&system_info, &config.behavior.language);

    // Format user message
    let user_message = match mode {
        SystemPrompt::Command => SystemPrompt::format_command_query(query, context.as_deref()),
        SystemPrompt::Explain => SystemPrompt::format_explain_query(query, context.as_deref()),
        SystemPrompt::Chat => query.to_string(),
    };

    // Show thinking indicator
    renderer.print_thinking()?;

    // Get response
    let response = client.chat(&system_prompt_text, &user_message).await?;

    // Clear thinking line
    renderer.clear_line()?;

    // Print response based on mode
    match mode {
        SystemPrompt::Command => {
            let suggestion = parse_command_suggestion(&response);
            let explanation = suggestion
                .explanation
                .as_deref()
                .map(str::trim)
                .filter(|text| !text.is_empty());
            let mut explanation_rendered = false;

            if let Some(cmd) = suggestion.command.as_deref() {
                // Safety check
                if crate::safety::is_dangerous_command(cmd) {
                    renderer.print_warning(&crate::safety::get_dangerous_command_warning(cmd))?;

                    // Ask if user wants to see the command anyway
                    if !config.behavior.show_run_prompt
                        || !renderer.prompt_confirm("Show the command anyway")?
                    {
                        // Skip printing the full response if user doesn't want to see the command
                        return Ok(());
                    }
                }

                renderer.print_command(cmd)?;
                if let Some(text) = explanation {
                    renderer.render_markdown(text)?;
                    explanation_rendered = true;
                }

                if suggestion.auto_execute {
                    if is_auto_executable_command(cmd) && renderer.prompt_run_command(cmd)? {
                        std::process::Command::new("sh")
                            .arg("-c")
                            .arg(cmd)
                            .status()
                            .context("Failed to execute command")?;
                    }
                }
            }

            if !explanation_rendered && suggestion.command.is_none() {
                renderer.render_markdown(&response)?;
            }
        }
        _ => {
            renderer.render_markdown(&response)?;
        }
    }

    Ok(())
}

fn read_piped_input() -> Result<Option<String>> {
    if atty::isnt(atty::Stream::Stdin) {
        let mut piped_input = String::new();
        std::io::stdin().read_to_string(&mut piped_input)?;
        let trimmed = piped_input.trim();
        if !trimmed.is_empty() {
            return Ok(Some(format!("Piped input:\n{}", trimmed)));
        }
    }
    Ok(None)
}

#[derive(Debug, Deserialize)]
struct CommandResponse {
    command: Option<String>,
    explanation: Option<String>,
    #[serde(default)]
    auto_execute: bool,
}

#[derive(Debug)]
struct ParsedSuggestion {
    command: Option<String>,
    explanation: Option<String>,
    auto_execute: bool,
}

fn parse_command_suggestion(response: &str) -> ParsedSuggestion {
    if let Some(parsed) = parse_command_response_json(response) {
        let command = parsed
            .command
            .map(|c| c.trim().trim_start_matches('$').trim().to_string())
            .filter(|c| !c.is_empty());

        return ParsedSuggestion {
            command,
            explanation: parsed.explanation,
            auto_execute: parsed.auto_execute,
        };
    }

    ParsedSuggestion {
        command: extract_command_from_text(response),
        explanation: None,
        auto_execute: false,
    }
}

fn parse_command_response_json(response: &str) -> Option<CommandResponse> {
    if let Ok(parsed) = serde_json::from_str::<CommandResponse>(response) {
        return Some(parsed);
    }

    let mut in_code_block = false;
    let mut json_lines = Vec::new();
    for line in response.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            if in_code_block {
                let candidate = json_lines.join("\n");
                if let Ok(parsed) = serde_json::from_str::<CommandResponse>(&candidate) {
                    return Some(parsed);
                }
                json_lines.clear();
            }
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            json_lines.push(line);
        }
    }

    None
}

fn extract_command_from_text(response: &str) -> Option<String> {
    let lines: Vec<&str> = response.lines().collect();
    let mut in_code_block = false;
    for line in &lines {
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block && !line.trim().is_empty() {
            let cmd = line.trim().trim_start_matches('$').trim();
            if !cmd.starts_with('#') {
                return Some(cmd.to_string());
            }
        }
    }

    for line in &lines {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with("//") {
            return Some(trimmed.trim_start_matches('$').trim().to_string());
        }
    }

    None
}

fn is_auto_executable_command(command: &str) -> bool {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Placeholders and TODO-like hints indicate incomplete commands.
    if trimmed.contains('<')
        || trimmed.contains('>')
        || trimmed.contains('[')
        || trimmed.contains(']')
        || trimmed.contains("...")
        || trimmed.contains("YOUR_")
        || trimmed.contains("REPLACE_")
        || trimmed.contains("{")
        || trimmed.contains("}")
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json_command_suggestion() {
        let response = r#"{"command":"ls -la","explanation":"list files","auto_execute":true}"#;
        let parsed = parse_command_suggestion(response);

        assert_eq!(parsed.command.as_deref(), Some("ls -la"));
        assert_eq!(parsed.explanation.as_deref(), Some("list files"));
        assert!(parsed.auto_execute);
    }

    #[test]
    fn test_parse_markdown_code_block_fallback() {
        let response = "```bash\nls -la\n```\nList files";
        let parsed = parse_command_suggestion(response);
        assert_eq!(parsed.command.as_deref(), Some("ls -la"));
        assert!(!parsed.auto_execute);
    }

    #[test]
    fn test_auto_execute_placeholder_guard() {
        assert!(!is_auto_executable_command(
            "kubectl delete deployment <name>"
        ));
        assert!(!is_auto_executable_command("echo YOUR_TOKEN"));
        assert!(is_auto_executable_command("ls -la"));
    }
}

/// Execute an explain command
async fn execute_explain(
    config: &Config,
    command: &str,
    context: Option<String>,
    _copy: bool,
) -> Result<()> {
    use crate::client::AIClient;
    use crate::prompt::SystemPrompt;
    use crate::render::Renderer;
    use crate::system::SystemInfo;

    let profile = config.get_active_profile()?;
    let client = AIClient::new(profile)?;
    let system_info = SystemInfo::detect();
    let renderer = Renderer::new(config.clone());

    // Get system prompt
    let system_prompt_text =
        SystemPrompt::Explain.get_prompt(&system_info, &config.behavior.language);

    // Format user message
    let user_message = SystemPrompt::format_explain_query(command, context.as_deref());

    let mut stream_state = crate::render::MarkdownStreamState::default();
    match client
        .chat_stream(&system_prompt_text, &user_message, |chunk| {
            renderer.render_markdown_stream_chunk(chunk, &mut stream_state)
        })
        .await
    {
        Ok(_) => {
            renderer.finish_markdown_stream(&mut stream_state)?;
            println!();
        }
        Err(_) => {
            // Fallback for providers that don't support streaming.
            let response = client.chat(&system_prompt_text, &user_message).await?;
            renderer.render_markdown(&response)?;
        }
    }

    Ok(())
}
