use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub auto_approve: bool,
    pub environment: HashMap<String, String>,
    pub synthetic_credentials: HashMap<String, String>,
    pub auto_approve_flags: HashMap<String, Vec<String>>,
    pub ignored_paths: Vec<String>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        let mut env = HashMap::new();
        env.insert("SANDBOX".to_string(), "1".to_string());
        env.insert("CI".to_string(), "1".to_string());
        env.insert("DEBIAN_FRONTEND".to_string(), "noninteractive".to_string());
        env.insert("NONINTERACTIVE".to_string(), "1".to_string());
        env.insert("PAGER".to_string(), "cat".to_string());

        let mut creds = HashMap::new();
        creds.insert("GIT_AUTHOR_NAME".to_string(), "ShadowOS Agent".to_string());
        creds.insert("GIT_AUTHOR_EMAIL".to_string(), "agent@shadowos.local".to_string());
        creds.insert("GIT_COMMITTER_NAME".to_string(), "ShadowOS Agent".to_string());
        creds.insert("GIT_COMMITTER_EMAIL".to_string(), "agent@shadowos.local".to_string());
        creds.insert("GITHUB_TOKEN".to_string(), "ghp_mock_shadowos_synthetic_token".to_string());

        let mut flags = HashMap::new();
        flags.insert(
            "claude".to_string(),
            vec!["--dangerously-skip-permissions".to_string()],
        );
        flags.insert(
            "aider".to_string(),
            vec!["--yes".to_string(), "--no-auto-commits".to_string()],
        );
        flags.insert("swe-agent".to_string(), vec!["-y".to_string()]);
        flags.insert(
            "antigravity".to_string(),
            vec!["--auto-approve".to_string(), "--non-interactive".to_string()],
        );

        Self {
            auto_approve: true,
            environment: env,
            synthetic_credentials: creds,
            auto_approve_flags: flags,
            ignored_paths: vec![
                ".git".to_string(),
                "target".to_string(),
                "node_modules".to_string(),
                ".shadow".to_string(),
            ],
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

    /// Resolves auto-approve flags for target agent (e.g. --dangerously-skip-permissions)
    pub fn resolve_flags_for_agent(&self, agent: &str) -> Vec<String> {
        if !self.auto_approve {
            return Vec::new();
        }

        let agent_lower = agent.to_lowercase();
        if let Some(flags) = self.auto_approve_flags.get(&agent_lower) {
            flags.clone()
        } else if agent_lower.contains("claude") {
            vec!["--dangerously-skip-permissions".to_string()]
        } else if agent_lower.contains("aider") {
            vec!["--yes".to_string(), "--no-auto-commits".to_string()]
        } else {
            vec!["-y".to_string()]
        }
    }

    /// Builds combined environment with noninteractive markers and synthetic credentials
    pub fn build_execution_env(&self) -> HashMap<String, String> {
        let mut combined = self.environment.clone();
        for (k, v) in &self.synthetic_credentials {
            combined.insert(k.clone(), v.clone());
        }
        combined
    }
}
