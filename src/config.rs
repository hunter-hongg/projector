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
    pub max_depth: u32,
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
                max_depth: 1,
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
            "scan.max_depth" => {
                self.scan.max_depth = value.parse()
                    .map_err(|_| anyhow::anyhow!("max_depth must be a number"))?
            }
            "report.stale_threshold_days" => {
                self.report.stale_threshold_days = value.parse()
                    .map_err(|_| anyhow::anyhow!("stale_threshold_days must be a number"))?
            }
            _ => anyhow::bail!("Unknown config key: {}", key),
        }
        Ok(())
    }
}

pub fn snapshot_dir() -> PathBuf {
    let dir = projector_dir().join("snapshots");
    let _ = std::fs::create_dir_all(&dir);
    dir
}
