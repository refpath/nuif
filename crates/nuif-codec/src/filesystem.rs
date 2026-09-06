//! Replacement of regular output files without truncating the destination.

use std::fs::{self, File};
use std::io::{self, Write as _};
use std::path::Path;

/// Writes bytes to a sibling temporary file, then replaces the destination.
///
/// Existing writable regular-file permissions are retained; new files use tempfile's
/// private permissions. Symlinks and other non-regular destinations are rejected.
/// A write or file-sync failure leaves the previous destination intact. The
/// rename is a single-file replacement, not a transaction across several files.
/// This function does not lock out concurrent writers or promise directory-entry
/// durability after power loss. Callers retain their encoded-input size bounds.
///
/// # Errors
///
/// Returns an I/O error if staging, writing, syncing or replacement fails.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    write_atomic_with(path, |file| file.write_all(bytes))
}

fn write_atomic_with(
    path: &Path,
    write: impl FnOnce(&mut File) -> io::Result<()>,
) -> io::Result<()> {
    let permissions = match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            let permissions = metadata.permissions();
            if permissions.readonly() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "output is read-only",
                ));
            }
            Some(permissions)
        }
        Ok(_) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "output must be a regular file",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut staged = tempfile::Builder::new()
        .prefix(".nuif-")
        .tempfile_in(parent)?;
    write(staged.as_file_mut())?;
    if let Some(permissions) = permissions {
        staged.as_file().set_permissions(permissions)?;
    }
    staged.as_file().sync_all()?;
    staged.persist(path).map_err(|error| error.error)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_staged_write_preserves_existing_file_and_cleans_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.nuif");
        fs::write(&path, b"previous document").unwrap();
        let error = write_atomic_with(&path, |file| {
            file.write_all(b"incomplete replacement")?;
            Err(io::Error::other("injected write failure"))
        })
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Other);
        assert_eq!(fs::read(&path).unwrap(), b"previous document");
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 1);
    }

    #[test]
    fn failed_new_file_write_leaves_no_partial_output() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("new.nuif");
        assert!(write_atomic_with(&path, |_| Err(io::Error::other("injected failure"))).is_err());
        assert!(!path.exists());
        assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 0);
    }

    #[test]
    fn replacement_removes_old_suffix_and_rejects_directories() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.nuif");
        write_atomic(&path, b"long original document").unwrap();
        write_atomic(&path, b"short").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"short");
        assert!(write_atomic(directory.path(), b"invalid").is_err());
        assert!(directory.path().is_dir());
    }

    #[test]
    fn replacement_rejects_read_only_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.nuif");
        fs::write(&path, b"old").unwrap();
        let original = fs::metadata(&path).unwrap().permissions();
        let mut read_only = original.clone();
        read_only.set_readonly(true);
        fs::set_permissions(&path, read_only).unwrap();
        let result = write_atomic(&path, b"new");
        fs::set_permissions(&path, original).unwrap();
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs::read(&path).unwrap(), b"old");
    }

    #[cfg(unix)]
    #[test]
    fn replacement_retains_permissions_and_does_not_follow_symlinks() {
        use std::os::unix::fs::{PermissionsExt as _, symlink};
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("document.nuif");
        fs::write(&path, b"old").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o640)).unwrap();
        write_atomic(&path, b"new").unwrap();
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
        let link = directory.path().join("linked.nuif");
        symlink(&path, &link).unwrap();
        assert!(write_atomic(&link, b"unexpected").is_err());
        assert!(
            fs::symlink_metadata(&link)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read(&path).unwrap(), b"new");
    }
}
