//! Run-level inspection projections for in-memory core events.

use crate::{CoreError, CoreEvent, CoreEventEnvelope, EventSequence, RunId, RunState};

/// Deterministic summary produced by projecting one run's in-memory events.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunInspection {
    run_id: RunId,
    event_count: usize,
    state_transition_count: usize,
    current_state: RunState,
    is_terminal: bool,
}

impl RunInspection {
    /// Projects an in-memory event stream into a compact run summary.
    ///
    /// The stream must begin with `RunCreated` at sequence `0`, and every
    /// following event must use the next consecutive sequence number. Mixed-run
    /// streams and impossible lifecycle transitions are rejected.
    pub fn from_events(run_id: RunId, events: &[CoreEventEnvelope]) -> Result<Self, CoreError> {
        if events.is_empty() {
            return Err(CoreError::MissingRunCreated { run_id });
        }

        let mut current_state = RunState::Created;
        let mut state_transition_count = 0;

        for (index, envelope) in events.iter().enumerate() {
            let expected = EventSequence::new(index as u64);
            if envelope.sequence() != expected {
                return Err(CoreError::EventSequenceOutOfOrder {
                    expected,
                    found: envelope.sequence(),
                });
            }

            match envelope.event() {
                CoreEvent::RunCreated {
                    run_id: event_run_id,
                    ..
                } => {
                    validate_run_id(run_id, *event_run_id)?;
                    if index != 0 {
                        return Err(CoreError::UnexpectedRunInspectionEvent {
                            run_id,
                            event_kind: event_kind(envelope.event()),
                        });
                    }
                }
                _ if index == 0 => return Err(CoreError::MissingRunCreated { run_id }),
                CoreEvent::RunStateChanged {
                    run_id: event_run_id,
                    from,
                    to,
                } => {
                    validate_run_id(run_id, *event_run_id)?;
                    if current_state != *from || !from.can_transition_to(*to) {
                        return Err(CoreError::InvalidReplayTransition {
                            current: current_state,
                            event_from: *from,
                            event_to: *to,
                        });
                    }

                    current_state = *to;
                    state_transition_count += 1;
                }
                CoreEvent::StepRecorded {
                    run_id: event_run_id,
                    ..
                } => validate_run_id(run_id, *event_run_id)?,
                event => {
                    return Err(CoreError::UnexpectedRunInspectionEvent {
                        run_id,
                        event_kind: event_kind(event),
                    });
                }
            }
        }

        Ok(Self {
            run_id,
            event_count: events.len(),
            state_transition_count,
            current_state,
            is_terminal: current_state.is_terminal(),
        })
    }

    /// Returns the inspected run identifier.
    pub fn run_id(&self) -> RunId {
        self.run_id
    }

    /// Returns the number of projected events.
    pub fn event_count(&self) -> usize {
        self.event_count
    }

    /// Returns how many run lifecycle transition events were applied.
    pub fn state_transition_count(&self) -> usize {
        self.state_transition_count
    }

    /// Returns the final projected run state.
    pub fn current_state(&self) -> RunState {
        self.current_state
    }

    /// Returns whether the final projected state is terminal.
    pub fn is_terminal(&self) -> bool {
        self.is_terminal
    }
}

fn validate_run_id(expected: RunId, found: RunId) -> Result<(), CoreError> {
    if expected != found {
        return Err(CoreError::EventRunIdMismatch { expected, found });
    }

    Ok(())
}

fn event_kind(event: &CoreEvent) -> &'static str {
    match event {
        CoreEvent::ProjectCreated { .. } => "ProjectCreated",
        CoreEvent::ThreadCreated { .. } => "ThreadCreated",
        CoreEvent::RunCreated { .. } => "RunCreated",
        CoreEvent::RunStateChanged { .. } => "RunStateChanged",
        CoreEvent::StepRecorded { .. } => "StepRecorded",
        CoreEvent::ArtifactRecorded { .. } => "ArtifactRecorded",
    }
}

#[cfg(test)]
mod tests {
    use super::RunInspection;
    use crate::{
        CoreError, CoreEvent, CoreEventEnvelope, EventSequence, RunId, RunState, ThreadId,
    };
    use uuid::Uuid;

    fn run_id(value: u128) -> RunId {
        RunId::from_uuid(Uuid::from_u128(value))
    }

    fn thread_id(value: u128) -> ThreadId {
        ThreadId::from_uuid(Uuid::from_u128(value))
    }

    fn envelope(sequence: u64, event: CoreEvent) -> CoreEventEnvelope {
        CoreEventEnvelope::new(EventSequence::new(sequence), event)
    }

    fn run_created(sequence: u64, run_id: RunId) -> CoreEventEnvelope {
        envelope(
            sequence,
            CoreEvent::RunCreated {
                thread_id: thread_id(1),
                run_id,
            },
        )
    }

    fn state_changed(
        sequence: u64,
        run_id: RunId,
        from: RunState,
        to: RunState,
    ) -> CoreEventEnvelope {
        envelope(sequence, CoreEvent::RunStateChanged { run_id, from, to })
    }

    #[test]
    fn inspecting_valid_run_events_returns_final_state_and_counts() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(1, run_id, RunState::Created, RunState::Planning),
            state_changed(2, run_id, RunState::Planning, RunState::RunningModel),
            state_changed(3, run_id, RunState::RunningModel, RunState::Completed),
        ];

        let inspection = RunInspection::from_events(run_id, &events)
            .expect("valid event stream should inspect cleanly");

        assert_eq!(inspection.run_id(), run_id);
        assert_eq!(inspection.event_count(), 4);
        assert_eq!(inspection.state_transition_count(), 3);
        assert_eq!(inspection.current_state(), RunState::Completed);
        assert!(inspection.is_terminal());
    }

    #[test]
    fn inspecting_non_terminal_run_reports_non_terminal_state() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(1, run_id, RunState::Created, RunState::Planning),
        ];

        let inspection = RunInspection::from_events(run_id, &events)
            .expect("valid event stream should inspect cleanly");

        assert_eq!(inspection.current_state(), RunState::Planning);
        assert!(!inspection.is_terminal());
    }

    #[test]
    fn out_of_order_event_sequence_returns_structured_error() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(2, run_id, RunState::Created, RunState::Planning),
        ];

        let result = RunInspection::from_events(run_id, &events);

        assert_eq!(
            result,
            Err(CoreError::EventSequenceOutOfOrder {
                expected: EventSequence::new(1),
                found: EventSequence::new(2),
            })
        );
    }

    #[test]
    fn mixed_run_event_stream_returns_structured_error() {
        let expected_run_id = run_id(1);
        let other_run_id = run_id(2);
        let events = [
            run_created(0, expected_run_id),
            state_changed(1, other_run_id, RunState::Created, RunState::Planning),
        ];

        let result = RunInspection::from_events(expected_run_id, &events);

        assert_eq!(
            result,
            Err(CoreError::EventRunIdMismatch {
                expected: expected_run_id,
                found: other_run_id,
            })
        );
    }

    #[test]
    fn invalid_lifecycle_transition_inside_events_returns_structured_error() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(1, run_id, RunState::Created, RunState::RunningTool),
        ];

        let result = RunInspection::from_events(run_id, &events);

        assert_eq!(
            result,
            Err(CoreError::InvalidReplayTransition {
                current: RunState::Created,
                event_from: RunState::Created,
                event_to: RunState::RunningTool,
            })
        );
    }

    #[test]
    fn missing_run_created_returns_structured_error() {
        let run_id = run_id(1);
        let events = [state_changed(
            0,
            run_id,
            RunState::Created,
            RunState::Planning,
        )];

        let result = RunInspection::from_events(run_id, &events);

        assert_eq!(result, Err(CoreError::MissingRunCreated { run_id }));
    }

    #[test]
    fn projection_is_deterministic_across_repeated_calls() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(1, run_id, RunState::Created, RunState::Planning),
            state_changed(2, run_id, RunState::Planning, RunState::Canceled),
        ];

        let first = RunInspection::from_events(run_id, &events)
            .expect("valid event stream should inspect cleanly");
        let second = RunInspection::from_events(run_id, &events)
            .expect("valid event stream should inspect cleanly");

        assert_eq!(first, second);
    }

    #[test]
    fn projection_does_not_mutate_input_events() {
        let run_id = run_id(1);
        let events = [
            run_created(0, run_id),
            state_changed(1, run_id, RunState::Created, RunState::Planning),
        ];
        let original = events.clone();

        let _inspection = RunInspection::from_events(run_id, &events)
            .expect("valid event stream should inspect cleanly");

        assert_eq!(events, original);
    }
}
