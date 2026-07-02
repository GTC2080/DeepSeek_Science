//! Pure storage layout contracts.
//!
//! This module computes paths only. It never canonicalizes, checks existence,
//! creates directories, writes files, or binds the storage crate to a concrete
//! database or filesystem implementation.

use crate::{PathSafetyViolation, StorageError};
use deepseek_science_core::ProjectId;
use std::path::{Component, Path, PathBuf};

/// Caller-supplied storage root for future project data.
///
/// Construction validates only the path value itself. It does not require the
/// directory to exist and does not canonicalize through the filesystem.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageRoot {
    path: PathBuf,
}

impl StorageRoot {
    /// Creates a storage root without touching the filesystem.
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, StorageError> {
        let path = path.into();
        if path.as_os_str().is_empty() {
            return Err(StorageError::InvalidStorageRoot {
                reason: "root path is empty".to_string(),
            });
        }

        Ok(Self { path })
    }

    /// Returns the root path supplied by the caller.
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Consumes the root and returns the underlying path buffer.
    pub fn into_path_buf(self) -> PathBuf {
        self.path
    }

    /// Joins a logical relative path under this root after traversal checks.
    pub fn join_logical(&self, relative_path: impl AsRef<Path>) -> Result<PathBuf, StorageError> {
        let relative_path = relative_path.as_ref();
        validate_logical_relative_path(relative_path)?;

        Ok(self.path.join(relative_path))
    }
}

/// Expected directory layout for one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectLayout {
    /// Storage root supplied by the caller.
    pub root: PathBuf,
    /// Project directory under the storage root.
    pub project_dir: PathBuf,
    /// Directory reserved for raw input files.
    pub raw_files_dir: PathBuf,
    /// Directory reserved for derived files.
    pub derived_files_dir: PathBuf,
    /// Directory reserved for run metadata.
    pub runs_dir: PathBuf,
    /// Directory reserved for artifact files.
    pub artifacts_dir: PathBuf,
    /// Directory reserved for report artifacts.
    pub reports_dir: PathBuf,
    /// Directory reserved for table artifacts.
    pub tables_dir: PathBuf,
    /// Directory reserved for figure artifacts.
    pub figures_dir: PathBuf,
    /// Directory reserved for code artifacts.
    pub code_dir: PathBuf,
    /// Directory reserved for model call logs.
    pub model_call_logs_dir: PathBuf,
    /// Directory reserved for tool call logs.
    pub tool_call_logs_dir: PathBuf,
    /// Directory reserved for core event logs.
    pub event_logs_dir: PathBuf,
    /// Project metadata file path.
    pub metadata_path: PathBuf,
}

impl ProjectLayout {
    /// Computes a deterministic project layout from a validated storage root.
    pub fn new(root: StorageRoot, project_id: ProjectId) -> Self {
        Self::from_root_path(root.into_path_buf(), project_id)
    }

    /// Computes a deterministic project layout from a root path and project id.
    ///
    /// This keeps the original Phase 1 API available for lightweight callers.
    /// Use [`StorageRoot::new`] when caller-supplied roots must be validated.
    pub fn for_project(root: impl AsRef<Path>, project_id: ProjectId) -> Self {
        Self::from_root_path(root.as_ref().to_path_buf(), project_id)
    }

    fn from_root_path(root: PathBuf, project_id: ProjectId) -> Self {
        let project_dir = root.join("projects").join(project_id.to_string());
        let files_dir = project_dir.join("files");
        let runs_dir = project_dir.join("runs");
        let artifacts_dir = project_dir.join("artifacts");
        let logs_dir = project_dir.join("logs");
        let metadata_path = project_dir.join("project.json");

        Self {
            root,
            project_dir: project_dir.clone(),
            raw_files_dir: files_dir.join("raw"),
            derived_files_dir: files_dir.join("derived"),
            runs_dir,
            artifacts_dir: artifacts_dir.clone(),
            reports_dir: artifacts_dir.join("reports"),
            tables_dir: artifacts_dir.join("tables"),
            figures_dir: artifacts_dir.join("figures"),
            code_dir: artifacts_dir.join("code"),
            model_call_logs_dir: logs_dir.join("model-calls"),
            tool_call_logs_dir: logs_dir.join("tool-calls"),
            event_logs_dir: logs_dir.join("events"),
            metadata_path,
        }
    }
}

/// Backward-compatible name for the project storage layout.
pub type StorageLayout = ProjectLayout;

fn validate_logical_relative_path(path: &Path) -> Result<(), StorageError> {
    if path.as_os_str().is_empty() {
        return Err(unsafe_relative_path(path, PathSafetyViolation::Empty));
    }

    if path.is_absolute() {
        return Err(unsafe_relative_path(path, PathSafetyViolation::Absolute));
    }

    for component in path.components() {
        let violation = match component {
            Component::Prefix(_) => Some(PathSafetyViolation::Prefix),
            Component::RootDir => Some(PathSafetyViolation::Root),
            Component::ParentDir => Some(PathSafetyViolation::Parent),
            Component::CurDir => Some(PathSafetyViolation::CurrentDir),
            Component::Normal(_) => None,
        };

        if let Some(reason) = violation {
            return Err(unsafe_relative_path(path, reason));
        }
    }

    Ok(())
}

fn unsafe_relative_path(path: &Path, reason: PathSafetyViolation) -> StorageError {
    StorageError::UnsafeRelativePath {
        path: path.to_path_buf(),
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectLayout, StorageRoot};
    use crate::{PathSafetyViolation, StorageError};
    use deepseek_science_core::ProjectId;
    use std::path::PathBuf;

    #[test]
    fn storage_root_accepts_relative_test_path() {
        let root = StorageRoot::new("storage-root").expect("relative root should be valid");

        assert_eq!(root.as_path(), PathBuf::from("storage-root").as_path());
    }

    #[test]
    fn storage_root_rejects_empty_path() {
        let error = StorageRoot::new(PathBuf::new()).expect_err("empty root should fail");

        assert!(matches!(error, StorageError::InvalidStorageRoot { .. }));
    }

    #[test]
    fn project_layout_paths_are_deterministic() {
        let project_id = ProjectId::new();
        let root = StorageRoot::new("workspace").expect("valid root");
        let first = ProjectLayout::new(root.clone(), project_id);
        let second = ProjectLayout::new(root, project_id);

        assert_eq!(first, second);
        assert!(first.project_dir.ends_with(project_id.to_string()));
        assert!(first.metadata_path.ends_with("project.json"));
        assert!(first.raw_files_dir.ends_with("files/raw"));
        assert!(first.derived_files_dir.ends_with("files/derived"));
        assert!(first.reports_dir.ends_with("artifacts/reports"));
        assert!(first.tables_dir.ends_with("artifacts/tables"));
        assert!(first.figures_dir.ends_with("artifacts/figures"));
        assert!(first.code_dir.ends_with("artifacts/code"));
        assert!(first.model_call_logs_dir.ends_with("logs/model-calls"));
        assert!(first.tool_call_logs_dir.ends_with("logs/tool-calls"));
        assert!(first.event_logs_dir.ends_with("logs/events"));
    }

    #[test]
    fn project_layout_does_not_require_filesystem_existence() {
        let project_id = ProjectId::new();
        let root = StorageRoot::new("missing/storage/root").expect("valid root");
        let layout = ProjectLayout::new(root, project_id);

        assert_eq!(
            layout.project_dir,
            PathBuf::from("missing/storage/root")
                .join("projects")
                .join(project_id.to_string())
        );
    }

    #[test]
    fn safe_join_accepts_normal_relative_path() {
        let root = StorageRoot::new("workspace").expect("valid root");

        let path = root
            .join_logical("projects/project_001/runs/run_001/events.jsonl")
            .expect("normal relative path should be safe");

        assert_eq!(
            path,
            PathBuf::from("workspace")
                .join("projects")
                .join("project_001")
                .join("runs")
                .join("run_001")
                .join("events.jsonl")
        );
    }

    #[test]
    fn safe_join_rejects_absolute_paths() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let absolute = if cfg!(windows) {
            PathBuf::from(r"C:\secrets")
        } else {
            PathBuf::from("/secrets")
        };
        let error = root
            .join_logical(absolute)
            .expect_err("absolute path should fail");

        assert!(matches!(
            error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Absolute,
                ..
            }
        ));
    }

    #[test]
    fn safe_join_rejects_parent_component() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let error = root.join_logical("..").expect_err("parent should fail");

        assert!(matches!(
            error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Parent,
                ..
            }
        ));
    }

    #[test]
    fn safe_join_rejects_traversal_attempts() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let error = root
            .join_logical("runs/../secrets")
            .expect_err("traversal should fail");

        assert!(matches!(
            error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Parent,
                ..
            }
        ));
    }

    #[test]
    fn safe_join_rejects_empty_relative_path() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let error = root
            .join_logical(PathBuf::new())
            .expect_err("empty logical path should fail");

        assert!(matches!(
            error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Empty,
                ..
            }
        ));
    }

    #[cfg(windows)]
    #[test]
    fn safe_join_rejects_root_and_prefix_components() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let rooted_error = root
            .join_logical(r"\rooted")
            .expect_err("root component should fail");
        let prefixed_error = root
            .join_logical(r"C:relative")
            .expect_err("prefix component should fail");

        assert!(matches!(
            rooted_error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Root,
                ..
            }
        ));
        assert!(matches!(
            prefixed_error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Prefix,
                ..
            }
        ));
    }
}
