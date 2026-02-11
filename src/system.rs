/// System information for context enrichment
#[derive(Debug, Clone)]
pub struct SystemInfo {
    /// Operating system name
    pub os: String,
    /// Shell name (bash, zsh, fish, etc.)
    pub shell: String,
    /// Current working directory (optional)
    pub current_dir: Option<String>,
}

impl SystemInfo {
    /// Detect current system information
    pub fn detect() -> Self {
        let os = Self::detect_os();
        let shell = Self::detect_shell();
        let current_dir = std::env::current_dir()
            .ok()
            .and_then(|path| path.to_str().map(|s| s.to_string()));

        Self {
            os,
            shell,
            current_dir,
        }
    }

    /// Detect the operating system
    fn detect_os() -> String {
        if cfg!(target_os = "macos") {
            return "macOS".to_string();
        }
        if cfg!(target_os = "linux") {
            return "Linux".to_string();
        }
        if cfg!(target_os = "windows") {
            return "Windows".to_string();
        }

        // Try to get more specific OS info
        if let Ok(output) = std::process::Command::new("uname").arg("-s").output() {
            if output.status.success() {
                if let Ok(os_str) = String::from_utf8(output.stdout) {
                    return os_str.trim().to_string();
                }
            }
        }

        // Default fallback
        "Unknown OS".to_string()
    }

    /// Detect the current shell
    fn detect_shell() -> String {
        // Try SHELL environment variable first
        if let Ok(shell_path) = std::env::var("SHELL") {
            // Extract shell name from path
            if let Some(shell_name) = shell_path.rsplit('/').next() {
                return shell_name.to_string();
            }
        }

        // Try to detect parent process
        if let Ok(ppid) = std::process::id().to_string().parse::<u32>() {
            #[cfg(unix)]
            {
                use std::fs;
                let stat_path = format!("/proc/{}/comm", ppid);
                if let Ok(comm) = fs::read_to_string(&stat_path) {
                    let shell_name = comm.trim();
                    if ["bash", "zsh", "fish", "sh", "ksh", "csh", "tcsh"].contains(&shell_name) {
                        return shell_name.to_string();
                    }
                }
            }
        }

        // Default shell
        "bash".to_string()
    }

    /// Get the current working directory
    pub fn _current_dir(&self) -> Option<&str> {
        self.current_dir.as_deref()
    }

    /// Format system context for inclusion in prompts
    pub fn format_context(&self) -> String {
        let mut context = format!("OS: {}", self.os);
        context.push_str(&format!("\nShell: {}", self.shell));

        if let Some(dir) = &self.current_dir {
            context.push_str(&format!("\nCurrent directory: {}", dir));
        }

        context
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_info_creation() {
        let sys_info = SystemInfo {
            os: "macOS".to_string(),
            shell: "zsh".to_string(),
            current_dir: Some("/home/user".to_string()),
        };

        assert_eq!(sys_info.os, "macOS");
        assert_eq!(sys_info.shell, "zsh");
        assert_eq!(sys_info._current_dir(), Some("/home/user"));
    }

    #[test]
    fn test_format_context() {
        let sys_info = SystemInfo {
            os: "Linux".to_string(),
            shell: "bash".to_string(),
            current_dir: Some("/var/www".to_string()),
        };

        let context = sys_info.format_context();
        assert!(context.contains("OS: Linux"));
        assert!(context.contains("Shell: bash"));
        assert!(context.contains("Current directory: /var/www"));
    }

    #[test]
    fn test_format_context_without_dir() {
        let sys_info = SystemInfo {
            os: "Windows".to_string(),
            shell: "powershell".to_string(),
            current_dir: None,
        };

        let context = sys_info.format_context();
        assert!(context.contains("OS: Windows"));
        assert!(context.contains("Shell: powershell"));
        assert!(!context.contains("Current directory:"));
    }
}
