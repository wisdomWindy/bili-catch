use bilicatch_lib::models::{AuthAccount, AuthSnapshot, AuthStateEvent, AuthStatus};
use serde_json::json;

#[test]
fn auth_snapshot_serializes_without_private_credentials() {
    let snapshot = AuthSnapshot {
        revision: 7,
        status: AuthStatus::Authenticated,
        qr_content: None,
        expires_at: None,
        account: Some(AuthAccount {
            mid: Some("9007199254740993".into()),
            name: "测试账号".into(),
            avatar_url: Some("https://i0.hdslb.com/bfs/face/example.jpg".into()),
        }),
        error: None,
    };

    assert_eq!(
        serde_json::to_value(AuthStateEvent { snapshot }).expect("auth event should serialize"),
        json!({
            "snapshot": {
                "revision": 7,
                "status": "authenticated",
                "qrContent": null,
                "expiresAt": null,
                "account": {
                    "mid": "9007199254740993",
                    "name": "测试账号",
                    "avatarUrl": "https://i0.hdslb.com/bfs/face/example.jpg"
                },
                "error": null
            }
        })
    );
}

#[test]
fn auth_statuses_use_the_stable_wire_values() {
    assert_eq!(
        serde_json::to_value([
            AuthStatus::Restoring,
            AuthStatus::Anonymous,
            AuthStatus::Requesting,
            AuthStatus::WaitingScan,
            AuthStatus::WaitingConfirm,
            AuthStatus::Authenticated,
            AuthStatus::Expired,
            AuthStatus::Cancelled,
            AuthStatus::Error,
        ])
        .expect("auth statuses should serialize"),
        json!([
            "restoring",
            "anonymous",
            "requesting",
            "waiting_scan",
            "waiting_confirm",
            "authenticated",
            "expired",
            "cancelled",
            "error"
        ])
    );
}
