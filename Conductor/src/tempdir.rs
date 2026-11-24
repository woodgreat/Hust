//! Temporary build directories owned by the conductor.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Error, Result};

static COUNTER: AtomicU64 = AtomicU64::new(0);

pub struct TempDir {
    path: PathBuf,
    keep: bool,
}

impl TempDir {
    pub fn create(keep: bool) -> Result<Self> {
        let base = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);

        for _ in 0..100 {
            let sequence = COUNTER.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "hustc-{}-{}-{}",
                std::process::id(),
                nanos,
                sequence
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path, keep }),
                Err(error) if error.kind() == ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(Error::internal(format!(
                        "cannot create temporary directory `{}`: {}",
                        path.display(),
                        error
                    )))
                }
            }
        }

        Err(Error::internal(format!(
            "cannot create a unique temporary directory in `{}`",
            base.display()
        )))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if self.keep {
            eprintln!("Temporary directory kept: {}", self.path.display());
            return;
        }
        if let Err(error) = fs::remove_dir_all(&self.path) {
            if error.kind() != ErrorKind::NotFound {
                eprintln!(
                    "Warning: cannot remove temporary directory `{}`: {}",
                    self.path.display(),
                    error
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_and_removes_a_directory() {
        let path = {
            let temp = TempDir::create(false).unwrap();
            assert!(temp.path().is_dir());
            temp.path().to_path_buf()
        };
        assert!(!path.exists());
    }

    #[test]
    fn keep_preserves_the_directory() {
        let path = {
            let temp = TempDir::create(true).unwrap();
            temp.path().to_path_buf()
        };
        assert!(path.is_dir());
        let _ = fs::remove_dir_all(&path);
    }
}
