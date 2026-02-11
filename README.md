ask(1)
======
> As simple as using man, but smarter

NAME
----
`ask` - terminal AI assistant for command suggestion, command explanation, and interactive chat.

SYNOPSIS
--------
`ask [OPTIONS] [QUERY]...`

`ask explain [COMMAND]...`

`ask chat`

`ask config <show|set|edit|path>`

`ask setup`

DESCRIPTION
-----------
`ask` is a single-binary CLI tool that uses an OpenAI-compatible API.

It provides three working modes:

1. command mode (default): suggest one command and short explanation for a query.
2. explain mode (`-e` or `explain`): explain a command or piped input.
3. chat mode (`-c` or `chat`): multi-turn interactive assistant.

The tool supports streaming output in `-e` and `chat` mode.

COMMANDS
--------
`explain` (`x`)
: explain a command or pipeline.

`chat` (`c`)
: start interactive mode.

`config` (`cfg`)
: configuration management.

`setup` (`s`)
: create initial configuration file.

OPTIONS
-------
`-p, --profile <PROFILE>`
: select profile from config.

`-e, --explain`
: explain mode without subcommand.

`-c, --chat`
: chat mode without subcommand.

`--copy`
: copy output (requires clipboard feature/tools).

`-h, --help`
: show help.

`-V, --version`
: show version.

CONFIGURATION
-------------
Configuration file:

`~/.config/ask/config.toml`

Minimal example:

```toml
[behavior]
default_mode = "command"
show_run_prompt = true
language = "auto"

[profiles.default]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"
model = "gpt-4o-mini"
max_tokens = 1024
temperature = 0.3
stream = true
timeout = 30
```

Common commands:

```bash
ask setup
ask config show
ask config set api_key sk-...
ask config set model gpt-4o-mini
```

EXECUTION POLICY
----------------
In command mode, the model can return `auto_execute=true`.

`ask` executes only when all checks pass:

1. command is complete (no placeholders like `<name>`).
2. command is considered safe enough for direct run.
3. user confirms `y/yes` on prompt.

If checks fail, command is shown but not executed.

EXAMPLES
--------
Command suggestion:

```bash
ask 'how to list files'
```

Explain a command:

```bash
ask -e 'ls -la'
```

Explain piped input:

```bash
ps aux | ask -e 'which process uses most CPU?'
```

Query with piped context:

```bash
uptime | ask 'is this load normal?'
```

Interactive chat:

```bash
ask -c
# or
ask chat
```

FILES
-----
`~/.config/ask/config.toml`
: runtime configuration.

`target/release/ask`
: compiled binary.

BUILD
-----
```bash
cargo build
cargo build --release
cargo test
```

NOTES
-----
- API endpoint must be OpenAI-compatible (`/chat/completions`).
- Current implementation targets Unix-like terminals (macOS/Linux).
- Dangerous commands are guarded with warnings and confirmation flow.

LICENSE
-------
MIT. See `LICENSE`.
