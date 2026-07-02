//! Agent run and step state for replayable kernel execution.

use crate::{CoreError, RunId, StepId, ThreadId};
use serde::{Deserialize, Serialize};
use std::fmt;

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
        })
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
}

#[cfg(test)]
mod tests {
    use super::{AgentRun, RunState};
    use crate::{CoreError, ThreadId};

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
}
