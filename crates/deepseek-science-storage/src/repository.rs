//! Repository traits for future storage backends.

use crate::StorageError;
use deepseek_science_artifacts::ArtifactManifest;
use deepseek_science_core::{AgentRun, ArtifactId, Project, ProjectId, RunId};

/// Persistence boundary for projects.
pub trait ProjectRepository {
    /// Saves a project.
    fn save_project(&self, project: &Project) -> Result<(), StorageError>;

    /// Loads a project by id.
    fn load_project(&self, project_id: ProjectId) -> Result<Option<Project>, StorageError>;
}

/// Persistence boundary for agent runs.
pub trait RunRepository {
    /// Saves one run.
    fn save_run(&self, run: &AgentRun) -> Result<(), StorageError>;

    /// Loads one run by id.
    fn load_run(&self, run_id: RunId) -> Result<Option<AgentRun>, StorageError>;
}

/// Persistence boundary for artifact manifests.
pub trait ArtifactRepository {
    /// Saves an artifact manifest.
    fn save_artifact_manifest(&self, manifest: &ArtifactManifest) -> Result<(), StorageError>;

    /// Loads an artifact manifest by id.
    fn load_artifact_manifest(
        &self,
        artifact_id: ArtifactId,
    ) -> Result<Option<ArtifactManifest>, StorageError>;
}

#[cfg(test)]
mod tests {
    use super::{ArtifactRepository, ProjectRepository, RunRepository};

    #[test]
    fn repository_traits_remain_object_safe_contracts() {
        let project_repo: Option<&dyn ProjectRepository> = None;
        let run_repo: Option<&dyn RunRepository> = None;
        let artifact_repo: Option<&dyn ArtifactRepository> = None;

        assert!(project_repo.is_none());
        assert!(run_repo.is_none());
        assert!(artifact_repo.is_none());
    }
}
