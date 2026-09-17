use shadow_cli::config::ProjectConfig;
use shadow_cli::runner::AgentRunner;
use shadow_core::Result;
use shadow_cow::{OverlayManager, PatchGenerator};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_auto_approve_flag_injection() {
    let config = ProjectConfig::default();

    // 1. Claude Code auto-approve injection
    let claude_cmd = AgentRunner::prepare_agent_invocation(
        "claude",
        "Fix type errors in auth.ts",
        &["--verbose".to_string()],
        &config,
    );

    assert_eq!(claude_cmd.cmd, "claude");
    assert!(claude_cmd
        .args
        .contains(&"--dangerously-skip-permissions".to_string()));
    assert!(claude_cmd.args.contains(&"-p".to_string()));
    assert!(claude_cmd
        .args
        .contains(&"Fix type errors in auth.ts".to_string()));
    assert!(claude_cmd.args.contains(&"--verbose".to_string()));

    // 2. Synthetic credentials & noninteractive environment verification
    assert_eq!(
        claude_cmd.env.get("SANDBOX").map(String::as_str),
        Some("1")
    );
    assert_eq!(claude_cmd.env.get("CI").map(String::as_str), Some("1"));
    assert_eq!(
        claude_cmd.env.get("NONINTERACTIVE").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        claude_cmd.env.get("GIT_AUTHOR_NAME").map(String::as_str),
        Some("ShadowOS Agent")
    );
    assert_eq!(
        claude_cmd.env.get("GITHUB_TOKEN").map(String::as_str),
        Some("ghp_mock_shadowos_synthetic_token")
    );

    // 3. Aider auto-approve injection
    let aider_cmd = AgentRunner::prepare_agent_invocation(
        "aider",
        "Add login handler",
        &[],
        &config,
    );
    assert_eq!(aider_cmd.cmd, "aider");
    assert!(aider_cmd.args.contains(&"--yes".to_string()));
    assert!(aider_cmd.args.contains(&"--no-auto-commits".to_string()));

    // 4. swe-agent auto-approve injection
    let swe_cmd = AgentRunner::prepare_agent_invocation(
        "swe-agent",
        "Resolve issue #42",
        &[],
        &config,
    );
    assert!(swe_cmd.args.contains(&"-y".to_string()));
}

#[test]
fn test_sandbox_initialization_and_unattended_execution() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_repo = temp_root.path().join("my_repo");
    fs::create_dir_all(host_repo.join("src"))?;

    let mut index_file = File::create(host_repo.join("src/index.ts"))?;
    index_file.write_all(b"export const original = true;\n")?;

    let initial_host_hash = OverlayManager::compute_directory_sha256(&host_repo)?;

    // Initialize sandbox harness
    let context = AgentRunner::initialize_sandbox(&host_repo, None)?;
    assert!(context.upper_dir.exists());
    assert!(context.work_dir.exists());

    // Capture baseline in /dev/shm
    let (mem_path, state_path) =
        AgentRunner::capture_baseline_checkpoint(&context, "test_baseline")?;
    assert!(mem_path.exists());
    assert!(state_path.exists());

    // Prepare and execute unattended command
    let config = ProjectConfig::default();
    let cmd = AgentRunner::prepare_agent_invocation(
        "claude",
        "Refactor index.ts to v2",
        &[],
        &config,
    );

    let summary = AgentRunner::execute_unattended_command(&context, &cmd)?;
    assert_eq!(summary.exit_code, 0);
    assert!(!summary.host_pollution_detected);

    // Verify host repository remains 100% bit-identical
    let post_host_hash = OverlayManager::compute_directory_sha256(&host_repo)?;
    assert_eq!(initial_host_hash, post_host_hash);

    Ok(())
}

#[test]
fn test_diff_rollback_and_promote_workflow() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_repo = temp_root.path().join("prod_app");
    fs::create_dir_all(&host_repo)?;

    let host_app = host_repo.join("app.py");
    fs::write(&host_app, "def run():\n    return 'v1'\n")?;
    let initial_hash = OverlayManager::compute_directory_sha256(&host_repo)?;

    let context = AgentRunner::initialize_sandbox(&host_repo, None)?;

    // 1. Simulate agent modifying app.py inside ephemeral upperdir
    let upper_app = context.upper_dir.join("app.py");
    fs::write(&upper_app, "def run():\n    return 'v2-upgraded'\n")?;

    // Diff inspection
    let changes = context.overlay_manager.scan_upperdir_changes()?;
    assert_eq!(changes.len(), 1);

    let diff = PatchGenerator::generate_unified_diff(&host_app, &upper_app, std::path::Path::new("app.py"))?;
    assert!(diff.contains("-    return 'v1'"));
    assert!(diff.contains("+    return 'v2-upgraded'"));

    // 2. Test Instant Rollback (<100ms)
    context.overlay_manager.reset_upperdir()?;
    assert_eq!(context.overlay_manager.scan_upperdir_changes()?.len(), 0);
    assert_eq!(
        initial_hash,
        OverlayManager::compute_directory_sha256(&host_repo)?
    );

    // 3. Simulate another modification and promote
    fs::write(&upper_app, "def run():\n    return 'v3-promoted'\n")?;
    PatchGenerator::promote_file(&upper_app, &host_app)?;

    let promoted_content = fs::read_to_string(&host_app)?;
    assert_eq!(promoted_content, "def run():\n    return 'v3-promoted'\n");

    let updated_host_hash = OverlayManager::compute_directory_sha256(&host_repo)?;
    assert_ne!(initial_hash, updated_host_hash);

    Ok(())
}
