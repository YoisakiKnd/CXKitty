//! Password login matching CXKitty `ChaoXingAPI.login_passwd`.

use aes::Aes128;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use cbc::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
use serde::{Deserialize, Serialize};

use super::client::{ChaoxingClient, ClientError, Result, UA_WEB};

const API_LOGIN_WEB: &str = "https://passport2.chaoxing.com/fanyalogin";
const API_SSO_LOGIN: &str = "https://sso.chaoxing.com/apis/login/userLogin4Uname.do";
const AES_KEY: &[u8; 16] = b"u2oh6Vu^HWe4_AES";

type Aes128CbcEnc = cbc::Encryptor<Aes128>;

fn aes_encrypt_b64(plain: &str) -> String {
    let encryptor = Aes128CbcEnc::new(AES_KEY.into(), AES_KEY.into());
    let ciphertext = encryptor.encrypt_padded_vec_mut::<Pkcs7>(plain.as_bytes());
    B64.encode(ciphertext)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfo {
    pub puid: i64,
    pub name: String,
    pub sex: i32,
    pub phone: String,
    pub school: String,
    pub stu_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginJson {
    status: Option<bool>,
    msg: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
struct SsoJson {
    result: Option<i32>,
    msg: Option<SsoMsg>,
}

#[derive(Debug, Deserialize)]
struct SsoMsg {
    puid: Option<serde_json::Value>,
    name: Option<String>,
    sex: Option<i32>,
    phone: Option<String>,
    schoolname: Option<String>,
    uname: Option<String>,
}

fn value_to_i64(v: &serde_json::Value) -> Option<i64> {
    match v {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

impl ChaoxingClient {
    /// Web password login (AES-CBC + PKCS7 + base64), then fetch SSO account info.
    pub async fn login_passwd(&self, phone: &str, passwd: &str) -> Result<AccountInfo> {
        self.clear_cookies();

        let enc_phone = aes_encrypt_b64(phone);
        let enc_passwd = aes_encrypt_b64(passwd);

        let resp = self
            .http
            .post(API_LOGIN_WEB)
            .header(reqwest::header::USER_AGENT, UA_WEB)
            .form(&[
                ("fid", "-1"),
                ("uname", &enc_phone),
                ("password", &enc_passwd),
                ("t", "true"),
                ("forbidotherlogin", "0"),
                ("validate", ""),
            ])
            .send()
            .await?
            .error_for_status()?;

        let body: LoginJson = resp.json().await.map_err(|e| {
            ClientError::Parse(format!("login response is not JSON: {e}"))
        })?;

        if body.status != Some(true) {
            let msg = body
                .msg
                .map(|m| m.to_string())
                .unwrap_or_else(|| "login failed".into());
            return Err(ClientError::Api(format!("登录失败: {msg}")));
        }

        self.fetch_account_info().await
    }

    pub async fn fetch_account_info(&self) -> Result<AccountInfo> {
        let resp = self
            .http
            .get(API_SSO_LOGIN)
            .send()
            .await?
            .error_for_status()?;

        let body: SsoJson = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("SSO response: {e}")))?;

        if body.result == Some(0) || body.msg.is_none() {
            return Err(ClientError::Api("会话无效或未登录".into()));
        }

        let msg = body.msg.unwrap();
        let puid = msg
            .puid
            .as_ref()
            .and_then(value_to_i64)
            .ok_or_else(|| ClientError::Parse("missing puid".into()))?;

        Ok(AccountInfo {
            puid,
            name: msg.name.unwrap_or_default(),
            sex: msg.sex.unwrap_or(-1),
            phone: msg.phone.unwrap_or_default(),
            school: msg.schoolname.unwrap_or_default(),
            stu_id: msg.uname,
        })
    }
}
