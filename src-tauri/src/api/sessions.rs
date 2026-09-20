//! Local session persistence, ported from the original Python backend
//! (`webapp/service.py`: `_sessions_load` / `_save_session` / `_mask_phone` /
//! `_mask_name`).
//!
//! File layout is kept identical to the original: one JSON file per account at
//! `<session_dir>/<phone>.json` holding `phone` / `puid` / `passwd` / `name` /
//! `ck`, where `ck` is the `k=v;k=v;` cookie string produced by the original
//! `ck_dump()`.

use std::path::{Path, PathBuf};

use cookie_store::RawCookie;
use serde::{Deserialize, Serialize};
use url::Url;

use super::client::{ChaoxingClient, ClientError, Result};

/// Cookie string is re-attached to every `*.chaoxing.com` host on restore.
const COOKIE_HOST: &str = "https://chaoxing.com/";

/// A saved session as listed in the UI (mirrors `list_saved_sessions`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedSession {
    pub phone: String,
    pub puid: i64,
    pub name: String,
    pub masked_name: String,
    pub masked_phone: String,
    pub has_password: bool,
}

/// On-disk shape, identical to the original `<phone>.json`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    pub phone: String,
    pub puid: i64,
    #[serde(default)]
    pub passwd: Option<String>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub ck: String,
}

/// `_mask_phone`: keep the first 3 and last 4 digits.
fn mask_phone(phone: &str) -> String {
    let chars: Vec<char> = phone.chars().collect();
    if chars.len() <= 7 {
        return "*".repeat(chars.len().max(1));
    }
    let head: String = chars[..3].iter().collect();
    let tail: String = chars[chars.len() - 4..].iter().collect();
    format!("{head}****{tail}")
}

/// `_mask_name`: first char, stars, last char (a single `*` for short names).
fn mask_name(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    match chars.len() {
        0 => String::new(),
        1 => chars[0].to_string(),
        2 => format!("{}*", chars[0]),
        n => {
            let head = chars[0];
            let tail = chars[n - 1];
            format!("{head}{}{tail}", "*".repeat(n - 2))
        }
    }
}

/// Resolve the session directory (created on demand).
pub fn sessions_dir() -> PathBuf {
    // Kept out of the repo: the app data dir for this bundle identifier.
    let base = dirs_fallback();
    base.join("session")
}

fn dirs_fallback() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = PathBuf::from(home).join(".cxkitty-tauri");
        if std::fs::create_dir_all(&p).is_ok() {
            return p;
        }
    }
    PathBuf::from(".cxkitty-tauri")
}

impl ChaoxingClient {
    /// Export the current cookie jar as the original `k=v;k=v;` string.
    pub fn ck_dump(&self) -> String {
        let jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        let mut parts: Vec<String> = Vec::new();
        for cookie in jar.iter_any() {
            parts.push(format!("{}={}", cookie.name(), cookie.value()));
        }
        parts.join(";")
    }

    /// Import a `k=v;k=v;` cookie string (`ck_load`).
    pub fn ck_load(&self, ck: &str) -> Result<()> {
        let url = Url::parse(COOKIE_HOST).map_err(|e| ClientError::Parse(e.to_string()))?;
        let mut jar = self.cookies.lock().unwrap_or_else(|e| e.into_inner());
        jar.clear();
        for field in ck.split(';') {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            if let Some((key, value)) = field.split_once('=') {
                let cookie = RawCookie::new(key.trim().to_string(), value.to_string());
                let _ = jar.insert_raw(&cookie, &url);
            }
        }
        Ok(())
    }
}

/// `_sessions_load`: read every `<phone>.json` in the session directory.
pub fn list_saved_sessions() -> Vec<SavedSession> {
    let dir = sessions_dir();
    let mut out: Vec<SavedSession> = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(file) = serde_json::from_str::<SessionFile>(&text) else {
            continue;
        };
        out.push(SavedSession {
            masked_name: mask_name(&file.name),
            masked_phone: mask_phone(&file.phone),
            has_password: file.passwd.is_some(),
            phone: file.phone,
            puid: file.puid,
            name: file.name,
        });
    }
    out.sort_by(|a, b| a.phone.cmp(&b.phone));
    out
}

/// Read one session file by phone (`use_saved_session` needs the cookie string).
pub fn read_session(phone: &str) -> Option<SessionFile> {
    let path = session_path(phone);
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str::<SessionFile>(&text).ok()
}

fn session_path(phone: &str) -> PathBuf {
    sessions_dir().join(format!("{phone}.json"))
}

/// `_save_session`: persist cookies + account so the session can be reused.
pub fn save_session(client: &ChaoxingClient, phone: &str, puid: i64, name: &str, passwd: Option<&str>) {
    let dir = sessions_dir();
    if !dir.is_dir() && std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let file = SessionFile {
        phone: phone.to_string(),
        puid,
        passwd: passwd.map(|s| s.to_string()),
        name: name.to_string(),
        ck: client.ck_dump(),
    };
    if let Ok(text) = serde_json::to_string_pretty(&file) {
        let _ = std::fs::write(session_path(phone), text);
    }
}

/// True when the given path is inside the session directory (test helper).
#[allow(dead_code)]
pub fn is_session_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()) == Some("json")
}
