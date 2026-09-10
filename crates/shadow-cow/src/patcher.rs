use similar::{ChangeTag, TextDiff};
use std::fs;
use std::path::Path;
use shadow_core::{Result, ShadowError};
use crate::overlay::{FileChangeType, ModifiedFile};

pub struct PatchGenerator;

impl PatchGenerator {
    /// Generates unified diff patch text comparing the lowerdir file with its modified upperdir version
    pub fn generate_unified_diff(
        lower_file_path: &Path,
        upper_file_path: &Path,
        rel_path: &Path,
    ) -> Result<String> {
        let old_content = if lower_file_path.exists() {
            fs::read_to_string(lower_file_path).unwrap_or_default()
        } else {
            String::new()
        };

        let new_content = if upper_file_path.exists() {
            fs::read_to_string(upper_file_path).unwrap_or_default()
        } else {
            String::new()
        };

        let diff = TextDiff::from_lines(&old_content, &new_content);
        let mut patch = String::new();
        let file_header = format!(
            "--- a/{}\n+++ b/{}\n",
            rel_path.display(),
            rel_path.display()
        );
        patch.push_str(&file_header);

        for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
            patch.push_str(&format!("{}", hunk.header()));
            for change in hunk.iter_changes() {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                patch.push_str(&format!("{}{}", sign, change.value()));
            }
        }

        Ok(patch)
    }

    /// Atomically applies a file promotion from upperdir to host lowerdir
    pub fn promote_file(
        upper_file_path: &Path,
        host_target_path: &Path,
    ) -> Result<()> {
        if let Some(parent) = host_target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(upper_file_path, host_target_path)?;
        tracing::info!("Promoted file to host: {}", host_target_path.display());
        Ok(())
    }
}
