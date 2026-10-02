use bilicatch_lib::services::download::{download_percent, DownloadControl, DownloadControlState};

#[test]
fn download_progress_uses_only_zero_to_ninety_percent() {
    assert_eq!(download_percent(50, Some(100), 0), 45);
    assert_eq!(download_percent(100, Some(100), 45), 90);
    assert_eq!(download_percent(200, Some(100), 80), 90);
    assert_eq!(download_percent(50, None, 37), 37);
    assert_eq!(download_percent(1, Some(0), 12), 12);
}

#[test]
fn download_progress_does_not_regress_when_the_known_total_expands() {
    assert_eq!(download_percent(1_014_439, Some(3_124_379), 60), 60);
}

#[test]
fn download_control_is_cloneable_and_cancel_wins_over_pause() {
    let control = DownloadControl::new();
    let observer = control.clone();

    control.request_pause();
    assert_eq!(observer.state(), DownloadControlState::PauseRequested);

    observer.request_cancel();
    assert_eq!(control.state(), DownloadControlState::CancelRequested);
    control.request_pause();
    assert_eq!(observer.state(), DownloadControlState::CancelRequested);
}
