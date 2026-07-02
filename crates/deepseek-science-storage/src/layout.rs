//! Deterministic storage layout helpers.

use deepseek_science_core::ProjectId;
use std::path::{Path, PathBuf};

/// Expected directory layout for one project.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StorageLayout {
    /// Storage root supplied by the caller.
    pub root: PathBuf,
    /// Project directory under the storage root.
    pub project_dir: PathBuf,
    /// Directory reserved for run metadata.
    pub runs_dir: PathBuf,
    /// Directory reserved for artifact files.
    pub artifacts_dir: PathBuf,
    /// Project metadata file path.
    pub metadata_path: PathBuf,
}

impl StorageLayout {
    /// Computes a deterministic project layout from a root path and project id.
    pub fn for_project(root: impl AsRef<Path>, project_id: ProjectId) -> Self {
        let root = root.as_ref().to_path_buf();
        let project_dir = root.join("projects").join(project_id.to_string());
        let runs_dir = project_dir.join("runs");
        let artifacts_dir = project_dir.join("artifacts");
        let metadata_path = project_dir.join("project.json");

        Self {
            root,
            project_dir,
            runs_dir,
            artifacts_dir,
            metadata_path,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::StorageLayout;
    use deepseek_science_core::ProjectId;

    #[test]
    fn layout_paths_are_deterministic() {
        let project_id = ProjectId::new();
        let first = StorageLayout::for_project("/workspace", project_id);
        let second = StorageLayout::for_project("/workspace", project_id);

        assert_eq!(first, second);
        assert!(first.project_dir.ends_with(project_id.to_string()));
        assert!(first.metadata_path.ends_with("project.json"));
    }
}
