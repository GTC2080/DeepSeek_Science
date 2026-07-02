//! Thread entity for grouping agent runs.

use crate::{CoreError, ProjectId, ThreadId};
use serde::{Deserialize, Serialize};

/// A conversation or task thread inside a project.
///
/// Threads are domain-neutral containers. They do not know whether the work is
/// chemistry, physics, mathematics, or another future science domain.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Thread {
    id: ThreadId,
    project_id: ProjectId,
    title: String,
}

impl Thread {
    /// Creates a thread for an existing project.
    pub fn new(project_id: ProjectId, title: impl Into<String>) -> Result<Self, CoreError> {
        let title = title.into().trim().to_owned();
        if title.is_empty() {
            return Err(CoreError::EmptyField {
                field: "thread.title",
            });
        }

        Ok(Self {
            id: ThreadId::new(),
            project_id,
            title,
        })
    }

    /// Returns the stable thread identifier.
    pub fn id(&self) -> ThreadId {
        self.id
    }

    /// Returns the project that owns this thread.
    pub fn project_id(&self) -> ProjectId {
        self.project_id
    }

    /// Returns the user-facing thread title.
    pub fn title(&self) -> &str {
        &self.title
    }
}
