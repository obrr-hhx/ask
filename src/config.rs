use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

// Default configuration values
const DEFAULT_API_BASE_URL: &str = "https://api.openai.com/v1";
const DEFAULT_MODEL: &str = "gpt-4o-mini";
const DEFAULT_MAX_TOKENS: u32 = 1024;
const DEFAULT_TEMPERATURE: f32 = 0.3;

/// Top-level configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default profile to use when none is specified
    #[serde(default = "default_profile")]
    pub default_profile: String,

    /// Application behavior settings
    #[serde(default)]
    pub behavior: Behavior,

    /// Named profiles for different providers/settings
    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

/// Default profile name
fn default_profile() -> String {
    "default".to_string()
}

/// Application behavior settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Behavior {
    /// Default mode to use (command, explain, chat)
    #[serde(default = "default_mode")]
    pub default_mode: String,

    /// Whether to show prompt to run command
    #[serde(default = "default_true")]
    pub show_run_prompt: bool,

    /// Preferred language for responses (zh, en, auto)
    #[serde(default = "default_language")]
    pub language: String,

    /// Whether to use brief mode by default
    #[serde(default)]
    pub brief_by_default: bool,

    /// Whether to save history
    #[serde(default = "default_true")]
    pub save_history: bool,

    /// Maximum history entries to keep
    #[serde(default = "default_max_history")]
    pub max_history: usize,
}

impl Default for Behavior {
    fn default() -> Self {
        Self {
            default_mode: default_mode(),
            show_run_prompt: default_true(),
            language: default_language(),
            brief_by_default: false,
            save_history: default_true(),
            max_history: default_max_history(),
        }
    }
}

fn default_mode() -> String {
    "command".to_string()
}

fn default_true() -> bool {
    true
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_max_history() -> usize {
    1000
}

/// Profile for a specific AI provider/configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// API key for authentication
    pub api_key: String,

    /// Base URL for the API (e.g., https://api.openai.com/v1)
    #[serde(default = "default_base_url")]
    pub base_url: String,

    /// Model to use (e.g., gpt-4o-mini)
    #[serde(default = "default_model")]
    pub model: String,

    /// Maximum tokens in response
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,

    /// Temperature for generation (0.0-1.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Whether to use streaming
    #[serde(default = "default_true")]
    pub stream: bool,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout: u64,
}

impl Profile {
    /// Create a new profile with minimum required fields
    pub fn new(api_key: String) -> Self {
        Self {
            api_key,
            base_url: default_base_url(),
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            stream: default_true(),
            timeout: default_timeout(),
        }
    }

    /// Update a field by key
    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "api_key" => self.api_key = value.to_string(),
            "base_url" => self.base_url = value.to_string(),
            "model" => self.model = value.to_string(),
            "max_tokens" => {
                self.max_tokens = value.parse().context("max_tokens must be a number")?;
            }
            "temperature" => {
                self.temperature = value
                    .parse()
                    .context("temperature must be a number between 0.0 and 1.0")?;
            }
            "stream" => {
                self.stream = value.parse().context("stream must be true or false")?;
            }
            "timeout" => {
                self.timeout = value.parse().context("timeout must be a number")?;
            }
            _ => anyhow::bail!("Unknown configuration key: {}", key),
        }
        Ok(())
    }
}

fn default_base_url() -> String {
    DEFAULT_API_BASE_URL.to_string()
}

fn default_model() -> String {
    DEFAULT_MODEL.to_string()
}

fn default_max_tokens() -> u32 {
    DEFAULT_MAX_TOKENS
}

fn default_temperature() -> f32 {
    DEFAULT_TEMPERATURE
}

fn default_timeout() -> u64 {
    30
}

impl Config {
    /// Load configuration from default locations
    pub fn load(profile: Option<&str>) -> Result<Self> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            anyhow::bail!("Config file not found at {:?}", config_path);
        }

        let contents = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {:?}", config_path))?;

        let mut config: Config =
            toml::from_str(&contents).context("Failed to parse config file as TOML")?;

        // If a specific profile is requested, make it the default
        if let Some(profile_name) = profile {
            config.default_profile = profile_name.to_string();
        }

        Ok(config)
    }

    /// Get the active profile
    pub fn active_profile(&self) -> &str {
        &self.default_profile
    }

    /// Get the active profile configuration
    pub fn get_active_profile(&self) -> Result<&Profile> {
        self.profiles.get(&self.default_profile).with_context(|| {
            format!(
                "Profile '{}' not found in configuration",
                self.default_profile
            )
        })
    }

    /// Create a sample configuration (for first-time setup)
    pub fn create_sample() -> Self {
        let mut profiles = HashMap::new();

        // Add default profile for OpenAI
        profiles.insert(
            "default".to_string(),
            Profile::new("YOUR_OPENAI_API_KEY_HERE".to_string()),
        );

        // Add profile for DeepSeek
        profiles.insert(
            "deepseek".to_string(),
            Profile {
                api_key: "YOUR_DEEPSEEK_API_KEY_HERE".to_string(),
                base_url: "https://api.deepseek.com/v1".to_string(),
                model: "deepseek-chat".to_string(),
                ..Profile::new(String::new())
            },
        );

        // Add profile for Ollama (local)
        profiles.insert(
            "local".to_string(),
            Profile {
                api_key: "not-needed".to_string(),
                base_url: "http://localhost:11434/v1".to_string(),
                model: "llama3".to_string(),
                ..Profile::new(String::new())
            },
        );

        Self {
            default_profile: "default".to_string(),
            behavior: Behavior::default(),
            profiles,
        }
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path()?;
        let config_dir = config_path
            .parent()
            .context("Could not get config directory")?;

        // Create directory if it doesn't exist
        if !config_dir.exists() {
            fs::create_dir_all(config_dir)
                .with_context(|| format!("Failed to create config directory: {:?}", config_dir))?;
        }

        let toml_str =
            toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;

        fs::write(&config_path, toml_str)
            .with_context(|| format!("Failed to write config to {:?}", config_path))?;

        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path() -> Result<PathBuf> {
        // Use ~/.config/ask/config.toml for both Linux and macOS
        let home = dirs::home_dir().context("Could not find home directory")?;
        Ok(home.join(".config").join("ask").join("config.toml"))
    }

    /// Update a configuration value
    pub fn update(&mut self, key: &str, value: &str, profile: Option<&str>) -> Result<()> {
        let profile_name = profile.unwrap_or(&self.default_profile);

        let profile = self
            .profiles
            .get_mut(profile_name)
            .with_context(|| format!("Profile '{}' not found", profile_name))?;

        profile.set(key, value)?;
        self.save()?;

        Ok(())
    }
}

/// Run interactive setup wizard
pub fn run_setup() -> Result<()> {
    use std::io::{self, Write};

    println!("=== Ask Terminal AI Assistant Setup ===\n");

    // Check if config already exists
    if Config::config_path()?.exists() {
        print!("Configuration file already exists. Overwrite? [y/N]: ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Setup cancelled.");
            return Ok(());
        }
    }

    println!("\nLet's configure your Ask installation!\n");

    // Create sample configuration
    let mut config = Config::create_sample();

    println!("Available profiles:");
    for (name, profile) in &config.profiles {
        println!("  - {} ({})", name, profile.model);
    }

    // Prompt for API key
    print!("\nEnter your OpenAI API key (or press Enter to skip): ");
    io::stdout().flush()?;

    let mut api_key = String::new();
    io::stdin().read_line(&mut api_key)?;

    if !api_key.trim().is_empty() {
        config.profiles.get_mut("default").unwrap().api_key = api_key.trim().to_string();
    }

    // Save configuration
    config.save()?;

    println!(
        "\n✅ Configuration saved to {}",
        Config::config_path()?.display()
    );
    println!("\nNext steps:");
    println!("  1. Edit the config file with your API keys");
    println!("  2. Run 'ask how to list files' to test");
    println!("  3. Run 'ask config set model gpt-4o' to change the model");
    println!("\nNote: Config file is located at ~/.config/ask/config.toml");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::create_sample();
        assert_eq!(config.default_profile, "default");
        assert!(config.profiles.contains_key("default"));
        assert!(config.profiles.contains_key("deepseek"));
        assert!(config.profiles.contains_key("local"));
    }

    #[test]
    fn test_profile_set() {
        let mut profile = Profile::new("test-key".to_string());

        assert!(profile.set("model", "gpt-4").is_ok());
        assert_eq!(profile.model, "gpt-4");

        assert!(profile.set("max_tokens", "2048").is_ok());
        assert_eq!(profile.max_tokens, 2048);
    }

    #[test]
    fn test_profile_set_invalid() {
        let mut profile = Profile::new("test-key".to_string());

        assert!(profile.set("max_tokens", "invalid").is_err());
        assert!(profile.set("unknown_key", "value").is_err());
    }

    #[test]
    fn test_behavior_default() {
        let behavior = Behavior::default();
        assert_eq!(behavior.default_mode, "command");
        assert!(behavior.show_run_prompt);
        assert_eq!(behavior.language, "auto");
    }

    #[test]
    fn test_config_path() {
        let path = Config::config_path().unwrap();

        // Should end with ask/config.toml
        assert!(path.ends_with("ask/config.toml"));

        // The filename should be config.toml
        assert_eq!(path.file_name().unwrap(), "config.toml");
    }
}
