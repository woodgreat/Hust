//! Target resolution: turns a file or project directory into the entry file,
//! project root, and `settings.config` values used by the rest of the
//! conductor.

use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{self, ProjectConfig};
use crate::error::{Error, Result};

#[derive(Debug, Clone)]
pub struct ResolvedTarget {
    pub entry: PathBuf,
    pub root: PathBuf,
    pub project: bool,
    pub config: ProjectConfig,
}

pub fn resolve(target: &Path, project_flag: bool) -> Result<ResolvedTarget> {
    let metadata = fs::metadata(target).map_err(|error| {
        Error::usage(format!(
            "cannot access target `{}`: {}",
            target.display(),
            error
        ))
    })?;

    if metadata.is_dir() {
        let root = fs::canonicalize(target).map_err(|error| {
            Error::usage(format!(
                "cannot resolve target `{}`: {}",
                target.display(),
                error
            ))
        })?;
        let config = config::load_from_dir(&root)?;
        let entry = root.join(&config.entry);
        let entry = validate_entry(&entry)?;
        Ok(ResolvedTarget {
            entry,
            root,
            project: true,
            config,
        })
    } else {
        let entry = fs::canonicalize(target).map_err(|error| {
            Error::usage(format!(
                "cannot resolve target `{}`: {}",
                target.display(),
                error
            ))
        })?;
        let root = entry
            .parent()
            .ok_or_else(|| {
                Error::usage(format!(
                    "target `{}` has no parent directory",
                    target.display()
                ))
            })?
            .to_path_buf();
        let config = config::load_from_dir(&root)?;
        let entry = validate_entry(&entry)?;
        Ok(ResolvedTarget {
            entry,
            root,
            project: project_flag,
            config,
        })
    }
}

fn validate_entry(entry: &Path) -> Result<PathBuf> {
    match fs::metadata(entry) {
        Ok(metadata) if metadata.is_file() => fs::canonicalize(entry).map_err(|error| {
            Error::usage(format!(
                "cannot resolve entry `{}`: {}",
                entry.display(),
                error
            ))
        }),
        Ok(_) => Err(Error::usage(format!(
            "entry is not a file: {}",
            entry.display()
        ))),
        Err(_) => Err(Error::usage(format!(
            "entry file not found: {}",
            entry.display()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "hustc-target-{}-{}-{}",
                name,
                process::id(),
                COUNTER.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn directory_target_uses_config_entry() {
        let dir = TestDir::new("config-entry");
        fs::write(
            dir.path().join("settings.config"),
            "[project]\nentry = app.hust\ncore = c\n",
        )
        .unwrap();
        fs::write(dir.path().join("app.hust"), "fn main() {}\n").unwrap();

        let resolved = resolve(dir.path(), false).unwrap();
        assert_eq!(resolved.entry.file_name().unwrap(), "app.hust");
        assert!(resolved.project);
        assert_eq!(resolved.config.core, config::CorePreference::C);
        assert_eq!(resolved.root, fs::canonicalize(dir.path()).unwrap());
    }

    #[test]
    fn directory_target_defaults_to_main_hust() {
        let dir = TestDir::new("default-entry");
        fs::write(dir.path().join("main.hust"), "fn main() {}\n").unwrap();
        let resolved = resolve(dir.path(), false).unwrap();
        assert_eq!(resolved.entry.file_name().unwrap(), "main.hust");
        assert!(resolved.project);
    }

    #[test]
    fn file_target_honors_project_flag() {
        let dir = TestDir::new("file-target");
        fs::write(
            dir.path().join("settings.config"),
            "[project]\nname = demo\n",
        )
        .unwrap();
        let entry = dir.path().join("main.hust");
        fs::write(&entry, "fn main() {}\n").unwrap();

        let resolved = resolve(&entry, true).unwrap();
        assert!(resolved.project);
        assert_eq!(resolved.config.name.as_deref(), Some("demo"));
        assert_eq!(resolved.root, fs::canonicalize(dir.path()).unwrap());
        assert_eq!(resolved.entry, fs::canonicalize(&entry).unwrap());
    }

    #[test]
    fn missing_entry_is_a_usage_error() {
        let dir = TestDir::new("missing-entry");
        let error = resolve(dir.path(), false).unwrap_err();
        assert_eq!(error.code, crate::error::EXIT_USAGE);
        assert!(error.message.contains("entry file not found"));
    }

    #[test]
    fn missing_target_is_a_usage_error() {
        let dir = TestDir::new("missing-target");
        let error = resolve(&dir.path().join("nope.hust"), false).unwrap_err();
        assert_eq!(error.code, crate::error::EXIT_USAGE);
        assert!(error.message.contains("cannot access target"));
    }
}
