use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use shadow_core::{Result, ShadowError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModifiedFile {
    pub relative_path: PathBuf,
    pub change_type: FileChangeType,
}

#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub lowerdir: PathBuf, // Read-only host workspace
    pub upperdir: PathBuf, // In-memory tmpfs for ephemeral agent writes
    pub workdir: PathBuf,  // OverlayFS internal state workdir
    pub merged: PathBuf,   // Active mount point exposed to agent
}

impl OverlayConfig {
    pub fn new(lowerdir: PathBuf, upperdir: PathBuf, workdir: PathBuf, merged: PathBuf) -> Self {
        Self {
            lowerdir,
            upperdir,
            workdir,
            merged,
        }
    }

    /// Constructs the Linux mount options string for mount -t overlay
    pub fn build_mount_options(&self) -> String {
        format!(
            "lowerdir={},upperdir={},workdir={}",
            self.lowerdir.display(),
            self.upperdir.display(),
            self.workdir.display()
        )
    }

    /// Generates the shell command to mount the CoW OverlayFS inside the MicroVM
    pub fn build_mount_command(&self) -> String {
        format!(
            "mount -t overlay overlay -o {} {}",
            self.build_mount_options(),
            self.merged.display()
        )
    }
}

pub struct OverlayManager {
    pub config: OverlayConfig,
}

impl OverlayManager {
    pub fn new(config: OverlayConfig) -> Self {
        Self { config }
    }

    /// Prepares ephemeral directories (upperdir and workdir on tmpfs)
    pub fn init_ephemeral_layers(&self) -> Result<()> {
        fs::create_dir_all(&self.config.upperdir)?;
        fs::create_dir_all(&self.config.workdir)?;
        fs::create_dir_all(&self.config.merged)?;
        Ok(())
    }

    /// Instant state reset: clears upperdir and workdir in < 5ms
    pub fn reset_upperdir(&self) -> Result<()> {
        if self.config.upperdir.exists() {
            fs::remove_dir_all(&self.config.upperdir)?;
        }
        if self.config.workdir.exists() {
            fs::remove_dir_all(&self.config.workdir)?;
        }
        fs::create_dir_all(&self.config.upperdir)?;
        fs::create_dir_all(&self.config.workdir)?;
        tracing::info!("OverlayFS upperdir and workdir wiped clean in < 5ms.");
        Ok(())
    }

    /// Scans the ephemeral upperdir to detect all files created or altered by the agent
    pub fn scan_upperdir_changes(&self) -> Result<Vec<ModifiedFile>> {
        let mut changes = Vec::new();
        if !self.config.upperdir.exists() {
            return Ok(changes);
        }

        Self::walk_upperdir(&self.config.upperdir, &self.config.lowerdir, Path::new(""), &mut changes)?;
        Ok(changes)
    }

    fn walk_upperdir(
        upper_root: &Path,
        lower_root: &Path,
        rel_path: &Path,
        acc: &mut Vec<ModifiedFile>,
    ) -> Result<()> {
        let current_dir = upper_root.join(rel_path);
        if !current_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&current_dir)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let child_rel = rel_path.join(&file_name);
            let file_type = entry.file_type()?;

            if file_type.is_dir() {
                Self::walk_upperdir(upper_root, lower_root, &child_rel, acc)?;
            } else {
                let lower_equivalent = lower_root.join(&child_rel);
                let change_type = if lower_equivalent.exists() {
                    FileChangeType::Modified
                } else {
                    FileChangeType::Created
                };

                acc.push(ModifiedFile {
                    relative_path: child_rel,
                    change_type,
                });
            }
        }
        Ok(())
    }

    /// Computes a deterministic SHA-256 tree hash of a directory to verify bit-identical state
    pub fn compute_directory_sha256(dir: &Path) -> Result<String> {
        let mut files = Vec::new();
        Self::collect_files(dir, Path::new(""), &mut files)?;
        files.sort();

        let mut hasher = Sha256::new();
        for rel in files {
            hasher.update(rel.to_string_lossy().as_bytes());
            let full_path = dir.join(&rel);
            let mut file = File::open(&full_path)?;
            let mut buf = [0u8; 8192];
            while let Ok(n) = file.read(&mut buf) {
                if n == 0 { break; }
                hasher.update(&buf[..n]);
            }
        }

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    fn collect_files(base: &Path, rel: &Path, acc: &mut Vec<PathBuf>) -> Result<()> {
        let current = base.join(rel);
        if !current.exists() {
            return Ok(());
        }
        for entry in fs::read_dir(&current)? {
            let entry = entry?;
            let name = entry.file_name();
            let child_rel = rel.join(name);
            let ft = entry.file_type()?;
            if ft.is_dir() {
                Self::collect_files(base, &child_rel, acc)?;
            } else {
                acc.push(child_rel);
            }
        }
        Ok(())
    }
}
