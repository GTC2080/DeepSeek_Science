//! Atomic write planning contracts.
//!
//! This module defines path-safe write planning only. It does not create
//! directories, write files, rename files, or implement a persistence backend.

use crate::{StorageError, StorageRoot, WriteRequestViolation};
use std::path::{Path, PathBuf};

/// Explicit overwrite behavior for a future atomic write.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WriteMode {
    /// Create a new target only; a future writer must fail if the target exists.
    #[default]
    CreateNew,
    /// Replace an existing target only; a future writer must fail if it is missing.
    ReplaceExisting,
}

/// Caller intent for one future atomic write.
///
/// Planning validates the logical target through [`StorageRoot::join_logical`]
/// and derives a deterministic temporary sibling path. This request does not
/// write bytes by itself.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicWriteRequest {
    target_logical_path: PathBuf,
    content: Vec<u8>,
    write_mode: WriteMode,
}

impl AtomicWriteRequest {
    /// Creates a request with conservative create-new semantics.
    pub fn new(target_logical_path: impl Into<PathBuf>, content: impl Into<Vec<u8>>) -> Self {
        Self {
            target_logical_path: target_logical_path.into(),
            content: content.into(),
            write_mode: WriteMode::default(),
        }
    }

    /// Returns a copy of this request with explicit write mode.
    pub fn with_write_mode(mut self, write_mode: WriteMode) -> Self {
        self.write_mode = write_mode;
        self
    }

    /// Returns the caller-supplied logical target path.
    pub fn target_logical_path(&self) -> &Path {
        &self.target_logical_path
    }

    /// Returns the bytes intended for the future write.
    pub fn content(&self) -> &[u8] {
        &self.content
    }

    /// Returns the explicit overwrite behavior.
    pub fn write_mode(&self) -> WriteMode {
        self.write_mode
    }

    /// Builds a deterministic path plan without touching the filesystem.
    pub fn plan(&self, root: &StorageRoot) -> Result<AtomicWritePlan, StorageError> {
        let target_path = root.join_logical(&self.target_logical_path)?;

        AtomicWritePlan::new(target_path, self.write_mode)
    }
}

/// Resolved paths for a future atomic write.
///
/// The temporary path is a deterministic sibling of the final target. It is not
/// created by this plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AtomicWritePlan {
    target_path: PathBuf,
    temp_path: PathBuf,
    write_mode: WriteMode,
}

impl AtomicWritePlan {
    fn new(target_path: PathBuf, write_mode: WriteMode) -> Result<Self, StorageError> {
        let target_name = target_path
            .file_name()
            .ok_or(StorageError::InvalidWriteRequest {
                reason: WriteRequestViolation::MissingTargetFileName,
            })?;
        let target_parent = target_path
            .parent()
            .ok_or(StorageError::InvalidWriteRequest {
                reason: WriteRequestViolation::MissingTargetFileName,
            })?;

        let mut temp_name = target_name.to_os_string();
        temp_name.push(".atomic-write.tmp");
        let temp_path = target_parent.join(temp_name);

        Ok(Self {
            target_path,
            temp_path,
            write_mode,
        })
    }

    /// Returns the validated final target path.
    pub fn target_path(&self) -> &Path {
        &self.target_path
    }

    /// Returns the deterministic temporary sibling path.
    pub fn temp_path(&self) -> &Path {
        &self.temp_path
    }

    /// Returns the planned overwrite behavior.
    pub fn write_mode(&self) -> WriteMode {
        self.write_mode
    }
}

#[cfg(test)]
mod tests {
    use super::{AtomicWriteRequest, WriteMode};
    use crate::{PathSafetyViolation, StorageError, StorageRoot};
    use std::ffi::OsStr;
    use std::path::PathBuf;

    #[test]
    fn request_rejects_unsafe_logical_path_when_planned() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let request = AtomicWriteRequest::new("../escape.bin", b"x".to_vec());
        let error = request
            .plan(&root)
            .expect_err("unsafe logical target path should fail");

        assert!(matches!(
            error,
            StorageError::UnsafeRelativePath {
                reason: PathSafetyViolation::Parent,
                ..
            }
        ));
    }

    #[test]
    fn default_write_mode_is_conservative() {
        let request = AtomicWriteRequest::new("runs/run-001/state.bin", b"state".to_vec());

        assert_eq!(WriteMode::default(), WriteMode::CreateNew);
        assert_eq!(request.write_mode(), WriteMode::CreateNew);
    }

    #[test]
    fn overwrite_mode_is_explicit() {
        let request = AtomicWriteRequest::new("runs/run-001/state.bin", b"state".to_vec())
            .with_write_mode(WriteMode::ReplaceExisting);

        assert_eq!(request.write_mode(), WriteMode::ReplaceExisting);
    }

    #[test]
    fn planned_target_path_is_deterministic() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let request = AtomicWriteRequest::new("runs/run-001/state.bin", b"state".to_vec());

        let first = request.plan(&root).expect("safe path should plan");
        let second = request.plan(&root).expect("safe path should plan");

        assert_eq!(first.target_path(), second.target_path());
        assert_eq!(
            first.target_path(),
            PathBuf::from("workspace")
                .join("runs")
                .join("run-001")
                .join("state.bin")
        );
    }

    #[test]
    fn planned_temp_path_stays_under_same_safe_parent() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let request = AtomicWriteRequest::new("runs/run-001/state.bin", b"state".to_vec());

        let plan = request.plan(&root).expect("safe path should plan");

        assert_eq!(plan.temp_path().parent(), plan.target_path().parent());
        assert_eq!(
            plan.temp_path().file_name(),
            Some(OsStr::new("state.bin.atomic-write.tmp"))
        );
    }

    #[test]
    fn unsafe_paths_fail_before_a_write_plan_exists() {
        let root = StorageRoot::new("workspace").expect("valid root");
        let request = AtomicWriteRequest::new("runs/../escape.bin", b"x".to_vec());

        assert!(request.plan(&root).is_err());
    }
}
