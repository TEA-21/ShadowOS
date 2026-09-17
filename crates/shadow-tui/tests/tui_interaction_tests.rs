use crossterm::event::KeyCode;
use shadow_core::Result;
use shadow_cow::{OverlayConfig, OverlayManager};
use shadow_tui::{ActivePane, DiffApp, TuiAction};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

#[test]
fn test_tui_keyboard_navigation_and_staging() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged");

    fs::create_dir_all(host_dir.join("src"))?;
    fs::create_dir_all(upper_dir.join("src"))?;

    let host_file = host_dir.join("src/service.rs");
    let upper_file = upper_dir.join("src/service.rs");

    fs::write(
        &host_file,
        "fn run() {\n    println!(\"v1\");\n}\n\nfn helper() {}\n",
    )?;
    fs::write(
        &upper_file,
        "fn run() {\n    println!(\"v2-upgraded\");\n}\n\nfn helper() {}\n",
    )?;

    let overlay_cfg = OverlayConfig::new(host_dir.clone(), upper_dir.clone(), work_dir, merged_dir);
    let overlay_manager = OverlayManager::new(overlay_cfg);
    overlay_manager.init_ephemeral_layers()?;

    let changes = overlay_manager.scan_upperdir_changes()?;
    assert_eq!(changes.len(), 1);

    let mut app = DiffApp::new(host_dir, upper_dir, changes);
    assert_eq!(app.files.len(), 1);
    assert!(!app.files[0].is_staged);

    // 1. Simulate [Space]: Stage whole file
    let act = app.handle_key(KeyCode::Char(' '));
    assert_eq!(act, TuiAction::ToggleStage);
    assert!(app.files[0].is_staged);
    assert!(app.files[0].hunks.iter().all(|h| h.is_staged));

    // 2. Simulate [Tab]: Switch focus to Hunk pane
    let act = app.handle_key(KeyCode::Tab);
    assert_eq!(act, TuiAction::None);
    assert_eq!(app.active_pane, ActivePane::HunkList);

    // 3. Simulate [j] / Down arrow in Hunk pane
    let act = app.handle_key(KeyCode::Char('j'));
    assert_eq!(act, TuiAction::None);

    // 4. Simulate [Space] in Hunk pane: Toggle single hunk staging
    let act = app.handle_key(KeyCode::Char(' '));
    assert_eq!(act, TuiAction::ToggleStage);
    assert!(!app.files[0].hunks[0].is_staged);

    // 5. Simulate [Tab]: Switch back to File pane
    app.handle_key(KeyCode::Tab);
    assert_eq!(app.active_pane, ActivePane::FileList);

    Ok(())
}

#[test]
fn test_tui_promote_keyboard_action() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged");

    fs::create_dir_all(host_dir.join("src"))?;
    fs::create_dir_all(upper_dir.join("src"))?;

    let host_calc = host_dir.join("src/calc.rs");
    let upper_calc = upper_dir.join("src/calc.rs");
    let upper_extra = upper_dir.join("src/extra.rs");

    fs::write(&host_calc, "pub fn calculate() -> i32 { 10 }\n")?;
    fs::write(&upper_calc, "pub fn calculate() -> i32 { 42 }\n")?;
    fs::write(&upper_extra, "pub fn extra() {}\n")?;

    let initial_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;

    let overlay_cfg = OverlayConfig::new(host_dir.clone(), upper_dir.clone(), work_dir, merged_dir);
    let overlay_manager = OverlayManager::new(overlay_cfg);
    overlay_manager.init_ephemeral_layers()?;

    let changes = overlay_manager.scan_upperdir_changes()?;
    assert_eq!(changes.len(), 2);

    let mut app = DiffApp::new(host_dir.clone(), upper_dir, changes);

    // Select calc.rs and stage it
    let calc_idx = app
        .files
        .iter()
        .position(|f| f.relative_path == std::path::Path::new("src/calc.rs"))
        .unwrap();
    app.selected_file_index = calc_idx;

    // Simulate [Space] to stage calc.rs
    app.handle_key(KeyCode::Char(' '));
    assert!(app.files[calc_idx].is_staged);

    // Simulate [P]: Hotkey triggers PromoteSelected action
    let act = app.handle_key(KeyCode::Char('p'));
    assert_eq!(act, TuiAction::PromoteSelected);

    // Execute promotion
    let promoted_count = app.promote_staged()?;
    assert_eq!(promoted_count, 1);

    // Verify host repository updated
    let promoted_content = fs::read_to_string(&host_calc)?;
    assert_eq!(promoted_content, "pub fn calculate() -> i32 { 42 }\n");

    // Unstaged extra.rs must NOT exist on host
    assert!(!host_dir.join("src/extra.rs").exists());

    let updated_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;
    assert_ne!(initial_host_hash, updated_host_hash);

    Ok(())
}

#[test]
fn test_tui_rollback_keyboard_action() -> Result<()> {
    let temp_root = TempDir::new()?;
    let host_dir = temp_root.path().join("host_repo");
    let upper_dir = temp_root.path().join("ephemeral_upper");
    let work_dir = temp_root.path().join("workdir");
    let merged_dir = temp_root.path().join("merged");

    fs::create_dir_all(host_dir.join("src"))?;
    fs::create_dir_all(upper_dir.join("src"))?;

    let host_app = host_dir.join("src/app.rs");
    fs::write(&host_app, "pub fn pristine() -> bool { true }\n")?;
    let initial_host_hash = OverlayManager::compute_directory_sha256(&host_dir)?;

    // Destructive mutations in upperdir
    let upper_app = upper_dir.join("src/app.rs");
    let upper_corrupted = upper_dir.join("src/corrupted.rs");
    fs::write(&upper_app, "pub fn pristine() -> bool { false }\n")?;
    fs::write(&upper_corrupted, "syntax error!!\n")?;

    let overlay_cfg = OverlayConfig::new(host_dir.clone(), upper_dir.clone(), work_dir, merged_dir);
    let overlay_manager = OverlayManager::new(overlay_cfg);
    overlay_manager.init_ephemeral_layers()?;

    let changes = overlay_manager.scan_upperdir_changes()?;
    assert_eq!(changes.len(), 2);

    let mut app = DiffApp::new(host_dir.clone(), upper_dir, changes);

    // Simulate [R]: Hotkey triggers Rollback action
    let act = app.handle_key(KeyCode::Char('r'));
    assert_eq!(act, TuiAction::Rollback);

    // Execute rollback
    app.rollback_ephemeral(&overlay_manager)?;

    // Verify all ephemeral modifications are wiped (<100ms)
    assert_eq!(overlay_manager.scan_upperdir_changes()?.len(), 0);
    assert_eq!(app.files.len(), 0);

    // Host remains 100% bit-identical
    let post_rollback_hash = OverlayManager::compute_directory_sha256(&host_dir)?;
    assert_eq!(initial_host_hash, post_rollback_hash);

    // Simulate [Q]: Exits TUI
    let act = app.handle_key(KeyCode::Char('q'));
    assert_eq!(act, TuiAction::Quit);
    assert!(!app.is_running);

    Ok(())
}
