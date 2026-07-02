//! Project entity for grouping scientific work.

use crate::{CoreError, ProjectId};
use serde::{Deserialize, Serialize};

/// A top-level scientific workspace.
///
/// `Project` is intentionally small in the kernel. Storage, domain packs, and
/// future UI shells can attach richer metadata without changing the core model.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Project {
    id: ProjectId,
    name: String,
}

impl Project {
    /// Creates a project with a generated identifier and validated name.
    pub fn new(name: impl Into<String>) -> Result<Self, CoreError> {
        let name = name.into().trim().to_owned();
        if name.is_empty() {
            return Err(CoreError::EmptyField {
                field: "project.name",
            });
        }

        Ok(Self {
            id: ProjectId::new(),
            name,
        })
    }

    /// Returns the stable project identifier.
    pub fn id(&self) -> ProjectId {
        self.id
    }

    /// Returns the display name supplied by the user.
    pub fn name(&self) -> &str {
        &self.name
    }
}
