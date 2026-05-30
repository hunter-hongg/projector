use anyhow::Result;

use crate::color;
use crate::config::Config;

pub fn subcmd_config_show() -> Result<()> {
    let config = Config::load()?;
    println!("{}", color::info("Current configuration:"));
    println!();
    println!("  scan.default_path = {}", config.scan.default_path);
    println!(
        "  report.stale_threshold_days = {}",
        config.report.stale_threshold_days
    );
    println!(
        "  snapshot.keep_count = {}",
        config.snapshot.keep_count
    );
    println!(
        "  alert.health_threshold = {}",
        config.alert.health_threshold
    );
    println!();
    println!("Config file: {}", Config::path().display());
    Ok(())
}

pub fn subcmd_config_set(key: String, value: String) -> Result<()> {
    let mut config = Config::load()?;
    config.set(&key, &value)?;
    config.save()?;
    println!(
        "{}",
        color::info(format!("Set {} = {}", key, value).as_str())
    );
    Ok(())
}
