/// Safety module for detecting dangerous commands
use lazy_static::lazy_static;
use std::collections::HashSet;

lazy_static! {
    /// Set of dangerous command patterns
    static ref DANGEROUS_PATTERNS: HashSet<&'static str> = {
        let mut patterns = HashSet::new();

        // Filesystem destructive commands
        patterns.insert("rm -rf /");
        patterns.insert("rm -rf /*");
        patterns.insert("rm -rf $HOME");
        patterns.insert("rm -rf ~");
        patterns.insert("rm -rf .*");

        // System destructive commands
        patterns.insert("dd if=");
        patterns.insert("mkfs");
        patterns.insert("fdisk");
        patterns.insert("gdisk");
        patterns.insert("wipefs");

        // Privilege escalation
        patterns.insert("chmod 777");
        patterns.insert("chmod -R 777");

        // System shutdown/reboot
        patterns.insert("shutdown -h");
        patterns.insert("shutdown now");
        patterns.insert("reboot");
        patterns.insert("poweroff");
        patterns.insert("halt");

        // Network operations
        patterns.insert("iptables -F");
        patterns.insert("ufw --force");

        // Recursive operations with wildcards
        patterns.insert("-exec rm");
        patterns.insert("-delete");

        patterns
    };
}

/// Check if a command is potentially dangerous
pub fn is_dangerous_command(command: &str) -> bool {
    let cmd_lower = command.to_lowercase();

    // Check against dangerous patterns
    for pattern in DANGEROUS_PATTERNS.iter() {
        if cmd_lower.contains(pattern) {
            return true;
        }
    }

    // Additional heuristics
    let cmd_trimmed = cmd_lower.trim();

    // Check for destructive operations with sudo
    if cmd_trimmed.starts_with("sudo") && cmd_trimmed.contains("rm -rf") {
        return true;
    }

    // Check for overwriting system files
    if cmd_trimmed.starts_with('>')
        && (cmd_trimmed.contains("/etc/")
            || cmd_trimmed.contains("/boot/")
            || cmd_trimmed.contains("/sys/"))
    {
        return true;
    }

    false
}

/// Get warning message for dangerous command
pub fn get_dangerous_command_warning(command: &str) -> String {
    let cmd_lower = command.to_lowercase();

    if cmd_lower.contains("rm -rf /") {
        return "This command will DELETE EVERYTHING on your system!".to_string();
    }

    if cmd_lower.contains("dd if=") {
        return "This command can overwrite disks and destroy data!".to_string();
    }

    if cmd_lower.contains("shutdown") || cmd_lower.contains("reboot") {
        return "This will shut down or reboot the system!".to_string();
    }

    if cmd_lower.contains("chmod 777") {
        return "This removes security restrictions on files/directories!".to_string();
    }

    if cmd_lower.starts_with("sudo") && cmd_lower.contains("rm -rf") {
        return "This command with sudo can delete system files!".to_string();
    }

    "This command may be destructive or dangerous!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dangerous_commands() {
        assert!(is_dangerous_command("rm -rf /"));
        assert!(is_dangerous_command("rm -rf /*"));
        assert!(is_dangerous_command("sudo rm -rf /important"));
        assert!(is_dangerous_command("dd if=/dev/zero of=/dev/sda"));
        assert!(is_dangerous_command("mkfs.ext4 /dev/sdb"));
        assert!(is_dangerous_command("shutdown -h now"));
        assert!(is_dangerous_command("chmod 777 /etc/passwd"));
    }

    #[test]
    fn test_safe_commands() {
        assert!(!is_dangerous_command("ls -la"));
        assert!(!is_dangerous_command("cd /home"));
        assert!(!is_dangerous_command("grep pattern file.txt"));
        assert!(!is_dangerous_command("cat /etc/hosts"));
    }

    #[test]
    fn test_warning_messages() {
        assert!(get_dangerous_command_warning("rm -rf /").contains("DELETE EVERYTHING"));
        assert!(
            get_dangerous_command_warning("dd if=/dev/zero of=/dev/sda")
                .contains("overwrite disks")
        );
        assert!(get_dangerous_command_warning("shutdown -h now").contains("shut down"));
    }
}
