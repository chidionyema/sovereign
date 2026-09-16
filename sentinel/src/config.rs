use crate::guardrails::GuardrailPolicy;
use anyhow::Result;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub device_id: String,
    pub webhook_url: String,
    pub ledger_path: PathBuf,
    #[serde(default)]
    pub policy: GuardrailPolicy,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = dirs::config_dir()
            .ok_or_else(|| anyhow::anyhow!("no config dir"))?
            .join("sovereign/config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        let cfg = toml::from_str(&content)?;
        Ok(cfg)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            device_id: "dev-local".to_string(),
            webhook_url: "http://127.0.0.1:8000".to_string(),
            ledger_path: dirs::data_dir()
                .map(|d| d.join("sovereign/ledger.db"))
                .unwrap_or_else(|| PathBuf::from("ledger.db")),
            policy: GuardrailPolicy::default(),
        }
    }
}
