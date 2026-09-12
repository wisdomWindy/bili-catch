use bilicatch_lib::services::{download::DownloadProgress, video::aggregate_video_progress};

#[test]
fn aggregates_dual_track_bytes_and_requires_both_totals_for_eta() {
    let video = DownloadProgress {
        downloaded_bytes: 60,
        total_bytes: Some(100),
        speed_bytes_per_second: 10,
        eta_seconds: Some(4),
    };
    let audio = DownloadProgress {
        downloaded_bytes: 20,
        total_bytes: None,
        speed_bytes_per_second: 5,
        eta_seconds: None,
    };

    let aggregate = aggregate_video_progress(&video, Some(&audio));

    assert_eq!(aggregate.downloaded_bytes, 80);
    assert_eq!(aggregate.total_bytes, None);
    assert_eq!(aggregate.speed_bytes_per_second, 15);
    assert_eq!(aggregate.eta_seconds, None);
}

#[test]
fn single_track_progress_is_preserved_for_video_only() {
    let video = DownloadProgress {
        downloaded_bytes: 90,
        total_bytes: Some(100),
        speed_bytes_per_second: 10,
        eta_seconds: Some(1),
    };

    assert_eq!(aggregate_video_progress(&video, None), video);
}
