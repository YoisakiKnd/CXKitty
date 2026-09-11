//! HTTP client with cookie jar and mobile/web UA helpers (CXKitty SessionWraper style).

use std::sync::Arc;

use cookie_store::CookieStore;
use rand::Rng;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest_cookie_store::CookieStoreMutex;
use thiserror::Error;

pub const UA_WEB: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.35";

fn random_mobile_ua() -> String {
    let mut rng = rand::thread_rng();
    let android_ver = rng.gen_range(9..=12);
    let mi = rng.gen_range(10..=12);
    let imei: String = (0..32)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect();
    format!(
        "Dalvik/2.1.0 (Linux; U; Android {android_ver}; MI{mi} Build/SKQ1.210216.001) (device:MI{mi}) Language/zh_CN com.chaoxing.mobile/ChaoXingStudy_3_5.1.4_android_phone_614_74 (@Kalimdor)_{imei}"
    )
}

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

/// Shared Chaoxing HTTP session (cookies persist across login / courses / homework).
#[derive(Clone)]
pub struct ChaoxingClient {
    pub http: reqwest::Client,
    pub cookies: Arc<CookieStoreMutex>,
    #[allow(dead_code)]
    mobile_ua: String,
}

impl ChaoxingClient {
    pub fn new() -> Result<Self> {
        let cookies = Arc::new(CookieStoreMutex::new(CookieStore::default()));
        let mobile_ua = random_mobile_ua();

        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_str(&mobile_ua).unwrap());
        headers.insert(
            "X-Requested-With",
            HeaderValue::from_static("com.chaoxing.mobile"),
        );

        let http = reqwest::Client::builder()
            .default_headers(headers)
            .cookie_provider(cookies.clone())
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            http,
            cookies,
            mobile_ua,
        })
    }

    #[allow(dead_code)]
    pub fn mobile_ua(&self) -> &str {
        &self.mobile_ua
    }

    #[allow(dead_code)]
    pub fn web_ua() -> &'static str {
        UA_WEB
    }

    /// Clear all cookies (e.g. before a fresh login).
    pub fn clear_cookies(&self) {
        let mut jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        jar.clear();
    }
}

impl Default for ChaoxingClient {
    fn default() -> Self {
        Self::new().expect("failed to build HTTP client")
    }
}
