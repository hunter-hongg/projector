use anyhow::Result;
use clap::Command;
use clap::CommandFactory;
use clap_complete::{generate, Shell};

use crate::command::{Projector, ShellKind};

pub fn subcmd_completion(shell: ShellKind) -> Result<()> {
    let shell = match shell {
        ShellKind::Bash => Shell::Bash,
        ShellKind::Zsh => Shell::Zsh,
        ShellKind::Fish => Shell::Fish,
    };

    let mut cmd: Command = Projector::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut std::io::stdout());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_bash_output() {
        let mut buf = Vec::new();
        let shell = Shell::Bash;
        let mut cmd: Command = Projector::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("_projector"));
    }

    #[test]
    fn test_completion_zsh_output() {
        let mut buf = Vec::new();
        let shell = Shell::Zsh;
        let mut cmd: Command = Projector::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_completion_fish_output() {
        let mut buf = Vec::new();
        let shell = Shell::Fish;
        let mut cmd: Command = Projector::command();
        let name = cmd.get_name().to_string();
        generate(shell, &mut cmd, name, &mut buf);
        let output = String::from_utf8(buf).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_completion_shell_kind_mapping() {
        assert!(subcmd_completion(ShellKind::Bash).is_ok());
        assert!(subcmd_completion(ShellKind::Zsh).is_ok());
        assert!(subcmd_completion(ShellKind::Fish).is_ok());
    }
}
