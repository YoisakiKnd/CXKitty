//! Fetch chapter card AttachmentSetting (CXKitty TaskPointBase.fetch_attachment).

use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

use crate::api::client::{ChaoxingClient, ClientError, Result};

const PAGE_MOBILE_CHAPTER_CARD: &str = "https://mooc1-api.chaoxing.com/knowledge/cards";
const PAGE_REFRESH_CHAPTER: &str =
    "https://mooc1.chaoxing.com/mooc-ans/mycourse/studentstudyAjax";

#[derive(Debug, Clone)]
pub struct CardContext {
    pub course_id: i64,
    pub class_id: i64,
    pub knowledge_id: i64,
    pub cpi: i64,
    pub card_index: i32,
}

fn attachment_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"window\.AttachmentSetting\s*=\s*(?P<json>.+?);").expect("regex")
    })
}

impl ChaoxingClient {
    /// Pull `window.AttachmentSetting` JSON for a chapter card.
    pub async fn fetch_attachment(&self, ctx: &CardContext) -> Result<Value> {
        let resp = self
            .http
            .get(PAGE_MOBILE_CHAPTER_CARD)
            .query(&[
                ("clazzid", ctx.class_id.to_string()),
                ("courseid", ctx.course_id.to_string()),
                ("knowledgeid", ctx.knowledge_id.to_string()),
                ("num", ctx.card_index.to_string()),
                ("isPhone", "1".into()),
                ("control", "true".into()),
                ("cpi", ctx.cpi.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let html = resp
            .text()
            .await
            .map_err(|e| ClientError::Parse(format!("attachment html: {e}")))?;

        if let Some(caps) = attachment_re().captures(&html) {
            let json_str = caps.name("json").map(|m| m.as_str()).unwrap_or("");
            return serde_json::from_str(json_str)
                .map_err(|e| ClientError::Parse(format!("AttachmentSetting JSON: {e}")));
        }

        if html.contains("章节未开放") {
            return Err(ClientError::Api("章节未开放！".into()));
        }
        Err(ClientError::Api("无法解析 AttachmentSetting".into()))
    }

    /// Soft-refresh chapter lock state (CXKitty refresh_chapter).
    pub async fn refresh_chapter(
        &self,
        course_id: i64,
        class_id: i64,
        chapter_id: i64,
        cpi: i64,
    ) -> Result<()> {
        let _ = self
            .http
            .get(PAGE_REFRESH_CHAPTER)
            .query(&[
                ("courseId", course_id.to_string()),
                ("clazzid", class_id.to_string()),
                ("chapterId", chapter_id.to_string()),
                ("cpi", cpi.to_string()),
                ("verificationcode", String::new()),
                ("mooc2", "1".into()),
            ])
            .send()
            .await?;
        Ok(())
    }
}

/// Locate an attachment entry whose property.objectid matches.
pub fn find_by_object_id<'a>(attachment: &'a Value, object_id: &str) -> Option<&'a Value> {
    let arr = attachment.get("attachments")?.as_array()?;
    arr.iter().find(|point| {
        point
            .pointer("/property/objectid")
            .and_then(|v| v.as_str())
            .map(|s| s == object_id)
            .unwrap_or(false)
    })
}

pub fn attachment_fid(attachment: &Value) -> Option<i64> {
    attachment
        .pointer("/defaults/fid")
        .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
}

