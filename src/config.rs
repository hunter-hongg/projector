use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub scan: ScanConfig,
    pub report: ReportConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub default_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub stale_threshold_days: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig {
                default_path: ".".to_string(),
            },
            report: ReportConfig {
                stale_threshold_days: 90,
            },
        }
    }
}

fn projector_dir() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join(".projector")
}

impl Config {
    pub fn path() -> PathBuf {
        projector_dir().join("config.toml")
    }

    pub fn load() -> Result<Self> {
        let path = Self::path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Config::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        match key {
            "scan.default_path" => self.scan.default_path = value.to_string(),
            "report.stale_threshold_days" => {
                self.report.stale_threshold_days = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("stale_threshold_days must be a number"))?
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }
}

pub fn snapshot_dir() -> PathBuf {
    projector_dir().join("snapshots")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.scan.default_path, ".");
        assert_eq!(config.report.stale_threshold_days, 90);
    }

    #[test]
    fn test_config_set_scan_path() {
        let mut config = Config::default();
        config.set("scan.default_path", "/tmp/projects").unwrap();
        assert_eq!(config.scan.default_path, "/tmp/projects");
    }

    #[test]
    fn test_config_set_stale_threshold() {
        let mut config = Config::default();
        config.set("report.stale_threshold_days", "30").unwrap();
        assert_eq!(config.report.stale_threshold_days, 30);
    }

    #[test]
    fn test_config_set_invalid_key() {
        let mut config = Config::default();
        let result = config.set("foo.bar", "value");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unknown config key")
        );
    }

    #[test]
    fn test_config_set_stale_threshold_non_number() {
        let mut config = Config::default();
        let result = config.set("report.stale_threshold_days", "not_a_number");
        assert!(result.is_err());
    }

    #[test]
    fn test_path_contains_home() {
        let path = Config::path();
        let home = std::env::var("HOME").unwrap();
        assert!(path.to_string_lossy().contains(&home));
        assert!(path.to_string_lossy().ends_with(".projector/config.toml"));
    }
}
