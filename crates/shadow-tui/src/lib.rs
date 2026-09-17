pub mod app;
pub mod ui;

pub use app::{ActivePane, DiffApp, DiffHunk, DiffLine, FileDiff, LineType, TuiAction};
pub use ui::render_diff_ui;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use shadow_core::Result;
use shadow_cow::OverlayManager;
use std::io;
use std::path::Path;

/// Launches the interactive terminal UI event loop
pub fn run_interactive_tui(
    host_root: &Path,
    upper_root: &Path,
    overlay_manager: &OverlayManager,
) -> Result<()> {
    let modified_files = overlay_manager.scan_upperdir_changes()?;
    let mut app = DiffApp::new(
        host_root.to_path_buf(),
        upper_root.to_path_buf(),
        modified_files,
    );

    // Setup crossterm raw mode
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    while app.is_running {
        terminal.draw(|f| ui::render_diff_ui(f, &app))?;

        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                let action = app.handle_key(key.code);
                match action {
                    TuiAction::PromoteSelected => {
                        let _ = app.promote_staged();
                    }
                    TuiAction::Rollback => {
                        let _ = app.rollback_ephemeral(overlay_manager);
                    }
                    TuiAction::Quit => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    // Restore terminal cleanly
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
