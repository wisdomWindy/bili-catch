use url::Url;

use crate::models::{AppError, AppErrorCode};

fn invalid_media_url() -> AppError {
    AppError::new(AppErrorCode::E004, "The media source is unavailable")
}

fn has_allowed_suffix(host: &str, suffix: &str) -> bool {
    host == suffix || host.ends_with(&format!(".{suffix}"))
}

pub fn validate_media_url(value: &str) -> Result<Url, AppError> {
    let url = Url::parse(value).map_err(|_| invalid_media_url())?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid_media_url());
    }
    let host = url
        .host_str()
        .map(str::to_ascii_lowercase)
        .ok_or_else(invalid_media_url)?;
    if ["bilivideo.com", "bilivideo.cn", "hdslb.com"]
        .iter()
        .any(|suffix| has_allowed_suffix(&host, suffix))
    {
        Ok(url)
    } else {
        Err(invalid_media_url())
    }
}
