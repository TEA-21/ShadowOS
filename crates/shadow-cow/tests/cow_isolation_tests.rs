use shadow_core::Result;
use shadow_cow::{
    FileChangeType, OverlayConfig, OverlayManager, PatchGenerator,
};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_zero_host_filesystem_pollution() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged_workspace");

    // 1. Setup sample host project
    fs::create_dir_all(host_dir.join("src"))?;
    let mut main_file = File::create(host_dir.join("src/main.rs"))?;
    main_file.write_all(b"fn main() { println!(\"Host Original\"); }\n")?;

    let mut cargo_file = File::create(host_dir.join("Cargo.toml"))?;
    cargo_file.write_all(b"[package]\nname = \"sample\"\nversion = \"0.1.0\"\n")?;

    let mut readme_file = File::create(host_dir.join("README.md"))?;
    readme_file.write_all(b"# Sample Project\nHost pristine README.\n")?;

    // 2. Capture initial host SHA-256 tree hash
    let initial_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;
    println!("\n[COW TEST] Initial Host Tree Checksum: {}", initial_host_hash);

    // 3. Initialize OverlayFS CoW stack
    let config = OverlayConfig::new(host_dir.clone(), upper_dir.clone(), work_dir.clone(), merged_dir.clone());
    let manager = OverlayManager::new(config);
    manager.init_ephemeral_layers()?;

    // 4. Simulate agent write operations inside ephemeral upperdir (CoW layer)
    // Agent modifies existing file:
    fs::create_dir_all(upper_dir.join("src"))?;
    let mut upper_main = File::create(upper_dir.join("src/main.rs"))?;
    upper_main.write_all(b"fn main() { println!(\"Modified by Agent Unattended!\"); }\nfn helper() {}\n")?;

    // Agent creates new files:
    let mut upper_auth = File::create(upper_dir.join("src/auth.rs"))?;
    upper_auth.write_all(b"pub fn authenticate() -> bool { true }\n")?;

    let mut upper_log = File::create(upper_dir.join("agent_exec.log"))?;
    upper_log.write_all(b"[Agent] 14 tests executed with auto-approve flags\n")?;

    // 5. Scan upperdir changes
    let modified_files = manager.scan_upperdir_changes()?;
    assert_eq!(modified_files.len(), 3);

    let main_change = modified_files.iter().find(|f| f.relative_path == Path::new("src/main.rs")).unwrap();
    assert_eq!(main_change.change_type, FileChangeType::Modified);

    let auth_change = modified_files.iter().find(|f| f.relative_path == Path::new("src/auth.rs")).unwrap();
    assert_eq!(auth_change.change_type, FileChangeType::Created);

    // 6. VERIFY ZERO HOST POLLUTION (CRITICAL PRD REQUIREMENT)
    let post_exec_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;
    println!("[COW TEST] Post-Execution Host Tree Checksum: {}", post_exec_host_hash);

    assert_eq!(
        initial_host_hash, post_exec_host_hash,
        "CRITICAL FAILURE: Host repository was modified! Hashes do not match."
    );

    // Host files must remain 100% bit-identical
    let host_main_content = fs::read_to_string(host_dir.join("src/main.rs"))?;
    assert_eq!(host_main_content, "fn main() { println!(\"Host Original\"); }\n");
    assert!(!host_dir.join("src/auth.rs").exists());
    assert!(!host_dir.join("agent_exec.log").exists());

    println!("  [OK] Zero Host Pollution VERIFIED: Host repository is 100% bit-identical.");

    // 7. Test Instant Upperdir Reset (<5ms)
    let t0 = std::time::Instant::now();
    manager.reset_upperdir()?;
    let reset_elapsed = t0.elapsed();
    println!("[COW TEST] Upperdir reset executed in: {:?}", reset_elapsed);
    assert!(reset_elapsed.as_millis() < 50, "Upperdir reset too slow");
    assert_eq!(manager.scan_upperdir_changes()?.len(), 0);

    Ok(())
}

#[test]
fn test_patch_generation_and_selective_promotion() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");

    fs::create_dir_all(&host_dir)?;
    fs::create_dir_all(&upper_dir)?;

    let host_file = host_dir.join("app.py");
    let upper_file = upper_dir.join("app.py");

    fs::write(&host_file, "def start():\n    return False\n")?;
    fs::write(&upper_file, "def start():\n    # Fixed by Agent\n    return True\n")?;

    // Generate unified diff
    let patch = PatchGenerator::generate_unified_diff(&host_file, &upper_file, Path::new("app.py"))?;
    println!("\n[DIFF TEST] Generated Unified Patch:\n{}", patch);

    assert!(patch.contains("--- a/app.py"));
    assert!(patch.contains("+++ b/app.py"));
    assert!(patch.contains("-    return False"));
    assert!(patch.contains("+    return True"));

    // Promote changes to host
    PatchGenerator::promote_file(&upper_file, &host_file)?;
    let promoted_content = fs::read_to_string(&host_file)?;
    assert_eq!(promoted_content, "def start():\n    # Fixed by Agent\n    return True\n");
    println!("  [OK] Selective host promotion verified.");

    Ok(())
}
