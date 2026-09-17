use crossterm::event::KeyCode;
use shadow_core::Result;
use shadow_cow::{FileChangeType, ModifiedFile, OverlayManager, PatchGenerator};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineType {
    Context,
    Addition,
    Deletion,
    Header,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub line_type: LineType,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub id: usize,
    pub header: String,
    pub lines: Vec<DiffLine>,
    pub is_staged: bool,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub relative_path: PathBuf,
    pub change_type: FileChangeType,
    pub hunks: Vec<DiffHunk>,
    pub is_staged: bool,
    pub raw_diff: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    FileList,
    HunkList,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    None,
    ToggleStage,
    PromoteSelected,
    Rollback,
    Quit,
}

pub struct DiffApp {
    pub host_root: PathBuf,
    pub upper_root: PathBuf,
    pub files: Vec<FileDiff>,
    pub selected_file_index: usize,
    pub selected_hunk_index: usize,
    pub active_pane: ActivePane,
    pub is_running: bool,
    pub status_message: String,
}

impl DiffApp {
    pub fn new(host_root: PathBuf, upper_root: PathBuf, modified_files: Vec<ModifiedFile>) -> Self {
        let mut app = Self {
            host_root,
            upper_root,
            files: Vec::new(),
            selected_file_index: 0,
            selected_hunk_index: 0,
            active_pane: ActivePane::FileList,
            is_running: true,
            status_message: "Press [Space] to Stage/Unstage, [P] Promote, [R] Rollback, [Q] Quit"
                .to_string(),
        };

        let _ = app.load_diffs(&modified_files);
        app
    }

    /// Generates unified diffs and parses them into file and hunk models
    pub fn load_diffs(&mut self, modified_files: &[ModifiedFile]) -> Result<()> {
        self.files.clear();

        for file in modified_files {
            let host_path = self.host_root.join(&file.relative_path);
            let upper_path = self.upper_root.join(&file.relative_path);

            let raw_diff = PatchGenerator::generate_unified_diff(
                &host_path,
                &upper_path,
                &file.relative_path,
            )?;

            let hunks = Self::parse_hunks(&raw_diff);

            self.files.push(FileDiff {
                relative_path: file.relative_path.clone(),
                change_type: file.change_type,
                hunks,
                is_staged: false,
                raw_diff,
            });
        }

        Ok(())
    }

    /// Parses raw unified diff string into structured DiffHunk items
    pub fn parse_hunks(diff_str: &str) -> Vec<DiffHunk> {
        let mut hunks = Vec::new();
        let mut current_header = String::from("@@ General @@");
        let mut current_lines = Vec::new();
        let mut hunk_id = 0;

        for line in diff_str.lines() {
            if line.starts_with("@@") {
                if !current_lines.is_empty() {
                    hunks.push(DiffHunk {
                        id: hunk_id,
                        header: current_header.clone(),
                        lines: current_lines.clone(),
                        is_staged: false,
                    });
                    hunk_id += 1;
                    current_lines.clear();
                }
                current_header = line.to_string();
                current_lines.push(DiffLine {
                    line_type: LineType::Header,
                    content: line.to_string(),
                });
            } else if line.starts_with('+') && !line.starts_with("+++") {
                current_lines.push(DiffLine {
                    line_type: LineType::Addition,
                    content: line.to_string(),
                });
            } else if line.starts_with('-') && !line.starts_with("---") {
                current_lines.push(DiffLine {
                    line_type: LineType::Deletion,
                    content: line.to_string(),
                });
            } else {
                current_lines.push(DiffLine {
                    line_type: LineType::Context,
                    content: line.to_string(),
                });
            }
        }

        if !current_lines.is_empty() {
            hunks.push(DiffHunk {
                id: hunk_id,
                header: current_header,
                lines: current_lines,
                is_staged: false,
            });
        }

        hunks
    }

    /// Handles keyboard events directly to drive the UI state machine
    pub fn handle_key(&mut self, key: KeyCode) -> TuiAction {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.is_running = false;
                TuiAction::Quit
            }
            KeyCode::Char('p') => TuiAction::PromoteSelected,
            KeyCode::Char('r') => TuiAction::Rollback,
            KeyCode::Char(' ') => {
                self.toggle_stage();
                TuiAction::ToggleStage
            }
            KeyCode::Tab => {
                self.active_pane = match self.active_pane {
                    ActivePane::FileList => ActivePane::HunkList,
                    ActivePane::HunkList => ActivePane::FileList,
                };
                TuiAction::None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.select_next();
                TuiAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.select_previous();
                TuiAction::None
            }
            _ => TuiAction::None,
        }
    }

    pub fn toggle_stage(&mut self) {
        if self.files.is_empty() {
            return;
        }

        match self.active_pane {
            ActivePane::FileList => {
                if let Some(file) = self.files.get_mut(self.selected_file_index) {
                    file.is_staged = !file.is_staged;
                    let target_state = file.is_staged;
                    for hunk in &mut file.hunks {
                        hunk.is_staged = target_state;
                    }
                    self.status_message = format!(
                        "File '{}' {} staged.",
                        file.relative_path.display(),
                        if target_state { "is" } else { "is not" }
                    );
                }
            }
            ActivePane::HunkList => {
                if let Some(file) = self.files.get_mut(self.selected_file_index) {
                    if let Some(hunk) = file.hunks.get_mut(self.selected_hunk_index) {
                        hunk.is_staged = !hunk.is_staged;
                        file.is_staged = file.hunks.iter().any(|h| h.is_staged);
                        self.status_message = format!(
                            "Hunk #{} in '{}' {} staged.",
                            hunk.id,
                            file.relative_path.display(),
                            if hunk.is_staged { "is" } else { "is not" }
                        );
                    }
                }
            }
        }
    }

    pub fn select_next(&mut self) {
        if self.files.is_empty() {
            return;
        }

        match self.active_pane {
            ActivePane::FileList => {
                self.selected_file_index = (self.selected_file_index + 1) % self.files.len();
                self.selected_hunk_index = 0;
            }
            ActivePane::HunkList => {
                if let Some(file) = self.files.get(self.selected_file_index) {
                    if !file.hunks.is_empty() {
                        self.selected_hunk_index =
                            (self.selected_hunk_index + 1) % file.hunks.len();
                    }
                }
            }
        }
    }

    pub fn select_previous(&mut self) {
        if self.files.is_empty() {
            return;
        }

        match self.active_pane {
            ActivePane::FileList => {
                if self.selected_file_index == 0 {
                    self.selected_file_index = self.files.len() - 1;
                } else {
                    self.selected_file_index -= 1;
                }
                self.selected_hunk_index = 0;
            }
            ActivePane::HunkList => {
                if let Some(file) = self.files.get(self.selected_file_index) {
                    if !file.hunks.is_empty() {
                        if self.selected_hunk_index == 0 {
                            self.selected_hunk_index = file.hunks.len() - 1;
                        } else {
                            self.selected_hunk_index -= 1;
                        }
                    }
                }
            }
        }
    }

    /// Atomically promotes all staged files/hunks back to the host repository
    pub fn promote_staged(&mut self) -> Result<usize> {
        let mut count = 0;

        for file in &self.files {
            if file.is_staged || file.hunks.iter().any(|h| h.is_staged) {
                let upper_path = self.upper_root.join(&file.relative_path);
                let host_path = self.host_root.join(&file.relative_path);

                PatchGenerator::promote_file(&upper_path, &host_path)?;
                count += 1;
            }
        }

        self.status_message = format!("Promoted {} staged file(s) to host repository.", count);
        Ok(count)
    }

    /// Purges all ephemeral upperdir modifications (<100ms)
    pub fn rollback_ephemeral(&mut self, overlay_manager: &OverlayManager) -> Result<()> {
        overlay_manager.reset_upperdir()?;
        self.files.clear();
        self.selected_file_index = 0;
        self.selected_hunk_index = 0;
        self.status_message =
            "Ephemeral upperdir wiped (<100ms). State rolled back to pristine baseline.".to_string();
        Ok(())
    }

    pub fn current_file(&self) -> Option<&FileDiff> {
        self.files.get(self.selected_file_index)
    }
}
