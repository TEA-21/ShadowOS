use shadow_cow::ModifiedFile;

pub enum TuiAction {
    PromoteSelected,
    Rollback,
    Quit,
    None,
}

pub struct DiffApp {
    pub files: Vec<ModifiedFile>,
    pub selected_index: usize,
    pub current_diff_preview: String,
    pub is_running: bool,
}

impl DiffApp {
    pub fn new(files: Vec<ModifiedFile>) -> Self {
        Self {
            files,
            selected_index: 0,
            current_diff_preview: String::new(),
            is_running: true,
        }
    }

    pub fn next(&mut self) {
        if !self.files.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.files.len();
        }
    }

    pub fn previous(&mut self) {
        if !self.files.is_empty() {
            if self.selected_index == 0 {
                self.selected_index = self.files.len() - 1;
            } else {
                self.selected_index -= 1;
            }
        }
    }
}
