//! QR-code login, ported from the original Python backend (`cxapi/api.py`).
//!
//! Flow (identical to `ChaoXingAPI.qr_get` / `login_qr`):
//!   1. `GET https://passport2.chaoxing.com/login` with the **web** UA and read the
//!      hidden `uuid` / `enc` inputs. The mobile UA is rejected here — the
//!      original code carries an explicit warning about this.
//!   2. `GET /createqr?uuid=..&fid=-1`, which returns the QR **image** itself
//!      (the same `<img src="/createqr?...">` the official login page renders).
//!      The original re-encoded the URL with Python's `qrcode` library; returning
//!      the server's own image is equivalent and needs no encoder dependency.
//!   3. `POST /getauthstatus` with `uuid` + `enc` until `status == true`, then
//!      read the account through the existing SSO call.

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{ChaoxingClient, ClientError, Result, UA_WEB};

const PAGE_LOGIN: &str = "https://passport2.chaoxing.com/login";
const API_QRCREATE: &str = "https://passport2.chaoxing.com/createqr";
const API_QRLOGIN: &str = "https://passport2.chaoxing.com/getauthstatus";

/// QR login session state (mirrors `ChaoXingAPI.qr_uuid` / `qr_enc`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QrSession {
    pub uuid: String,
    pub enc: String,
}

/// `start_qr_login` result: the QR image plus the login URL it encodes.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrStartResult {
    /// `data:image/...;base64,...` ready for an `<img src>`.
    pub qr_image: String,
    /// The `toauthlogin` URL encoded in the image (kept for parity/debugging).
    pub qr_url: String,
}

/// `poll_qr_login` result. `state` is `pending` until the phone confirms.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QrPollResult {
    pub state: String, // pending | success
    pub detail: Option<String>,
    /// Account summary, present only once `state == "success"` (mirrors the
    /// original's `{"state": "success", "account": ...}` reply).
    pub account: Option<super::login::AccountInfo>,
}

fn extract_hidden_input(html: &str, id: &str) -> Option<String> {
    let doc = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse(&format!("input#{id}")).ok()?;
    doc.select(&selector)
        .next()
        .and_then(|el| el.value().attr("value"))
        .map(|v| v.to_string())
}

impl ChaoxingClient {
    /// Step 1+2: fetch the QR image the official login page shows.
    pub async fn qr_get(&self) -> Result<QrStartResult> {
        self.clear_cookies();

        // The login page must be fetched with the web UA (mobile UA fails auth).
        let resp = self
            .http
            .get(PAGE_LOGIN)
            .header(reqwest::header::USER_AGENT, UA_WEB)
            .send()
            .await?
            .error_for_status()?;
        let html = resp.text().await?;

        let uuid = extract_hidden_input(&html, "uuid")
            .ok_or_else(|| ClientError::Parse("登录页缺少 uuid".into()))?;
        let enc = extract_hidden_input(&html, "enc")
            .ok_or_else(|| ClientError::Parse("登录页缺少 enc".into()))?;

        // Activate the QR key; the response body is the image we want.
        let qr_resp = self
            .http
            .get(API_QRCREATE)
            .query(&[("uuid", uuid.as_str()), ("fid", "-1")])
            .send()
            .await?
            .error_for_status()?;

        let content_type = qr_resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let bytes = qr_resp.bytes().await?;
        if bytes.is_empty() {
            return Err(ClientError::Api("二维码图片为空".into()));
        }

        let qr_url = format!(
            "https://passport2.chaoxing.com/toauthlogin?uuid={uuid}&enc={enc}&xxtrefer=&clientid=&mobiletip="
        );

        Ok(QrStartResult {
            qr_image: format!("data:{content_type};base64,{}", B64.encode(&bytes)),
            qr_url,
        })
    }

    /// Step 3: poll the auth status once. `status == true` means the phone confirmed.
    pub async fn login_qr(&self, session: &QrSession) -> Result<(bool, Option<String>)> {
        let resp = self
            .http
            .post(API_QRLOGIN)
            .form(&[("enc", session.enc.as_str()), ("uuid", session.uuid.as_str())])
            .send()
            .await?
            .error_for_status()?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("扫码状态响应: {e}")))?;

        let ok = body.get("status").and_then(|v| v.as_bool()).unwrap_or(false);
        let detail = body
            .get("mes")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        Ok((ok, detail))
    }
}
