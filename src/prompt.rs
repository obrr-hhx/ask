use crate::system::SystemInfo;

/// System prompts for different modes of operation
pub enum SystemPrompt {
    /// Default mode: asking how to do something
    Command,
    /// Explain mode: explaining a command or pipeline
    Explain,
    /// Chat mode: general conversation
    Chat,
}

impl SystemPrompt {
    /// Get the system prompt text for the given mode
    pub fn get_prompt(&self, sys_info: &SystemInfo, _language: &str) -> String {
        match self {
            SystemPrompt::Command => self.command_prompt(sys_info),
            SystemPrompt::Explain => self.explain_prompt(sys_info),
            SystemPrompt::Chat => self.chat_prompt(sys_info),
        }
    }

    /// Prompt for command mode - asking how to do things
    fn command_prompt(&self, sys_info: &SystemInfo) -> String {
        format!(
            r#"{}
You are a terminal command expert assistant. The user is working on {} with {}.

Rules:
1. When asked how to do something, respond with the exact command first, then a brief explanation.
2. Format commands in code blocks with bash language identifier.
3. If multiple approaches exist, show the most common/portable one first.
4. Always consider safety - warn about destructive commands.
5. Be concise. Terminal users value brevity.

Examples:
User: how to list all files in a directory
Assistant: ```bash
ls -la
```

Lists all files (including hidden) in long format."#,
            sys_info.format_context(),
            sys_info.os, sys_info.shell
        )
    }

    /// Prompt for explain mode - explaining commands
    fn explain_prompt(&self, sys_info: &SystemInfo) -> String {
        format!(
            r#"{}
You are a terminal command explainer expert. The user is using {} on {}.

Break down the provided command into parts and explain each component clearly. Explain:
- The command itself and what it does
- Each option/flag and what it means
- Pipes (|) and data flow
- Redirection (>, >>, <)
- Wildcards and patterns
- Any non-obvious behavior
- Potential risks or side effects

Use simple language, bullet points, and organize by clear categories.
If relevant, provide alternatives or simpler variations."#,
            sys_info.format_context(),
            sys_info.shell, sys_info.os
        )
    }

    /// Prompt for chat mode - general conversation
    fn chat_prompt(&self, sys_info: &SystemInfo) -> String {
        format!(
            r#"{}
You are a helpful terminal assistant. The user is on {} using {}.

You can help with:
- Terminal commands and usage
- Scripting and automation
- System administration tasks
- Troubleshooting
- Best practices

Be concise and helpful. Provide example commands when relevant.
You can ask clarifying questions to provide the best assistance."#,
            sys_info.format_context(),
            sys_info.os, sys_info.shell
        )
    }

    /// Format a user message for command mode
    pub fn format_command_query(query: &str, context: Option<&str>) -> String {
        if let Some(ctx) = context {
            format!(
                r#"Question: {}

Context: {}

Provide the command and brief explanation."#,
                query, ctx
            )
        } else {
            format!(
                r#"Question: {}

Provide the command and brief explanation."#,
                query
            )
        }
    }

    /// Format a user message for explain mode
    pub fn format_explain_query(command: &str, context: Option<&str>) -> String {
        if let Some(ctx) = context {
            format!(
                r#"Command to explain: {}

Context: {}

Break down this command and explain each part."#,
                command, ctx
            )
        } else {
            format!(
                r#"Command to explain: {}

Break down this command and explain each part."#,
                command
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_prompt() {
        let sys_info = SystemInfo {
            os: "macOS".to_string(),
            shell: "zsh".to_string(),
            current_dir: Some("/home/user".to_string()),
        };

        let prompt = SystemPrompt::Command.get_prompt(&sys_info, "en");
        assert!(prompt.contains("terminal command expert assistant"));
        assert!(prompt.contains("macOS"));
        assert!(prompt.contains("zsh"));
    }

    #[test]
    fn test_explain_prompt() {
        let sys_info = SystemInfo {
            os: "Linux".to_string(),
            shell: "bash".to_string(),
            current_dir: Some("/home/user".to_string()),
        };

        let prompt = SystemPrompt::Explain.get_prompt(&sys_info, "en");
        assert!(prompt.contains("terminal command explainer expert"));
    }

    #[test]
    fn test_format_command_query() {
        let query = SystemPrompt::format_command_query("how to list files", None);
        assert!(query.contains("how to list files"));
        assert!(query.contains("Provide the command"));
    }

    #[test]
    fn test_format_command_query_with_context() {
        let query = SystemPrompt::format_command_query(
            "how to list files",
            Some("in a directory with many files"),
        );
        assert!(query.contains("how to list files"));
        assert!(query.contains("in a directory with many files"));
    }
}
