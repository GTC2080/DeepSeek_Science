//! Agent run and step state for replayable kernel execution.

use crate::{CoreError, CoreEvent, RunId, StepId, ThreadId};
use crate::{WorkflowPlan, WorkflowStepKey, WorkflowStepPlan};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Lifecycle state for an agent run or run step.
///
/// The state machine is deliberately small and explicit. It models planning,
/// approvals, model work, tool work, review, and terminal states without
/// embedding domain workflow details in the core crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunState {
    /// The run has been created but no plan has been produced.
    Created,
    /// The agent is preparing a plan or prompt context.
    Planning,
    /// The run is paused until a user approves a risky action.
    WaitingForApproval,
    /// The run is waiting for a model response.
    RunningModel,
    /// The run is waiting for a tool result.
    RunningTool,
    /// The run is passing through a review or validation stage.
    Reviewing,
    /// The run finished successfully.
    Completed,
    /// The run failed with a recorded error.
    Failed,
    /// The run was canceled before completion.
    Canceled,
}

impl RunState {
    /// Returns true when the requested transition is allowed by the kernel.
    pub fn can_transition_to(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Created, Self::Planning)
                | (Self::Created, Self::Canceled)
                | (Self::Planning, Self::WaitingForApproval)
                | (Self::Planning, Self::RunningModel)
                | (Self::Planning, Self::RunningTool)
                | (Self::Planning, Self::Failed)
                | (Self::Planning, Self::Canceled)
                | (Self::WaitingForApproval, Self::RunningModel)
                | (Self::WaitingForApproval, Self::RunningTool)
                | (Self::WaitingForApproval, Self::Failed)
                | (Self::WaitingForApproval, Self::Canceled)
                | (Self::RunningModel, Self::RunningTool)
                | (Self::RunningModel, Self::Reviewing)
                | (Self::RunningModel, Self::Completed)
                | (Self::RunningModel, Self::Failed)
                | (Self::RunningModel, Self::Canceled)
                | (Self::RunningTool, Self::RunningModel)
                | (Self::RunningTool, Self::Reviewing)
                | (Self::RunningTool, Self::Completed)
                | (Self::RunningTool, Self::Failed)
                | (Self::RunningTool, Self::Canceled)
                | (Self::Reviewing, Self::Completed)
                | (Self::Reviewing, Self::Failed)
                | (Self::Reviewing, Self::Canceled)
        )
    }

    /// Returns true when the state closes the run lifecycle.
    pub fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Canceled)
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Created => "created",
            Self::Planning => "planning",
            Self::WaitingForApproval => "waiting_for_approval",
            Self::RunningModel => "running_model",
            Self::RunningTool => "running_tool",
            Self::Reviewing => "reviewing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Canceled => "canceled",
        };
        formatter.write_str(label)
    }
}

/// One replayable run of an agent against a thread.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentRun {
    id: RunId,
    thread_id: ThreadId,
    state: RunState,
    steps: Vec<RunStep>,
}

impl AgentRun {
    /// Creates a run in the `Created` state.
    pub fn new(thread_id: ThreadId) -> Self {
        Self {
            id: RunId::new(),
            thread_id,
            state: RunState::Created,
            steps: Vec::new(),
        }
    }

    /// Prepares a `Created` run skeleton from an ordered workflow plan.
    ///
    /// This is a pure in-memory projection: it copies planned step labels and
    /// workflow step keys into `Created` run steps, but it does not execute
    /// steps, call models, call tools, persist data, or emit events.
    pub fn prepare_from_plan(run_id: RunId, thread_id: ThreadId, plan: &WorkflowPlan) -> Self {
        let steps = plan
            .steps()
            .iter()
            .enumerate()
            .map(|(index, step)| RunStep::planned_from_workflow(run_id, index, step))
            .collect();

        Self {
            id: run_id,
            thread_id,
            state: RunState::Created,
            steps,
        }
    }

    /// Returns the stable run identifier.
    pub fn id(&self) -> RunId {
        self.id
    }

    /// Returns the thread this run belongs to.
    pub fn thread_id(&self) -> ThreadId {
        self.thread_id
    }

    /// Returns the current lifecycle state.
    pub fn state(&self) -> RunState {
        self.state
    }

    /// Returns all recorded steps in replay order.
    pub fn steps(&self) -> &[RunStep] {
        &self.steps
    }

    /// Moves the run to a new state when the transition is valid.
    pub fn transition_to(&mut self, next: RunState) -> Result<(), CoreError> {
        if !self.state.can_transition_to(next) {
            return Err(CoreError::InvalidRunTransition {
                from: self.state,
                to: next,
            });
        }

        self.state = next;
        Ok(())
    }

    /// Moves the run to a valid state and returns the matching transition event.
    pub fn transition_to_with_event(&mut self, next: RunState) -> Result<CoreEvent, CoreError> {
        let from = self.state;
        self.transition_to(next)?;
        Ok(CoreEvent::run_state_changed(self.id, from, next))
    }

    /// Appends a step to the run in replay order.
    pub fn record_step(&mut self, step: RunStep) {
        self.steps.push(step);
    }
}

/// A durable step recorded during an agent run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RunStep {
    id: StepId,
    title: String,
    state: RunState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    workflow_step_key: Option<WorkflowStepKey>,
}

impl RunStep {
    /// Creates a run step with a generated identifier.
    pub fn new(title: impl Into<String>, state: RunState) -> Result<Self, CoreError> {
        let title = title.into().trim().to_owned();
        if title.is_empty() {
            return Err(CoreError::EmptyField {
                field: "run_step.title",
            });
        }

        Ok(Self {
            id: StepId::new(),
            title,
            state,
            workflow_step_key: None,
        })
    }

    fn planned_from_workflow(run_id: RunId, index: usize, step: &WorkflowStepPlan) -> Self {
        Self {
            id: planned_step_id(run_id, index),
            title: step.label().to_owned(),
            state: RunState::Created,
            workflow_step_key: Some(step.key().clone()),
        }
    }

    /// Returns the stable step identifier.
    pub fn id(&self) -> StepId {
        self.id
    }

    /// Returns the step label shown in audit views.
    pub fn title(&self) -> &str {
        &self.title
    }

    /// Returns the step state at the time it was recorded.
    pub fn state(&self) -> RunState {
        self.state
    }

    /// Returns the source workflow step key for planned run skeleton steps.
    pub fn workflow_step_key(&self) -> Option<&WorkflowStepKey> {
        self.workflow_step_key.as_ref()
    }
}

fn planned_step_id(run_id: RunId, index: usize) -> StepId {
    let value = run_id.as_uuid().as_u128().wrapping_add((index as u128) + 1);
    StepId::from_uuid(Uuid::from_u128(value))
}

#[cfg(test)]
mod tests {
    use super::{AgentRun, RunState};
    use crate::{
        CoreError, RunId, RunInspection, ThreadId, WorkflowId, WorkflowPlan, WorkflowStepKey,
        WorkflowStepKind, WorkflowStepPlan,
    };
    use uuid::Uuid;

    fn fixed_run_id() -> RunId {
        RunId::from_uuid(Uuid::from_u128(1))
    }

    fn fixed_thread_id() -> ThreadId {
        ThreadId::from_uuid(Uuid::from_u128(2))
    }

    fn workflow_id(value: &str) -> WorkflowId {
        WorkflowId::new(value).expect("test workflow id should be valid")
    }

    fn step_key(value: &str) -> WorkflowStepKey {
        WorkflowStepKey::new(value).expect("test workflow step key should be valid")
    }

    fn workflow_step(key: &str, kind: WorkflowStepKind, label: &str) -> WorkflowStepPlan {
        WorkflowStepPlan::new(step_key(key), kind, label, None)
            .expect("test workflow step should be valid")
    }

    fn workflow_plan() -> WorkflowPlan {
        WorkflowPlan::new(
            workflow_id("generic"),
            "Generic workflow",
            vec![
                workflow_step("inspect", WorkflowStepKind::InspectInput, "Inspect input"),
                workflow_step("plan", WorkflowStepKind::Plan, "Plan work"),
                workflow_step("review", WorkflowStepKind::Review, "Review output"),
            ],
        )
        .expect("test workflow plan should be valid")
    }

    fn all_states() -> [RunState; 9] {
        [
            RunState::Created,
            RunState::Planning,
            RunState::WaitingForApproval,
            RunState::RunningModel,
            RunState::RunningTool,
            RunState::Reviewing,
            RunState::Completed,
            RunState::Failed,
            RunState::Canceled,
        ]
    }

    #[test]
    fn created_can_transition_to_planning() {
        assert!(RunState::Created.can_transition_to(RunState::Planning));
    }

    #[test]
    fn created_can_transition_to_canceled() {
        assert!(RunState::Created.can_transition_to(RunState::Canceled));
    }

    #[test]
    fn planning_can_transition_to_running_model() {
        assert!(RunState::Planning.can_transition_to(RunState::RunningModel));
    }

    #[test]
    fn running_model_can_transition_to_running_tool() {
        assert!(RunState::RunningModel.can_transition_to(RunState::RunningTool));
    }

    #[test]
    fn reviewing_can_transition_to_completed() {
        assert!(RunState::Reviewing.can_transition_to(RunState::Completed));
    }

    #[test]
    fn completed_cannot_transition_to_any_other_state() {
        for state in all_states() {
            assert!(!RunState::Completed.can_transition_to(state));
        }
    }

    #[test]
    fn failed_cannot_transition_to_any_other_state() {
        for state in all_states() {
            assert!(!RunState::Failed.can_transition_to(state));
        }
    }

    #[test]
    fn canceled_cannot_transition_to_any_other_state() {
        for state in all_states() {
            assert!(!RunState::Canceled.can_transition_to(state));
        }
    }

    #[test]
    fn terminal_state_helper_marks_only_terminal_states() {
        for state in all_states() {
            assert_eq!(
                state.is_terminal(),
                matches!(
                    state,
                    RunState::Completed | RunState::Failed | RunState::Canceled
                )
            );
        }
    }

    #[test]
    fn transition_to_updates_state_on_valid_transition() {
        let mut run = AgentRun::new(ThreadId::new());

        let result = run.transition_to(RunState::Planning);

        assert!(result.is_ok());
        assert_eq!(run.state(), RunState::Planning);
    }

    #[test]
    fn transition_to_preserves_state_on_invalid_transition() {
        let mut run = AgentRun::new(ThreadId::new());

        let result = run.transition_to(RunState::RunningTool);

        assert_eq!(
            result,
            Err(CoreError::InvalidRunTransition {
                from: RunState::Created,
                to: RunState::RunningTool,
            })
        );
        assert_eq!(run.state(), RunState::Created);
    }

    #[test]
    fn valid_transition_can_produce_state_change_event() {
        let mut run = AgentRun::new(ThreadId::new());
        let run_id = run.id();

        let result = run.transition_to_with_event(RunState::Planning);

        assert_eq!(run.state(), RunState::Planning);
        assert_eq!(
            result,
            Ok(crate::CoreEvent::RunStateChanged {
                run_id,
                from: RunState::Created,
                to: RunState::Planning,
            })
        );
    }

    #[test]
    fn invalid_transition_does_not_create_event_or_mutate_state() {
        let mut run = AgentRun::new(ThreadId::new());

        let result = run.transition_to_with_event(RunState::RunningTool);

        assert_eq!(
            result,
            Err(CoreError::InvalidRunTransition {
                from: RunState::Created,
                to: RunState::RunningTool,
            })
        );
        assert_eq!(run.state(), RunState::Created);
    }

    #[test]
    fn terminal_state_transition_attempt_does_not_create_event() {
        let mut run = AgentRun::new(ThreadId::new());
        let completed = run.transition_to_with_event(RunState::Canceled);

        assert!(completed.is_ok());

        let result = run.transition_to_with_event(RunState::Planning);

        assert_eq!(
            result,
            Err(CoreError::InvalidRunTransition {
                from: RunState::Canceled,
                to: RunState::Planning,
            })
        );
        assert_eq!(run.state(), RunState::Canceled);
    }

    #[test]
    fn workflow_plan_can_prepare_run_skeleton() {
        let run_id = fixed_run_id();
        let thread_id = fixed_thread_id();
        let plan = workflow_plan();

        let run = AgentRun::prepare_from_plan(run_id, thread_id, &plan);

        assert_eq!(run.id(), run_id);
        assert_eq!(run.thread_id(), thread_id);
        assert_eq!(run.steps().len(), plan.step_count());
    }

    #[test]
    fn prepared_run_starts_in_created_state() {
        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &workflow_plan());

        assert_eq!(run.state(), RunState::Created);
    }

    #[test]
    fn prepared_steps_preserve_workflow_step_order() {
        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &workflow_plan());
        let labels: Vec<_> = run.steps().iter().map(|step| step.title()).collect();

        assert_eq!(labels, vec!["Inspect input", "Plan work", "Review output"]);
    }

    #[test]
    fn prepared_steps_preserve_workflow_step_keys() {
        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &workflow_plan());
        let keys: Vec<_> = run
            .steps()
            .iter()
            .map(|step| step.workflow_step_key().map(WorkflowStepKey::as_str))
            .collect();

        assert_eq!(keys, vec![Some("inspect"), Some("plan"), Some("review")]);
    }

    #[test]
    fn repeated_preparation_with_same_inputs_is_deterministic() {
        let run_id = fixed_run_id();
        let thread_id = fixed_thread_id();
        let plan = workflow_plan();

        let first = AgentRun::prepare_from_plan(run_id, thread_id, &plan);
        let second = AgentRun::prepare_from_plan(run_id, thread_id, &plan);

        assert_eq!(first, second);
    }

    #[test]
    fn preparing_run_skeleton_does_not_execute_steps() {
        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &workflow_plan());

        assert!(run
            .steps()
            .iter()
            .all(|step| step.state() == RunState::Created));
    }

    #[test]
    fn preparing_run_skeleton_does_not_emit_events() {
        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &workflow_plan());
        let events = [];

        assert_eq!(
            RunInspection::from_events(run.id(), &events),
            Err(CoreError::MissingRunCreated { run_id: run.id() })
        );
    }

    #[test]
    fn preparing_run_skeleton_has_no_domain_or_runtime_resource_requirement() {
        let plan = WorkflowPlan::new(
            workflow_id("mixed-stem"),
            "Mixed STEM workflow",
            vec![
                workflow_step("inspect", WorkflowStepKind::InspectInput, "Inspect input"),
                workflow_step("model", WorkflowStepKind::Model, "Draft analysis"),
                workflow_step("tool", WorkflowStepKind::Tool, "Plan tool use"),
                workflow_step("custom", WorkflowStepKind::Custom, "Domain pack step"),
            ],
        )
        .expect("test workflow plan should be valid");

        let run = AgentRun::prepare_from_plan(fixed_run_id(), fixed_thread_id(), &plan);

        assert_eq!(run.steps().len(), 4);
        assert!(run
            .steps()
            .iter()
            .all(|step| step.state() == RunState::Created));
    }
}
