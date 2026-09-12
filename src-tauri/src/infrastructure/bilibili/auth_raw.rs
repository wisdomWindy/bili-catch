use serde::Deserialize;

use crate::models::{AppError, AppErrorCode, AuthAccount};

use super::raw::ApiResponse;

#[derive(Deserialize)]
pub(super) struct RawQrGenerateData {
    pub(super) url: String,
    pub(super) qrcode_key: String,
}

#[derive(Deserialize)]
pub(super) struct RawQrPollData {
    pub(super) code: i64,
    #[serde(default)]
    pub(super) refresh_token: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RawAuthNavData {
    pub(super) is_login: bool,
    pub(super) mid: Option<u64>,
    pub(super) uname: Option<String>,
    pub(super) face: Option<String>,
}

pub(super) struct ParsedGeneratedQr {
    pub(super) qr_content: String,
    pub(super) qrcode_key: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum QrPollState {
    WaitingScan,
    WaitingConfirm,
    Confirmed,
    Expired,
}

pub(super) struct ParsedPoll {
    pub(super) state: QrPollState,
    pub(super) refresh_token: Option<String>,
}

pub(super) struct ParsedNav {
    pub(super) account: Option<AuthAccount>,
}

fn unsupported_response() -> AppError {
    AppError::internal("Bilibili returned an unsupported authentication response")
}

fn rejected_response() -> AppError {
    AppError::new(
        AppErrorCode::E004,
        "Bilibili rejected the authentication request",
    )
}

fn decode<T: for<'de> Deserialize<'de>>(body: &[u8]) -> Result<ApiResponse<T>, AppError> {
    serde_json::from_slice(body).map_err(|_| unsupported_response())
}

pub(super) fn parse_generate_response(body: &[u8]) -> Result<ParsedGeneratedQr, AppError> {
    let envelope: ApiResponse<RawQrGenerateData> = decode(body)?;
    if envelope.code != 0 {
        return Err(rejected_response());
    }
    let data = envelope.data.ok_or_else(unsupported_response)?;
    if !is_bilibili_https_url(&data.url) || data.qrcode_key.trim().is_empty() {
        return Err(unsupported_response());
    }
    Ok(ParsedGeneratedQr {
        qr_content: data.url,
        qrcode_key: data.qrcode_key,
    })
}

pub(super) fn is_bilibili_https_url(value: &str) -> bool {
    let Ok(parsed) = url::Url::parse(value) else {
        return false;
    };
    let Some(host) = parsed.host_str() else {
        return false;
    };
    parsed.scheme() == "https" && (host == "bilibili.com" || host.ends_with(".bilibili.com"))
}

pub(super) fn parse_poll_response(body: &[u8]) -> Result<ParsedPoll, AppError> {
    let envelope: ApiResponse<RawQrPollData> = decode(body)?;
    if envelope.code != 0 {
        return Err(rejected_response());
    }
    let data = envelope.data.ok_or_else(unsupported_response)?;
    let state = match data.code {
        86101 => QrPollState::WaitingScan,
        86090 => QrPollState::WaitingConfirm,
        0 => QrPollState::Confirmed,
        86038 => QrPollState::Expired,
        _ => return Err(unsupported_response()),
    };
    Ok(ParsedPoll {
        state,
        refresh_token: data.refresh_token,
    })
}

pub(super) fn parse_nav_response(body: &[u8]) -> Result<ParsedNav, AppError> {
    let envelope: ApiResponse<RawAuthNavData> = decode(body)?;
    if envelope.code == -101 {
        return Ok(ParsedNav { account: None });
    }
    if envelope.code != 0 {
        return Err(rejected_response());
    }
    let data = envelope.data.ok_or_else(unsupported_response)?;
    if !data.is_login {
        return Ok(ParsedNav { account: None });
    }
    Ok(ParsedNav {
        account: Some(AuthAccount {
            mid: data.mid.map(|mid| mid.to_string()),
            name: data
                .uname
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "Bilibili account".into()),
            avatar_url: data.face.and_then(allowed_avatar_url),
        }),
    })
}

fn allowed_avatar_url(value: String) -> Option<String> {
    let parsed = url::Url::parse(&value).ok()?;
    let host = parsed.host_str()?;
    let allowed_host = ["hdslb.com", "biliimg.com"]
        .iter()
        .any(|domain| host == *domain || host.ends_with(&format!(".{domain}")));
    (parsed.scheme() == "https" && allowed_host).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{parse_generate_response, parse_nav_response, parse_poll_response, QrPollState};

    #[test]
    fn parses_generate_fixture_into_ephemeral_fields() {
        let generated = parse_generate_response(include_bytes!(
            "../../../tests/fixtures/auth/qr-generate.json"
        ))
        .expect("fixture should parse");

        assert_eq!(
            generated.qr_content,
            "https://passport.bilibili.com/h5-app/passport/login/scan?fixture=1"
        );
        assert_eq!(generated.qrcode_key, "fixture-key");
    }

    #[test]
    fn maps_all_documented_poll_codes_and_rejects_unknown_codes() {
        let cases = [
            (86101, QrPollState::WaitingScan),
            (86090, QrPollState::WaitingConfirm),
            (86038, QrPollState::Expired),
        ];
        for (code, expected) in cases {
            let body = format!(
                r#"{{"code":0,"message":"0","data":{{"code":{code},"message":"fixture"}}}}"#
            );
            assert_eq!(
                parse_poll_response(body.as_bytes()).unwrap().state,
                expected
            );
        }

        let confirmed = parse_poll_response(include_bytes!(
            "../../../tests/fixtures/auth/qr-poll-confirmed.json"
        ))
        .expect("confirmed fixture should parse");
        assert_eq!(confirmed.state, QrPollState::Confirmed);
        assert_eq!(confirmed.refresh_token.as_deref(), Some("fixture-refresh"));

        let unknown = br#"{"code":0,"message":"0","data":{"code":12345,"message":"new state"}}"#;
        let error = parse_poll_response(unknown)
            .err()
            .expect("unknown codes must fail closed");
        assert_eq!(serde_json::to_value(error).unwrap()["code"], "E_INTERNAL");
    }

    #[test]
    fn validates_nav_account_and_explicit_anonymous_state() {
        let valid = parse_nav_response(include_bytes!(
            "../../../tests/fixtures/auth/nav-authenticated.json"
        ))
        .expect("valid nav fixture should parse");
        let account = valid.account.expect("account should be present");
        assert_eq!(account.mid.as_deref(), Some("9007199254740993"));
        assert_eq!(account.name, "测试账号");

        let invalid = parse_nav_response(br#"{"code":0,"message":"0","data":{"isLogin":false}}"#)
            .expect("anonymous nav should be a stable result");
        assert!(invalid.account.is_none());
    }

    #[test]
    fn maps_the_live_anonymous_nav_code_to_an_invalid_session() {
        let body =
            br#"{"code":-101,"message":"account not signed in","ttl":1,"data":{"isLogin":false}}"#;

        let invalid = parse_nav_response(body)
            .expect("the documented anonymous response must not be treated as transient");

        assert!(invalid.account.is_none());
    }

    #[test]
    fn rejects_avatar_hosts_outside_the_approved_image_domains() {
        let body = br#"{"code":0,"message":"0","data":{"isLogin":true,"mid":9001,"uname":"fixture","face":"https://www.bilibili.com/avatar.png"}}"#;

        let account = parse_nav_response(body)
            .expect("the account response should parse")
            .account
            .expect("the account should remain authenticated");

        assert!(account.avatar_url.is_none());
    }

    #[test]
    fn rejects_missing_generate_fields_without_echoing_the_body() {
        let body = br#"{"code":0,"message":"0","data":{"url":"secret-query-value"}}"#;
        let error = parse_generate_response(body)
            .err()
            .expect("missing key must fail");
        let serialized = serde_json::to_string(&error).unwrap();
        assert!(!serialized.contains("secret-query-value"));
    }

    #[test]
    fn rejects_a_qr_destination_outside_bilibili_https_hosts() {
        let body = br#"{"code":0,"message":"0","data":{"url":"https://example.org/login","qrcode_key":"fixture-key"}}"#;
        assert!(parse_generate_response(body).is_err());

        let insecure = br#"{"code":0,"message":"0","data":{"url":"http://passport.bilibili.com/login","qrcode_key":"fixture-key"}}"#;
        assert!(parse_generate_response(insecure).is_err());
    }
}
