use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::error::MnemeError;

/// Write bytes to `path` using a same-directory temp file, fsync, then rename.
/// Holds no lock; callers must serialize writers.
pub fn atomic_write(path: &Path, data: &[u8]) -> Result<(), MnemeError> {
    let parent = path
        .parent()
        .ok_or_else(|| MnemeError::other(format!("no parent for {}", path.display())))?;
    fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(".{}.mneme.tmp", uuid::Uuid::new_v4().simple()));
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

/// Best-effort directory fsync after a durable file rename.
///
/// On Windows, `File::open(dir)` returns `ERROR_ACCESS_DENIED` (os error 5)
/// because a directory cannot be opened with ordinary read access. Flushing a
/// directory handle also fails on some cloud/network filesystems (Dropbox, NFS).
/// The file itself was already `sync_all`'d; directory metadata durability is
/// optional and must not abort a successful write.
pub fn sync_dir(dir: &Path) -> Result<(), MnemeError> {
    let file = match File::open(dir) {
        Ok(f) => f,
        Err(err) if ignorable_dir_sync(&err) => return Ok(()),
        Err(err) => return Err(io_path(dir, err)),
    };
    match file.sync_all() {
        Ok(()) => Ok(()),
        Err(err) if ignorable_dir_sync(&err) => Ok(()),
        Err(err) => Err(io_path(dir, err)),
    }
}

fn ignorable_dir_sync(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::PermissionDenied | io::ErrorKind::Unsupported | io::ErrorKind::InvalidInput
    )
}

fn io_path(path: &Path, err: io::Error) -> MnemeError {
    MnemeError::Io(io::Error::new(
        err.kind(),
        format!("{}: {err}", path.display()),
    ))
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

    #[test]
    fn sync_dir_on_fresh_directory_is_ok() {
        let dir = tempdir().unwrap();
        sync_dir(dir.path()).unwrap();
        atomic_write(&dir.path().join("config.toml"), b"language = \"en\"\n").unwrap();
        assert!(dir.path().join("config.toml").is_file());
    }
}
