//! Course list from `mycourse/backclazzdata` (CXKitty `fetch_classes`).

use serde::{Deserialize, Serialize};

use super::client::{ChaoxingClient, ClientError, Result};

const API_CLASS_LST: &str = "https://mooc1-api.chaoxing.com/mycourse/backclazzdata";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CourseInfo {
    pub course_id: i64,
    pub class_id: i64,
    pub cpi: i64,
    pub key: i64,
    pub name: String,
    pub teacher_name: String,
    /// 0 = 进行中, 1 = 已结课
    pub state: i32,
}

#[derive(Debug, Deserialize)]
struct BackClazzData {
    result: Option<i32>,
    #[serde(rename = "channelList")]
    channel_list: Option<Vec<ChannelItem>>,
}

#[derive(Debug, Deserialize)]
struct ChannelItem {
    cpi: Option<serde_json::Value>,
    key: Option<serde_json::Value>,
    content: Option<ChannelContent>,
}

#[derive(Debug, Deserialize)]
struct ChannelContent {
    id: Option<serde_json::Value>,
    state: Option<i32>,
    course: Option<CourseWrap>,
}

#[derive(Debug, Deserialize)]
struct CourseWrap {
    data: Option<Vec<CourseData>>,
}

#[derive(Debug, Deserialize)]
struct CourseData {
    id: Option<serde_json::Value>,
    name: Option<String>,
    teacherfactor: Option<String>,
}

fn as_i64(v: &Option<serde_json::Value>) -> Option<i64> {
    v.as_ref().and_then(|v| match v {
        serde_json::Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        serde_json::Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

impl ChaoxingClient {
    pub async fn list_courses(&self) -> Result<Vec<CourseInfo>> {
        let resp = self
            .http
            .get(API_CLASS_LST)
            .query(&[("view", "json"), ("rss", "1")])
            .send()
            .await?
            .error_for_status()?;

        let body: BackClazzData = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("course list JSON: {e}")))?;

        if body.result != Some(1) {
            return Err(ClientError::Api("课程列表拉取失败（会话可能已失效）".into()));
        }

        let mut out = Vec::new();
        for item in body.channel_list.unwrap_or_default() {
            let Some(content) = item.content else { continue };
            let Some(course) = content.course else { continue };
            let Some(data) = course.data.and_then(|d| d.into_iter().next()) else {
                continue;
            };
            let Some(course_id) = as_i64(&data.id) else { continue };
            let Some(class_id) = as_i64(&content.id) else { continue };
            let Some(cpi) = as_i64(&item.cpi) else { continue };
            let key = as_i64(&item.key).unwrap_or(class_id);

            out.push(CourseInfo {
                course_id,
                class_id,
                cpi,
                key,
                name: data.name.unwrap_or_else(|| "未命名课程".into()),
                teacher_name: data.teacherfactor.unwrap_or_else(|| "未知".into()),
                state: content.state.unwrap_or(0),
            });
        }

        Ok(out)
    }
}
