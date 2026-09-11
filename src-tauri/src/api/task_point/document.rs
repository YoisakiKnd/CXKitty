//! Document task-point mark-read (CXKitty PointDocumentDto).

use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use super::attachment::{self, CardContext};
use crate::api::client::{ChaoxingClient, ClientError, Result};

const API_DOCUMENT_READINGREPORT: &str = "https://mooc1.chaoxing.com/ananas/job/document";

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DocumentJob {
    pub object_id: String,
    pub job_id: String,
    pub jtoken: String,
    pub title: String,
}

fn now_ms() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

impl ChaoxingClient {
    pub fn parse_document_attachment(
        attachment: &Value,
        object_id: &str,
    ) -> Result<Option<DocumentJob>> {
        let Some(point) = attachment::find_by_object_id(attachment, object_id) else {
            return Ok(None);
        };
        // Only real job documents
        let is_job = match point.get("job") {
            Some(Value::Bool(true)) => true,
            Some(Value::String(s)) => s == "true" || s == "1",
            _ => false,
        };
        if !is_job {
            return Ok(None);
        }
        let job_id = point
            .get("jobid")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::Api("文档缺少 jobid".into()))?
            .to_string();
        let jtoken = point
            .get("jtoken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::Api("文档缺少 jtoken".into()))?
            .to_string();
        let title = point
            .pointer("/property/name")
            .and_then(|v| v.as_str())
            .unwrap_or("文档")
            .to_string();
        Ok(Some(DocumentJob {
            object_id: object_id.to_string(),
            job_id,
            jtoken,
            title,
        }))
    }

    pub async fn report_document(
        &self,
        ctx: &CardContext,
        job: &DocumentJob,
    ) -> Result<Value> {
        let resp = self
            .http
            .get(API_DOCUMENT_READINGREPORT)
            .query(&[
                ("jobid", job.job_id.clone()),
                ("knowledgeid", ctx.knowledge_id.to_string()),
                ("courseid", ctx.course_id.to_string()),
                ("clazzid", ctx.class_id.to_string()),
                ("jtoken", job.jtoken.clone()),
                ("_dc", now_ms()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("document report: {e}")))?;
        if let Some(err) = body.get("error") {
            if !err.is_null() {
                return Err(ClientError::Api(format!("文档上报失败: {err}")));
            }
        }
        Ok(body)
    }

    pub async fn complete_document(
        &self,
        ctx: &CardContext,
        object_id: &str,
    ) -> Result<(String, bool)> {
        let attachment = self.fetch_attachment(ctx).await?;
        let Some(job) = Self::parse_document_attachment(&attachment, object_id)? else {
            return Ok(("非任务点文档，已跳过".into(), false));
        };
        let _ = self.report_document(ctx, &job).await?;
        Ok((format!("文档完成：{}", job.title), true))
    }
}
