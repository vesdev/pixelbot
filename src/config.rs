use color_eyre::eyre::{Context, Result};
use serde::Deserialize;
use std::path::Path;

#[derive(Deserialize)]
pub struct Config {
    pub token: String,
    pub guild_id: Option<u64>,
    pub member_role_id: u64,
    pub elder_role_id: u64,
    pub db_path: Option<String>,
    pub polling_channel_id: u64,
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Could not read config file: {}", path.display()))?;
        toml::from_str(&raw).wrap_err("Invalid config format")
    }
}
