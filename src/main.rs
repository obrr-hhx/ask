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
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

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
                    // Check if we have piped input
                    let mut piped_input = String::new();
                    if atty::isnt(atty::Stream::Stdin) {
                        use std::io::Read;
                        std::io::stdin().read_to_string(&mut piped_input)?;
                    }

                    let context = if !piped_input.is_empty() {
                        Some(format!("Piped input:\n{}", piped_input))
                    } else {
                        None
                    };

                    execute_explain(&config, &cli::get_command_string(&command), context, false)
                        .await
                }
                Some(Commands::Chat) => {
                    // TODO: Implement chat mode
                    let chat = chat::ChatMode::new(config)?;
                    chat.run().await
                }
                Some(Commands::Config { .. }) | Some(Commands::Setup) => {
                    // Already handled above
                    Ok(())
                }
                None => {
                    // No subcommand provided - default to ask mode with the query
                    if cli.query.is_empty() {
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
                    execute_command(&config, mode, &cli::get_query_string(&cli.query), cli.brief, cli.copy)
                        .await
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
        SystemPrompt::Command => SystemPrompt::format_command_query(query, None),
        SystemPrompt::Explain => SystemPrompt::format_explain_query(query, None),
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
            // Extract command from response
            let lines: Vec<&str> = response.lines().collect();
            let mut command_found = None;

            // Look for code block or plain command
            let mut in_code_block = false;
            for line in &lines {
                if line.trim_start().starts_with("```") {
                    in_code_block = !in_code_block;
                    continue;
                }

                if in_code_block && !line.trim().is_empty() {
                    // Found command in code block
                    let cmd = line.trim().trim_start_matches('$').trim();
                    if !cmd.starts_with("#") {
                        command_found = Some(cmd);
                        break;
                    }
                }
            }

            // If no code block found, look for the first line that looks like a command
            if command_found.is_none() {
                for line in &lines {
                    let trimmed = line.trim();
                    if !trimmed.is_empty()
                        && !trimmed.starts_with("#")
                        && !trimmed.starts_with("//")
                    {
                        command_found = Some(trimmed.trim_start_matches('$').trim());
                        break;
                    }
                }
            }

            // Display command with safety check
            if let Some(cmd) = command_found {
                // Safety check
                if crate::safety::is_dangerous_command(cmd) {
                    renderer.print_warning(&crate::safety::get_dangerous_command_warning(cmd))?;

                    // Ask if user wants to see the command anyway
                    if !config.behavior.show_run_prompt
                        || !renderer.prompt_run_command("show the command")?
                    {
                        // Skip printing the full response if user doesn't want to see the command
                        return Ok(());
                    }
                }

                renderer.print_command(cmd)?;

                // Prompt to run the command
                if config.behavior.show_run_prompt && renderer.prompt_run_command(cmd)? {
                    // Execute the command
                    renderer.print_info(&format!("Executing: {}", cmd))?;
                    std::process::Command::new("sh")
                        .arg("-c")
                        .arg(cmd)
                        .status()
                        .context("Failed to execute command")?;
                }
            }

            // Show the full explanation
            renderer.render_markdown(&response)?;
        }
        _ => {
            renderer.render_markdown(&response)?;
        }
    }

    Ok(())
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

    // Show thinking indicator
    renderer.print_thinking()?;

    // Get response
    let response = client.chat(&system_prompt_text, &user_message).await?;

    // Clear thinking line
    renderer.clear_line()?;

    // Print explanation
    renderer.render_markdown(&response)?;

    Ok(())
}
