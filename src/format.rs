use std::fmt;

/// Output formats accepted by the `-f, --format` flag.
///
/// Every subcommand validates its `-f` value through
/// [`OutputFormat::parse`], which reports unsupported formats with the error
/// a user already knows: `Unsupported format: 'x'. Use 'json' or 'md'.`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable terminal table (the default when `-f` is omitted).
    Terminal,
    /// JSON output for script consumption.
    Json,
    /// Markdown output (currently only `report`).
    Markdown,
}

impl OutputFormat {
    /// Parse a `-f` value into a format the subcommand accepts.
    ///
    /// `None` means the flag was omitted: always the terminal default.
    /// `accepted` lists the non-terminal formats this subcommand supports.
    /// An unsupported value is an error, so typos like `deps -f jsson`
    /// fail loudly instead of silently falling back to the terminal table.
    pub fn parse(value: Option<&str>, accepted: &[OutputFormat]) -> Result<Self, anyhow::Error> {
        match value.map(str::trim).filter(|s| !s.is_empty()) {
            None => Ok(Self::Terminal),
            Some("json") if accepted.contains(&Self::Json) => Ok(Self::Json),
            Some("md") | Some("markdown") if accepted.contains(&Self::Markdown) => {
                Ok(Self::Markdown)
            }
            Some(name) => {
                let list: Vec<String> = std::iter::once("'terminal'".to_string())
                    .chain(accepted.iter().map(|f| match f {
                        Self::Json => "'json'".to_string(),
                        Self::Markdown => "'md'".to_string(),
                        Self::Terminal => "'terminal'".to_string(),
                    }))
                    .collect();
                anyhow::bail!("Unsupported format: '{}'. Use {}.", name, list.join(" or "));
            }
        }
    }

    pub fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }

    pub fn is_markdown(self) -> bool {
        matches!(self, Self::Markdown)
    }

    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Terminal)
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Terminal => "terminal",
            Self::Json => "json",
            Self::Markdown => "md",
        };
        f.write_str(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON_ONLY: &[OutputFormat] = &[OutputFormat::Json];
    const JSON_OR_MD: &[OutputFormat] = &[OutputFormat::Json, OutputFormat::Markdown];

    #[test]
    fn omitted_defaults_to_terminal() {
        assert_eq!(
            OutputFormat::parse(None, JSON_ONLY).unwrap(),
            OutputFormat::Terminal
        );
    }

    #[test]
    fn empty_string_defaults_to_terminal() {
        assert_eq!(
            OutputFormat::parse(Some(""), JSON_ONLY).unwrap(),
            OutputFormat::Terminal
        );
    }

    #[test]
    fn json_is_accepted_when_supported() {
        assert_eq!(
            OutputFormat::parse(Some("json"), JSON_ONLY).unwrap(),
            OutputFormat::Json
        );
        assert!(
            OutputFormat::parse(Some("json"), JSON_ONLY)
                .unwrap()
                .is_json()
        );
    }

    #[test]
    fn md_is_accepted_by_report_shape() {
        assert_eq!(
            OutputFormat::parse(Some("md"), JSON_OR_MD).unwrap(),
            OutputFormat::Markdown
        );
        assert_eq!(
            OutputFormat::parse(Some("markdown"), JSON_OR_MD).unwrap(),
            OutputFormat::Markdown
        );
    }

    #[test]
    fn json_only_commands_reject_md_with_known_error() {
        let err = OutputFormat::parse(Some("md"), JSON_ONLY).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Unsupported format: 'md'. Use 'terminal' or 'json'."
        );
    }

    #[test]
    fn unknown_format_lists_accepted_values() {
        let err = OutputFormat::parse(Some("yaml"), JSON_ONLY).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Unsupported format: 'yaml'. Use 'terminal' or 'json'."
        );
        let err = OutputFormat::parse(Some("csv"), JSON_OR_MD).unwrap_err();
        assert_eq!(
            err.to_string(),
            "Unsupported format: 'csv'. Use 'terminal' or 'json' or 'md'."
        );
    }

    #[test]
    fn is_json_only_true_for_json() {
        assert!(!OutputFormat::Terminal.is_json());
        assert!(OutputFormat::Json.is_json());
        assert!(!OutputFormat::Markdown.is_json());
    }
}
