//! Strongly typed identifiers shared by the kernel.
//!
//! Newtypes prevent accidentally mixing project, thread, run, step, and
//! artifact identifiers while keeping serialization compact and predictable.

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a project workspace.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ProjectId(Uuid);

impl ProjectId {
    /// Creates a new globally unique project identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Builds a project identifier from an existing UUID value.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for storage adapters and tests.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ProjectId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ProjectId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Unique identifier for a conversation thread inside a project.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ThreadId(Uuid);

impl ThreadId {
    /// Creates a new globally unique thread identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Builds a thread identifier from an existing UUID value.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for storage adapters and tests.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ThreadId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ThreadId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Unique identifier for one agent run.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct RunId(Uuid);

impl RunId {
    /// Creates a new globally unique run identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Builds a run identifier from an existing UUID value.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for storage adapters and tests.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for RunId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Unique identifier for a step recorded during an agent run.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct StepId(Uuid);

impl StepId {
    /// Creates a new globally unique step identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Builds a step identifier from an existing UUID value.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for storage adapters and tests.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for StepId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for StepId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

/// Unique identifier for an artifact produced or referenced by a run.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct ArtifactId(Uuid);

impl ArtifactId {
    /// Creates a new globally unique artifact identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Builds an artifact identifier from an existing UUID value.
    pub fn from_uuid(value: Uuid) -> Self {
        Self(value)
    }

    /// Returns the underlying UUID for storage adapters and tests.
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }
}

impl Default for ArtifactId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ArtifactId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::ProjectId;
    use uuid::Uuid;

    #[test]
    fn create_project_id() {
        let id = ProjectId::new();

        assert_ne!(id.as_uuid(), Uuid::nil());
    }
}
