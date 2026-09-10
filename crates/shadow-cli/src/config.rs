use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use shadow_core::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub auto_approve: bool,
    pub environment: HashMap<String, String>,
    pub ignored_paths: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let mut env = HashMap::new();
        env.insert("SANDBOX".to_string(), "1".to_string());
        env.insert("NODE_ENV".to_string(), "test".to_string());

        Self {
            auto_approve: true,
            environment: env,
            ignored_paths: vec![".git".to_string(), "target".to_string(), "node_modules".to_string()],
        }
    }
}

impl ProjectConfig {
    pub fn load_from_dir(dir: &Path) -> Self {
        let cfg_path = dir.join(".shadow").join("config.json");
        if cfg_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&cfg_path) {
                if let Ok(cfg) = serde_json::from_str::<ProjectConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }
}
