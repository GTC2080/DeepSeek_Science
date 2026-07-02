//! Agent run and step state for replayable kernel execution.

use crate::{CoreError, RunId, StepId, ThreadId};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Lifecycle state for an agent run or run step.
///
/// The state machine is deliberately small in Phase 1. It is enough to model
/// planning, execution, approvals, and terminal states without embedding domain
/// workflow details in the core crate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum RunState {
    /// The run has been created but no plan has been produced.
    Created,
    /// The agent is preparing a plan or prompt context.
    Planning,
    /// The agent is actively executing model or tool work.
    Running,
    /// The run is paused until a user approves a risky action.
    WaitingForApproval,
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
                | (Self::Planning, Self::Running)
                | (Self::Running, Self::WaitingForApproval)
                | (Self::WaitingForApproval, Self::Running)
                | (Self::Running, Self::Completed)
                | (Self::Running, Self::Failed)
                | (Self::Planning, Self::Failed)
                | (Self::Created, Self::Canceled)
                | (Self::Planning, Self::Canceled)
                | (Self::Running, Self::Canceled)
                | (Self::WaitingForApproval, Self::Canceled)
        )
    }
}

impl fmt::Display for RunState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::Created => "created",
            Self::Planning => "planning",
            Self::Running => "running",
            Self::WaitingForApproval => "waiting_for_approval",
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
    use crate::ThreadId;

    #[test]
    fn run_state_can_transition_from_created_to_planning() {
        let mut run = AgentRun::new(ThreadId::new());

        let result = run.transition_to(RunState::Planning);

        assert!(result.is_ok());
        assert_eq!(run.state(), RunState::Planning);
    }
}
