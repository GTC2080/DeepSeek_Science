//! Domain-neutral workflow plan descriptions.
//!
//! A workflow plan is only an ordered in-memory description. It does not store
//! runtime state, execute tools, call models, read files, or write files.

use crate::CoreError;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt};

/// Stable caller-provided identifier for a workflow plan.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WorkflowId(String);

impl WorkflowId {
    /// Creates a workflow identifier from a non-empty deterministic value.
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(CoreError::EmptyWorkflowId);
        }

        Ok(Self(value))
    }

    /// Returns the identifier string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Stable caller-provided key for a step inside a workflow plan.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct WorkflowStepKey(String);

impl WorkflowStepKey {
    /// Creates a workflow step key from a non-empty deterministic value.
    pub fn new(value: impl Into<String>) -> Result<Self, CoreError> {
        let value = value.into().trim().to_owned();
        if value.is_empty() {
            return Err(CoreError::EmptyWorkflowStepKey);
        }

        Ok(Self(value))
    }

    /// Returns the step key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowStepKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Broad conceptual category for a planned workflow step.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkflowStepKind {
    /// Inspect caller-provided input before planning or execution.
    InspectInput,
    /// Prepare or refine a plan.
    Plan,
    /// Planned model interaction without binding to a provider.
    Model,
    /// Planned tool interaction without binding to a tool registry.
    Tool,
    /// Review or validate intermediate results.
    Review,
    /// Produce an artifact or output record.
    ProduceArtifact,
    /// Mark the planned workflow complete.
    Complete,
    /// Domain-specific category owned by a future workflow pack.
    Custom,
}

/// One ordered step in a workflow plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkflowStepPlan {
    key: WorkflowStepKey,
    kind: WorkflowStepKind,
    label: String,
    description: Option<String>,
}

impl WorkflowStepPlan {
    /// Creates a planned step with a stable key and human-readable label.
    pub fn new(
        key: WorkflowStepKey,
        kind: WorkflowStepKind,
        label: impl Into<String>,
        description: Option<String>,
    ) -> Result<Self, CoreError> {
        let label = label.into().trim().to_owned();
        if label.is_empty() {
            return Err(CoreError::EmptyWorkflowStepLabel);
        }
        let description = description
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        Ok(Self {
            key,
            kind,
            label,
            description,
        })
    }

    /// Returns the stable step key.
    pub fn key(&self) -> &WorkflowStepKey {
        &self.key
    }

    /// Returns the broad conceptual step category.
    pub fn kind(&self) -> WorkflowStepKind {
        self.kind
    }

    /// Returns the human-readable step label.
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Returns the optional step description.
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
}

/// Ordered in-memory workflow plan.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WorkflowPlan {
    id: WorkflowId,
    name: String,
    steps: Vec<WorkflowStepPlan>,
}

impl WorkflowPlan {
    /// Creates a workflow plan with caller-provided deterministic step order.
    pub fn new(
        id: WorkflowId,
        name: impl Into<String>,
        steps: Vec<WorkflowStepPlan>,
    ) -> Result<Self, CoreError> {
        let name = name.into().trim().to_owned();
        if name.is_empty() {
            return Err(CoreError::EmptyWorkflowName);
        }
        if steps.is_empty() {
            return Err(CoreError::EmptyWorkflowPlan);
        }

        let mut seen = HashSet::new();
        for step in &steps {
            if !seen.insert(step.key()) {
                return Err(CoreError::DuplicateWorkflowStep {
                    step_key: step.key().clone(),
                });
            }
        }

        Ok(Self { id, name, steps })
    }

    /// Returns the workflow identifier.
    pub fn id(&self) -> &WorkflowId {
        &self.id
    }

    /// Returns the human-readable workflow name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns planned steps in caller-provided order.
    pub fn steps(&self) -> &[WorkflowStepPlan] {
        &self.steps
    }

    /// Returns the number of planned steps.
    pub fn step_count(&self) -> usize {
        self.steps.len()
    }

    /// Iterates over planned step keys in caller-provided order.
    pub fn step_keys(&self) -> impl Iterator<Item = &WorkflowStepKey> {
        self.steps.iter().map(WorkflowStepPlan::key)
    }

    /// Returns true when the plan contains the requested step key.
    pub fn contains_step(&self, key: &WorkflowStepKey) -> bool {
        self.steps.iter().any(|step| step.key() == key)
    }
}

#[cfg(test)]
mod tests {
    use super::{WorkflowId, WorkflowPlan, WorkflowStepKey, WorkflowStepKind, WorkflowStepPlan};
    use crate::CoreError;

    fn workflow_id(value: &str) -> WorkflowId {
        WorkflowId::new(value).expect("test workflow id should be valid")
    }

    fn step_key(value: &str) -> WorkflowStepKey {
        WorkflowStepKey::new(value).expect("test workflow step key should be valid")
    }

    fn step(key: &str, kind: WorkflowStepKind, label: &str) -> WorkflowStepPlan {
        WorkflowStepPlan::new(step_key(key), kind, label, None)
            .expect("test workflow step should be valid")
    }

    fn ordered_steps() -> Vec<WorkflowStepPlan> {
        vec![
            step("inspect", WorkflowStepKind::InspectInput, "Inspect input"),
            step("plan", WorkflowStepKind::Plan, "Plan work"),
            step("complete", WorkflowStepKind::Complete, "Complete"),
        ]
    }

    #[test]
    fn workflow_id_rejects_empty_value() {
        let result = WorkflowId::new("  ");

        assert_eq!(result, Err(CoreError::EmptyWorkflowId));
    }

    #[test]
    fn workflow_step_key_rejects_empty_value() {
        let result = WorkflowStepKey::new("  ");

        assert_eq!(result, Err(CoreError::EmptyWorkflowStepKey));
    }

    #[test]
    fn workflow_step_plan_rejects_empty_label() {
        let result = WorkflowStepPlan::new(
            step_key("inspect"),
            WorkflowStepKind::InspectInput,
            "  ",
            None,
        );

        assert_eq!(result, Err(CoreError::EmptyWorkflowStepLabel));
    }

    #[test]
    fn workflow_plan_can_be_constructed_with_ordered_steps() {
        let plan = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");

        assert_eq!(plan.id().as_str(), "generic");
        assert_eq!(plan.name(), "Generic workflow");
        assert_eq!(plan.steps()[0].key().as_str(), "inspect");
        assert_eq!(plan.steps()[1].key().as_str(), "plan");
        assert_eq!(plan.steps()[2].key().as_str(), "complete");
    }

    #[test]
    fn workflow_plan_rejects_empty_name() {
        let result = WorkflowPlan::new(workflow_id("generic"), "  ", ordered_steps());

        assert_eq!(result, Err(CoreError::EmptyWorkflowName));
    }

    #[test]
    fn workflow_plan_rejects_zero_steps() {
        let result = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", Vec::new());

        assert_eq!(result, Err(CoreError::EmptyWorkflowPlan));
    }

    #[test]
    fn workflow_plan_rejects_duplicate_step_keys() {
        let result = WorkflowPlan::new(
            workflow_id("generic"),
            "Generic workflow",
            vec![
                step("inspect", WorkflowStepKind::InspectInput, "Inspect input"),
                step("inspect", WorkflowStepKind::Review, "Review input"),
            ],
        );

        assert_eq!(
            result,
            Err(CoreError::DuplicateWorkflowStep {
                step_key: step_key("inspect"),
            })
        );
    }

    #[test]
    fn workflow_plan_preserves_caller_provided_step_order() {
        let plan = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");

        let keys: Vec<_> = plan.step_keys().map(WorkflowStepKey::as_str).collect();

        assert_eq!(keys, vec!["inspect", "plan", "complete"]);
    }

    #[test]
    fn workflow_plan_reports_step_count() {
        let plan = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");

        assert_eq!(plan.step_count(), 3);
    }

    #[test]
    fn workflow_plan_contains_known_step_key() {
        let plan = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");

        assert!(plan.contains_step(&step_key("plan")));
        assert!(!plan.contains_step(&step_key("missing")));
    }

    #[test]
    fn workflow_plan_does_not_require_domain_or_runtime_resources() {
        let plan = WorkflowPlan::new(
            workflow_id("domain-neutral"),
            "Domain-neutral workflow",
            vec![
                step("inspect", WorkflowStepKind::InspectInput, "Inspect input"),
                step(
                    "produce",
                    WorkflowStepKind::ProduceArtifact,
                    "Produce output",
                ),
                step("complete", WorkflowStepKind::Complete, "Complete"),
            ],
        )
        .expect("valid workflow plan should construct cleanly");

        assert_eq!(plan.step_count(), 3);
    }

    #[test]
    fn repeated_construction_with_same_inputs_is_deterministic() {
        let first = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");
        let second = WorkflowPlan::new(workflow_id("generic"), "Generic workflow", ordered_steps())
            .expect("valid workflow plan should construct cleanly");

        assert_eq!(first, second);
    }
}
