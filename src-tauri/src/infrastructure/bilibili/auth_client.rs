use std::time::Duration;

use async_trait::async_trait;
use reqwest::{
    header::{HeaderMap, COOKIE, SET_COOKIE},
    redirect::Policy,
    Client, Response,
};
use secrecy::{ExposeSecret, SecretString};

use crate::{
    models::{AppError, AppErrorCode},
    services::auth::ports::{
        GeneratedQr, QrAuthPort, QrPollResult, SessionValidation, StoredCredential,
    },
};

use super::{
    auth_raw::{
        is_bilibili_https_url, parse_generate_response, parse_nav_response, parse_poll_response,
        QrPollState,
    },
    client::{map_reqwest_error, read_bounded_body},
};

const QR_GENERATE_URL: &str =
    "https://passport.bilibili.com/x/passport-login/web/qrcode/generate?source=main_web";
const QR_POLL_URL: &str = "https://passport.bilibili.com/x/passport-login/web/qrcode/poll";
const NAV_URL: &str = "https://api.bilibili.com/x/web-interface/nav";
const ALLOWED_COOKIES: [&str; 5] = [
    "DedeUserID",
    "DedeUserID__ckMd5",
    "SESSDATA",
    "bili_jct",
    "sid",
];

pub(crate) struct BilibiliAuthClient {
    client: Client,
}

impl BilibiliAuthClient {
    pub(crate) fn new() -> Result<Self, AppError> {
        let client = Client::builder()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(20))
            .user_agent("Mozilla/5.0 BiliCatch/0.1")
            .build()
            .map_err(|_| AppError::internal("Unable to initialize the authentication client"))?;
        Ok(Self { client })
    }
}

fn response_status_error() -> AppError {
    AppError::new(
        AppErrorCode::E001,
        "Bilibili authentication is temporarily unavailable",
    )
}

async fn read_auth_response(response: Response) -> Result<(HeaderMap, Vec<u8>), AppError> {
    if !response.status().is_success() {
        return Err(response_status_error());
    }
    let headers = response.headers().clone();
    let body = read_bounded_body(response).await?;
    Ok((headers, body))
}

pub(super) fn validated_cookie_url(value: &str) -> Result<url::Url, AppError> {
    if !is_bilibili_https_url(value) {
        return Err(AppError::internal(
            "The authenticated Bilibili endpoint is invalid",
        ));
    }
    url::Url::parse(value)
        .map_err(|_| AppError::internal("The authenticated Bilibili endpoint is invalid"))
}

pub(super) fn extract_credential(
    headers: &HeaderMap,
    refresh_token: Option<&str>,
) -> Result<StoredCredential, AppError> {
    let mut values = std::collections::BTreeMap::<&str, String>::new();
    for header in headers.get_all(SET_COOKIE) {
        let Ok(value) = header.to_str() else {
            continue;
        };
        let Some((name, cookie_value)) = value
            .split(';')
            .next()
            .and_then(|item| item.split_once('='))
        else {
            continue;
        };
        let name = name.trim();
        let cookie_value = cookie_value.trim();
        if !cookie_value.is_empty() && ALLOWED_COOKIES.contains(&name) {
            values.insert(name, cookie_value.to_owned());
        }
    }
    let cookie_header = values
        .into_iter()
        .map(|(name, value)| format!("{name}={value}"))
        .collect::<Vec<_>>()
        .join("; ");
    StoredCredential::try_new(cookie_header, refresh_token.map(str::to_owned))
}

#[async_trait]
impl QrAuthPort for BilibiliAuthClient {
    async fn generate_qr(&self) -> Result<GeneratedQr, AppError> {
        let response = self
            .client
            .get(QR_GENERATE_URL)
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let (_, body) = read_auth_response(response).await?;
        let generated = parse_generate_response(&body)?;
        Ok(GeneratedQr {
            qr_content: generated.qr_content,
            qrcode_key: generated.qrcode_key.into(),
        })
    }

    async fn poll_qr(&self, qrcode_key: &SecretString) -> Result<QrPollResult, AppError> {
        let response = self
            .client
            .get(QR_POLL_URL)
            .query(&[("qrcode_key", qrcode_key.expose_secret())])
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let (headers, body) = read_auth_response(response).await?;
        let poll = parse_poll_response(&body)?;
        Ok(match poll.state {
            QrPollState::WaitingScan => QrPollResult::WaitingScan,
            QrPollState::WaitingConfirm => QrPollResult::WaitingConfirm,
            QrPollState::Expired => QrPollResult::Expired,
            QrPollState::Confirmed => QrPollResult::Confirmed(extract_credential(
                &headers,
                poll.refresh_token.as_deref(),
            )?),
        })
    }

    async fn validate_session(
        &self,
        credential: &StoredCredential,
    ) -> Result<SessionValidation, AppError> {
        let response = self
            .client
            .get(validated_cookie_url(NAV_URL)?)
            .header(COOKIE, credential.expose_cookie_header())
            .send()
            .await
            .map_err(map_reqwest_error)?;
        let (_, body) = read_auth_response(response).await?;
        Ok(match parse_nav_response(&body)?.account {
            Some(account) => SessionValidation::Valid(account),
            None => SessionValidation::Invalid,
        })
    }
}

#[cfg(test)]
mod tests {
    use reqwest::header::{HeaderMap, HeaderValue, SET_COOKIE};

    use super::{extract_credential, validated_cookie_url};

    #[test]
    fn extracts_only_allowlisted_cookie_names() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("SESSDATA=fixture-session; Path=/; Secure; HttpOnly"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("DedeUserID=9001; Path=/; Secure"),
        );
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("unrelated=must-not-survive; Path=/"),
        );

        let credential = extract_credential(&headers, Some("fixture-refresh"))
            .expect("required allowlist cookies are complete");
        assert_eq!(
            credential.expose_cookie_header(),
            "DedeUserID=9001; SESSDATA=fixture-session"
        );
        assert_eq!(credential.expose_refresh_token(), Some("fixture-refresh"));
    }

    #[test]
    fn rejects_partial_credentials_and_non_bilibili_cookie_hosts() {
        let mut headers = HeaderMap::new();
        headers.append(
            SET_COOKIE,
            HeaderValue::from_static("SESSDATA=fixture-session; Path=/"),
        );
        assert!(extract_credential(&headers, None).is_err());

        assert!(validated_cookie_url("https://api.bilibili.com/nav").is_ok());
        assert!(validated_cookie_url("https://passport.bilibili.com/login").is_ok());
        assert!(validated_cookie_url("https://bilibili.com.example.org/nav").is_err());
        assert!(validated_cookie_url("http://api.bilibili.com/nav").is_err());
    }
}
