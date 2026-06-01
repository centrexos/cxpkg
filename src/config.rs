use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{CxpkgError, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub backends: BackendConfig,
    pub resolver: ResolverConfig,
    pub cache: CacheConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    pub apt_enabled: bool,
    pub dnf_enabled: bool,
    pub flatpak_enabled: bool,
    pub priority: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolverConfig {
    pub allow_downgrades: bool,
    pub auto_remove_orphans: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    pub dir: PathBuf,
    pub max_age_days: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            backends: BackendConfig {
                apt_enabled: true,
                dnf_enabled: false,
                flatpak_enabled: true,
                priority: vec![
                    "apt".into(),
                    "flatpak".into(),
                    "dnf".into(),
                ],
            },
            resolver: ResolverConfig {
                allow_downgrades: false,
                auto_remove_orphans: false,
            },
            cache: CacheConfig {
                dir: PathBuf::from("/var/cache/cxpkg"),
                max_age_days: 7,
            },
        }
    }
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .map_err(|e| CxpkgError::Config(e.to_string()))?;
            toml::from_str(&content)
                .map_err(|e| CxpkgError::Config(e.to_string()))
        } else {
            Ok(Self::default())
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)
            .map_err(|e| CxpkgError::Config(e.to_string()))?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    PathBuf::from("/etc/cxpkg/config.toml")
}
