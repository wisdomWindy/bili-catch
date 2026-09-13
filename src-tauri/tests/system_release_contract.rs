use bilicatch_lib::services::system::{
    map_task_notification, validate_release_set, ReleaseArtifact, SidecarManifest,
    SystemNotification, UpdateMachine, UpdateState, WindowDecision,
};

#[test]
fn update_machine_rejects_install_before_verified_confirmation() {
    let mut machine = UpdateMachine::default();
    assert_eq!(machine.start_check(), UpdateState::Checking);
    assert_eq!(
        machine.finish_available("0.2.0", "windows", true),
        UpdateState::Available
    );
    assert_eq!(machine.begin_install(false), UpdateState::Available);
    assert_eq!(machine.begin_install(true), UpdateState::Installing);
}

#[test]
fn system_contract_validates_sidecar_and_release_artifact_fields() {
    let sidecar = SidecarManifest {
        name: "ffmpeg".into(),
        platform: "windows-x86_64".into(),
        sha256: "a".repeat(64),
        license_path: "licenses/ffmpeg.txt".into(),
    };
    assert!(sidecar.validate().is_ok());
    assert!(SidecarManifest {
        sha256: "bad".into(),
        ..sidecar.clone()
    }
    .validate()
    .is_err());

    let artifact = ReleaseArtifact {
        platform: "windows".into(),
        target: "nsis".into(),
        version: "0.1.0".into(),
        signature_path: "latest.sig".into(),
    };
    assert!(artifact.validate().is_ok());
    assert!(validate_release_set(&[artifact.clone()]).is_ok());
    assert!(validate_release_set(&[]).is_err());
    assert!(validate_release_set(&[artifact.clone(), artifact]).is_err());
}

#[test]
fn close_and_notification_policies_are_stable_and_private() {
    assert_eq!(
        WindowDecision::from_settings(false, true),
        WindowDecision::Exit
    );
    assert_eq!(
        WindowDecision::from_settings(true, true),
        WindowDecision::ConfirmExit
    );
    assert_eq!(
        map_task_notification("Completed", "episode.mp4"),
        SystemNotification::TaskCompleted {
            file_name: "episode.mp4".into()
        }
    );
}

#[test]
fn tray_uses_the_default_application_icon() {
    let app_source = include_str!("../src/lib.rs");
    let tray_builder = app_source
        .split("tauri::tray::TrayIconBuilder::new()")
        .nth(1)
        .and_then(|source| source.split(".build(app)?").next())
        .expect("tray builder source should be present");

    assert!(
        tray_builder.contains(".icon(") && tray_builder.contains("app.default_window_icon()"),
        "the tray icon must reuse Tauri's bundled application icon"
    );
}
