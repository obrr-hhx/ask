use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ask")]
#[command(about = "Terminal AI assistant for commands - like man, but smarter", long_about = None)]
#[command(version)]
pub struct Cli {
    /// Profile to use (from config file)
    #[arg(short = 'p', long = "profile", global = true)]
    pub profile: Option<String>,

    /// The question to ask (for command mode)
    #[arg(trailing_var_arg = true)]
    pub query: Vec<String>,

    /// Brief output (command only)
    #[arg(short = 'b', long, global = true)]
    pub brief: bool,

    /// Detailed explanation
    #[arg(short = 'd', long, global = true)]
    pub detail: bool,

    /// Explain a command (equivalent to --explain flag)
    #[arg(short = 'e', long, global = true)]
    pub explain: bool,

    /// Copy result to clipboard
    #[arg(long, global = true)]
    pub copy: bool,

    /// Start interactive chat mode (shortcut)
    #[arg(short = 'c', long = "chat", global = true)]
    pub chat: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Explain a command or pipeline
    #[command(visible_alias = "x")]
    Explain {
        /// Command to explain (or read from stdin)
        command: Vec<String>,
    },

    /// Start interactive chat mode
    #[command(visible_alias = "c")]
    Chat,

    /// Configuration management
    #[command(visible_alias = "cfg")]
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },

    /// Initial setup wizard
    #[command(visible_alias = "s")]
    Setup,
}

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// Show current configuration
    #[command(visible_alias = "sh")]
    Show,

    /// Set a configuration value
    Set {
        /// Configuration key (e.g., 'api_key', 'model')
        key: String,

        /// Configuration value
        value: String,

        /// Profile to update (uses active profile if not specified)
        #[arg(short = 'p', long = "profile")]
        profile: Option<String>,
    },

    /// Edit configuration file directly
    #[command(visible_alias = "e")]
    Edit {
        /// Open file in default editor
        #[arg(short = 'E', long)]
        editor: bool,
    },

    /// Path to configuration file
    Path,
}

// Helper function to get query as a string
pub fn get_query_string(query: &[String]) -> String {
    if query.is_empty() {
        String::new()
    } else {
        query.join(" ")
    }
}

// Helper function to get command as a string
pub fn get_command_string(command: &[String]) -> String {
    if command.is_empty() {
        String::new()
    } else {
        command.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_query_string() {
        assert_eq!(get_query_string(&[]), "");
        assert_eq!(
            get_query_string(&[
                "how".to_string(),
                "to".to_string(),
                "list".to_string(),
                "files".to_string()
            ]),
            "how to list files"
        );
    }
}
