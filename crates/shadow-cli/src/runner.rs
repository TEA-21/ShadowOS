use shadow_core::{
    config::VmConfig,
    protocol::CommandRequest,
    Result,
};
use crate::config::ProjectConfig;
use std::path::Path;

pub struct AgentRunner;

impl AgentRunner {
    /// Builds the unattended CommandRequest for Claude Code or other autonomous agents
    pub fn prepare_claude_invocation(
        prompt: &str,
        project_config: &ProjectConfig,
    ) -> CommandRequest {
        let mut args = vec![
            "-p".to_string(),
            prompt.to_string(),
        ];

        // PRD Requirement FR-03: Pass auto-approval flag to eliminate permission stalls
        if project_config.auto_approve {
            args.push("--dangerously-skip-permissions".to_string());
        }

        CommandRequest {
            cmd: "claude".to_string(),
            args,
            env: project_config.environment.clone(),
            workdir: "/workspace".to_string(),
            timeout_seconds: 600,
        }
    }
}
