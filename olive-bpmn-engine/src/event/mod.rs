
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ProcessEvent {
    /// Process has started
    Start,
    /// Process has ended
    End,
    /// None Event was thrown
    NoneEvent,
    /// Signal Event
    SignalEvent { signal_ref: Option<String> },
    /// Cancel Event
    CancelEvent,
    /// Terminate Event
    TerminateEvent,
    /// Compensation Event
    CompensationEvent { activity_ref: Option<String> },
    /// Message Event
    MessageEvent {
        message_ref: Option<String>,
        operation_ref: Option<String>,
    },
    /// Escalation Event
    EscalationEvent { escalation_ref: Option<String> },
    /// Link Event
    LinkEvent {
        sources: Vec<String>,
        target: Option<String>,
    },
    /// Error Event
    ErrorEvent { error_ref: Option<String> },
}