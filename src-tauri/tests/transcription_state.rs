use bilicatch_lib::{
    models::TranscribeTaskStatus,
    services::transcription::state_machine::{can_retry, can_transition, is_terminal},
};

#[test]
fn state_machine_accepts_only_the_approved_forward_transitions() {
    use TranscribeTaskStatus::*;
    let allowed = [
        (Pending, Preprocessing),
        (Pending, Cancelled),
        (Pending, Failed),
        (Preprocessing, LoadingModel),
        (Preprocessing, Cancelled),
        (Preprocessing, Failed),
        (LoadingModel, Transcribing),
        (LoadingModel, Cancelled),
        (LoadingModel, Failed),
        (Transcribing, Processing),
        (Transcribing, Cancelled),
        (Transcribing, Failed),
        (Processing, Saving),
        (Processing, Cancelled),
        (Processing, Failed),
        (Saving, Completed),
        (Saving, Failed),
    ];

    for (from, to) in allowed {
        assert!(
            can_transition(from, to),
            "{from:?} -> {to:?} should be allowed"
        );
    }
    assert!(!can_transition(Saving, Cancelled));
    assert!(!can_transition(Completed, Pending));
    assert!(!can_transition(Failed, Pending));
    assert!(can_retry(Failed));
    assert!(can_retry(Cancelled));
    assert!(!can_retry(Completed));
    assert!(is_terminal(Completed));
    assert!(is_terminal(Failed));
    assert!(is_terminal(Cancelled));
}
