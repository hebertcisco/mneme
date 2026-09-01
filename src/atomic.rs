use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

use crate::error::MnemeError;

/// Write bytes to `path` using a same-directory temp file, fsync, then rename.
/// Holds no lock; callers must serialize writers.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), MnemeError> {
    let parent = path
        .parent()
        .ok_or_else(|| MnemeError::other(format!("no parent for {}", path.display())))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".{}.mneme.tmp",
        uuid::Uuid::new_v4().simple()
    ));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&tmp)?;
        file.write_all(data)?;
        file.sync_all()?;
        drop(file);
        if path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename(&tmp, path)?;
        sync_dir(parent)?;
        Ok::<(), MnemeError>(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

pub fn sync_dir(dir: &Path) -> Result<(), MnemeError> {
    let file = File::open(dir)?;
    file.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn atomic_write_replaces_and_survives() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("note.md");
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        assert!(fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .all(|e| !e.file_name().to_string_lossy().contains(".tmp")));
    }
}
