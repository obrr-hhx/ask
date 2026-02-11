# `ask` - Terminal AI Assistant

> As simple as using `man`, but smarter

`ask` is a terminal AI assistant that lets you ask about command usage conversationally and get precise answers—without leaving your terminal.

## Features

- 🤖 **AI-Powered**: Intelligent responses based on OpenAI GPT models
- ⚡ **Multi-Mode Support**: Ask mode, Explain mode, and Chat mode
- 🛡️ **Safety Protection**: Automatically detects dangerous commands and warns
- 🔧 **Multi-Provider Support**: Supports OpenAI, DeepSeek, Ollama, and more
- 📦 **Zero Dependencies**: Single binary, no additional installation needed
- 🎨 **Colorful Output**: Beautiful terminal UI with Markdown rendering support

## Quick Start

### Installation

```bash
# Build from source (requires Rust)
git clone https://github.com/obrr-hhx/ask
cd ask
cargo build --release

# Binary is at target/release/ask
sudo cp target/release/ask /usr/local/bin/
```

### Initial Configuration

Configure your API key before first use:

```bash
# Interactive setup wizard
ask --setup

# Or set manually
ask config set api_key sk-your-openai-api-key

# View configuration
ask config show
```

Configuration file is located at `~/.config/ask/config.toml`.

## Usage

### 1. Ask Mode - Ask How to Do Something

```bash
# Basic usage
ask 'how to list all files'

# Result:
💡 Command:
   ls -la

# Detailed explanation
ask --detail 'how to compress a folder'

# Show only command (brief mode)
ask --brief 'how to find large files'

# Auto-execute suggested command
ask 'how to check disk usage'
# ...then type 'y' to execute
```

### 2. Explain Mode - Understand Commands

```bash
# Explain a single command
ask -e "tar -xzvf archive.tar.gz"

# Explain complex pipelines
ask -e "ps aux | grep python | awk '{print $2}' | xargs kill"

# Explain from pipe input
ps aux | ask -e "what processes are using the most CPU?"
```

### 3. Chat Mode - Interactive Conversation

```bash
# Start interactive chat
ask chat

# Or shorthand
ask -c

# In chat mode you can:
# - Have multi-turn conversations
# - Ask any terminal-related questions
# - Type /exit or Ctrl+D to exit
```

### 4. Configuration Management

```bash
# Show current configuration
ask config show

# Modify settings
ask config set model gpt-4o
ask config set temperature 0.7

# Show config file path
ask config path

# Edit config file directly
ask config edit
```

## Advanced Features

### Multi-Profile Support

Add multiple profiles in `~/.config/ask/config.toml`:

```toml
[profiles.default]
api_key = "sk-openai-key"
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"

[profiles.deepseek]
api_key = "sk-deepseek-key"
base_url = "https://api.deepseek.com/v1"
model = "deepseek-chat"

[profiles.local]
api_key = "not-needed"
base_url = "http://localhost:11434/v1"
model = "llama3"
```

Switch profiles:

```bash
# Use deepseek profile
ask -p deepseek 'how to use git rebase'

# Show specific profile configuration
ask -p local config show
```

### Pipeline Integration

```bash
# Explain error logs
cat error.log | ask -e "what's causing these errors?"

# Analyze system load
uptime | ask "is this load normal?"

# Process JSON data
cat data.json | ask -e "extract the user names from this JSON"
```

### Clipboard Support

```bash
# Copy results to clipboard
ask --copy "regex to match email addresses"

# On Mac/Linux, requires xclip/xsel/pbcopy tools
```

## Configuration Options

### Global Configuration

```toml
[behavior]
default_mode = "command"      # Default mode: command | explain | chat
show_run_prompt = true        # Whether to prompt for command execution
language = "auto"             # Language: zh | en | auto
brief_by_default = false      # Default brief mode
save_history = true          # Whether to save history
max_history = 1000           # Maximum history entries
```

### Profile Configuration

```toml
[profiles.default]
api_key = "sk-xxxxxxxx"      # API key
base_url = "https://api.openai.com/v1"  # API endpoint
model = "gpt-4o-mini"        # Model name
max_tokens = 1024            # Maximum tokens
temperature = 0.3           # Randomness (0.0-1.0)
stream = true               # Stream output
timeout = 30               # Timeout in seconds
```

## Supported AI Providers

- **OpenAI** (default): GPT-4, GPT-3.5-turbo
- **DeepSeek**: DeepSeek-Chat, DeepSeek-Coder
- **Ollama**: Locally running Llama, Mistral, and other models
- **Claude** (via API proxy)
- **Any OpenAI-compatible API**

## Security Features

`ask` has built-in security mechanisms to prevent dangerous commands:

- **Automatic Detection**: Identifies dangerous commands like `rm -rf`, `dd`, format commands, etc.
- **Interactive Confirmation**: Requires user confirmation before execution
- **Clear Warnings**: Displays distinct risk warnings
- **Educational**: Explains why commands are dangerous

**Example of dangerous commands flagged:**

```bash
rm -rf /                    # Delete all files
sudo rm -rf /usr            # Delete system directories
dd if=/dev/zero of=/dev/sda # Overwrite disk
shutdown -h now             # Shutdown
chmod -R 777 /              # Modify system permissions
```

## Troubleshooting

### 1. Connection Errors

```bash
# Test connection
ask config set base_url https://api.openai.com/v1

# Check API key
ask config show | grep api_key
```

### 2. Configuration Issues

```bash
# Re-run setup wizard
ask --setup

# Edit config file manually
ask config edit
```

### 3. Model-Related Issues

```bash
# Switch to cheaper model (gpt-4o-mini)
ask config set model gpt-4o-mini

# Switch to smarter model (gpt-4o)
ask config set model gpt-4o
```

### 4. Security Warnings

When you see security warnings:
- **Yellow warnings**: Need confirmation to display command
- **Red warnings**: Need double confirmation to execute

Use `--force` flag to skip warnings (not recommended).

## Development

### Build

```bash
# Development build
cargo build

# Release build (optimized)
cargo build --release

# Run tests
cargo test

# Check code
cargo clippy
```

### Architecture

```
src/
├── main.rs          # Program entry point and command routing
├── cli.rs           # Command-line argument parsing
├── config.rs        # Configuration management (TOML)
├── client.rs        # OpenAI API client
├── prompt.rs        # Prompt template system
├── render.rs        # Terminal output rendering
├── chat.rs          # Interactive chat mode
└── safety.rs        # Safety detection module
```

## Contributing

Contributions are welcome! Please:

1. Fork the project
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License - See [LICENSE](LICENSE) file for details.

## Acknowledgments

- OpenAI API for powerful language models
- Rust ecosystem for excellent crates (clap, reqwest, crossterm, etc.)
- Community feedback and suggestions

## Future Plans

- [ ] Local RAG (Retrieval-Augmented Generation) support
- [ ] Advanced streaming output
- [ ] Shell autocompletion
- [ ] Command execution history
- [ ] Plugin system
- [ ] Voice input support

## Issue Reporting

Encountered a problem or have suggestions? Please [create an Issue](https://github.com/obrr-hhx/ask/issues/new).

---

**Made with ❤️ by terminal lovers, for terminal lovers.**
