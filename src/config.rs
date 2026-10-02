use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub scan: ScanConfig,
    pub report: ReportConfig,
    #[serde(default)]
    pub snapshot: SnapshotConfig,
    #[serde(default)]
    pub alert: AlertConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    pub default_path: String,
    /// How many levels below the scan root to look for git repositories.
    /// `1` is the legacy one-level behaviour, `0` means unlimited.
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

fn default_max_depth() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportConfig {
    pub stale_threshold_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotConfig {
    pub keep_count: u32,
}

impl Default for SnapshotConfig {
    fn default() -> Self {
        Self { keep_count: 30 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    pub health_threshold: u32,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            health_threshold: 40,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            scan: ScanConfig {
                default_path: ".".to_string(),
                max_depth: default_max_depth(),
            },
            report: ReportConfig {
                stale_threshold_days: 90,
            },
            snapshot: SnapshotConfig { keep_count: 30 },
            alert: AlertConfig {
                health_threshold: 40,
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
            "scan.max_depth" => {
                self.scan.max_depth = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("max_depth must be a number"))?;
            }
            "report.stale_threshold_days" => {
                self.report.stale_threshold_days = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("stale_threshold_days must be a number"))?
            }
            "snapshot.keep_count" => {
                self.snapshot.keep_count = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("keep_count must be a number"))?
            }
            "alert.health_threshold" => {
                self.alert.health_threshold = value
                    .parse()
                    .map_err(|_| anyhow::anyhow!("health_threshold must be a number"))?
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
        assert_eq!(config.scan.max_depth, 1);
        assert_eq!(config.report.stale_threshold_days, 90);
        assert_eq!(config.snapshot.keep_count, 30);
        assert_eq!(config.alert.health_threshold, 40);
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
    fn test_config_set_keep_count() {
        let mut config = Config::default();
        config.set("snapshot.keep_count", "50").unwrap();
        assert_eq!(config.snapshot.keep_count, 50);
    }

    #[test]
    fn test_config_set_health_threshold() {
        let mut config = Config::default();
        config.set("alert.health_threshold", "20").unwrap();
        assert_eq!(config.alert.health_threshold, 20);
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
    fn test_config_set_max_depth() {
        let mut config = Config::default();
        config.set("scan.max_depth", "3").unwrap();
        assert_eq!(config.scan.max_depth, 3);
    }

    #[test]
    fn test_config_set_max_depth_zero_unlimited() {
        let mut config = Config::default();
        config.set("scan.max_depth", "0").unwrap();
        assert_eq!(config.scan.max_depth, 0);
    }

    #[test]
    fn test_config_set_max_depth_non_number() {
        let mut config = Config::default();
        assert!(config.set("scan.max_depth", "deep").is_err());
    }

    #[test]
    fn test_config_toml_without_max_depth_still_loads() {
        let toml_str = r#"[scan]
default_path = "."

[report]
stale_threshold_days = 90
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.scan.max_depth, 1);
    }

    #[test]
    fn test_path_contains_home() {
        let path = Config::path();
        let home = std::env::var("HOME").unwrap();
        assert!(path.to_string_lossy().contains(&home));
        assert!(path.to_string_lossy().ends_with(".projector/config.toml"));
    }
}
