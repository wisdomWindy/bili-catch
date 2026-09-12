use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use super::raw::WbiKey;

const MIXIN_TABLE: [usize; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19, 29,
    28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4, 22, 25,
    54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];
const WBI_KEY_TTL_SECONDS: u64 = 12 * 60 * 60;

pub(crate) trait Clock: Send + Sync {
    fn now_seconds(&self) -> u64;
}

pub(crate) struct SystemClock;

impl Clock for SystemClock {
    fn now_seconds(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    }
}

pub(crate) fn derive_mixin_key(img_url: &str, sub_url: &str) -> Option<WbiKey> {
    fn filename(url: &str) -> Option<&str> {
        url.rsplit('/')
            .next()?
            .split('.')
            .next()
            .filter(|value| !value.is_empty())
    }

    let source = format!("{}{}", filename(img_url)?, filename(sub_url)?);
    let characters = source.chars().collect::<Vec<_>>();
    if characters.len() < 64 {
        return None;
    }
    let mixin_key = MIXIN_TABLE
        .iter()
        .filter_map(|index| characters.get(*index))
        .take(32)
        .collect::<String>();
    Some(WbiKey { mixin_key })
}

pub(crate) fn sign_query(parameters: &[(String, String)], key: &WbiKey, timestamp: u64) -> String {
    let mut sorted = BTreeMap::new();
    for (name, value) in parameters {
        let filtered = value.replace(['!', '\'', '(', ')', '*'], "");
        sorted.insert(name.clone(), filtered);
    }
    sorted.insert("wts".into(), timestamp.to_string());

    let mut serializer = url::form_urlencoded::Serializer::new(String::new());
    for (name, value) in sorted {
        serializer.append_pair(&name, &value);
    }
    let query = serializer.finish();
    let signature = format!("{:x}", md5::compute(format!("{query}{}", key.mixin_key)));
    format!("{query}&w_rid={signature}")
}

pub(crate) struct WbiKeyCache {
    entry: Mutex<Option<(WbiKey, u64)>>,
}

impl WbiKeyCache {
    pub fn new() -> Self {
        Self {
            entry: Mutex::new(None),
        }
    }

    pub fn get(&self, now_seconds: u64) -> Option<WbiKey> {
        let entry = self.entry.lock().ok()?;
        entry
            .as_ref()
            .filter(|(_, stored_at)| now_seconds.saturating_sub(*stored_at) < WBI_KEY_TTL_SECONDS)
            .map(|(key, _)| key.clone())
    }

    pub fn put(&self, key: WbiKey, now_seconds: u64) {
        if let Ok(mut entry) = self.entry.lock() {
            *entry = Some((key, now_seconds));
        }
    }

    pub fn invalidate(&self) {
        if let Ok(mut entry) = self.entry.lock() {
            *entry = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{derive_mixin_key, sign_query, WbiKeyCache};
    use crate::infrastructure::bilibili::raw::WbiKey;

    #[test]
    fn creates_a_deterministic_sorted_signature() {
        let key = WbiKey {
            mixin_key: "abcdefghijklmnopqrstuvwxyz123456".into(),
        };
        let signature = sign_query(
            &[
                ("qn".into(), "80".into()),
                ("bvid".into(), "BV1xx411c7BF".into()),
            ],
            &key,
            1700000000,
        );
        assert!(signature.starts_with("bvid=BV1xx411c7BF&qn=80&wts=1700000000&w_rid="));
        assert_eq!(signature.rsplit('=').next().unwrap().len(), 32);
    }

    #[test]
    fn derives_and_expires_the_cached_key_after_twelve_hours() {
        let key = derive_mixin_key(
            "https://i0.hdslb.com/bfs/wbi/abcdefghijklmnopqrstuvwxyzABCDEF.png",
            "https://i0.hdslb.com/bfs/wbi/GHIJKLMNOPQRSTUVWXYZ0123456789abcd.png",
        )
        .expect("urls should contain enough key material");
        assert_eq!(key.mixin_key.len(), 32);

        let cache = WbiKeyCache::new();
        cache.put(key.clone(), 100);
        assert_eq!(cache.get(100 + 43_199), Some(key));
        assert_eq!(cache.get(100 + 43_200), None);
    }
}
