//! Video task-point: play info + progress report (CXKitty PointVideoDto).

use md5::{Digest, Md5};
use serde_json::Value;
use std::time::{SystemTime, UNIX_EPOCH};

use super::attachment::{self, CardContext};
use crate::api::client::{ChaoxingClient, ClientError, Result};

const API_CHAPTER_CARD_RESOURCE: &str = "https://mooc1-api.chaoxing.com/ananas/status";
const API_VIDEO_PLAYREPORT: &str = "https://mooc1-api.chaoxing.com/multimedia/log/a";
const ENC_SALT: &str = "d_yHJ!$pdA~5";

#[derive(Debug, Clone)]
pub struct VideoJob {
    pub object_id: String,
    pub job_id: String,
    pub other_info: String,
    pub rt: f64,
    pub fid: i64,
    pub already_passed: bool,
}

#[derive(Debug, Clone)]
pub struct VideoPlayInfo {
    pub dtoken: String,
    pub duration: i64,
    pub title: String,
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn video_enc(
    class_id: i64,
    puid: i64,
    job_id: &str,
    object_id: &str,
    playing_time: i64,
    duration: i64,
) -> String {
    let clip = format!("0_{duration}");
    let raw = format!(
        "[{class_id}][{puid}][{job_id}][{object_id}][{}][{ENC_SALT}][{}][{clip}]",
        playing_time * 1000,
        duration * 1000,
    );
    let mut hasher = Md5::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Percent-encode a value but keep `&` and `=` unescaped (CXKitty urlencode safe="&=").
fn encode_value_cx(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'&' | b'=' => {
                out.push(b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl ChaoxingClient {
    pub fn parse_video_attachment(attachment: &Value, object_id: &str) -> Result<Option<VideoJob>> {
        let Some(point) = attachment::find_by_object_id(attachment, object_id) else {
            return Ok(None);
        };
        let Some(job_id) = point.get("jobid").and_then(|v| v.as_str()) else {
            // Non-job video — ignore
            return Ok(None);
        };
        let other_info = point
            .get("otherInfo")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let rt = point
            .pointer("/property/rt")
            .and_then(|v| v.as_f64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
            .unwrap_or(0.9);
        let fid = attachment::attachment_fid(attachment).unwrap_or(0);
        let already_passed = match point.get("isPassed") {
            Some(Value::Bool(true)) => true,
            Some(Value::String(s)) if s == "true" || s == "1" => true,
            Some(Value::Number(n)) if n.as_i64() == Some(1) => true,
            _ => false,
        };
        Ok(Some(VideoJob {
            object_id: object_id.to_string(),
            job_id: job_id.to_string(),
            other_info,
            rt,
            fid,
            already_passed,
        }))
    }

    pub async fn fetch_video_play_info(
        &self,
        object_id: &str,
        fid: i64,
    ) -> Result<VideoPlayInfo> {
        let resp = self
            .http
            .get(format!("{API_CHAPTER_CARD_RESOURCE}/{object_id}"))
            .query(&[
                ("k", fid.to_string()),
                ("flag", "normal".into()),
                ("_dc", now_ms().to_string()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("video status: {e}")))?;

        if body.get("status").and_then(|v| v.as_str()) != Some("success")
            && body.get("dtoken").is_none()
        {
            return Err(ClientError::Api(format!(
                "拉取视频失败: {}",
                body.get("msg")
                    .or_else(|| body.get("error"))
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| body.to_string())
            )));
        }

        let dtoken = body
            .get("dtoken")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ClientError::Api("视频缺少 dtoken".into()))?
            .to_string();
        let duration = body
            .get("duration")
            .and_then(|v| v.as_i64().or_else(|| v.as_u64().map(|u| u as i64)))
            .unwrap_or(0);
        let title = body
            .get("filename")
            .and_then(|v| v.as_str())
            .unwrap_or(object_id)
            .to_string();

        Ok(VideoPlayInfo {
            dtoken,
            duration,
            title,
        })
    }

    /// Report watch progress. Returns JSON; `isPassed == true` means done.
    /// On HTTP 403, caller may retry once (common CXKitty note on enc encoding).
    pub async fn video_play_report(
        &self,
        cpi: i64,
        class_id: i64,
        puid: i64,
        job: &VideoJob,
        play: &VideoPlayInfo,
        playing_time: i64,
    ) -> Result<Value> {
        let enc = video_enc(
            class_id,
            puid,
            &job.job_id,
            &job.object_id,
            playing_time,
            play.duration,
        );
        let clip = format!("0_{}", play.duration);
        // Build query manually (CXKitty safe="&=") to avoid 403 from over-encoding.
        let qs = format!(
            "otherInfo={}&playingTime={}&duration={}&jobid={}&clipTime={}&clazzId={}&objectId={}&userid={}&isdrag=0&enc={}&rt={}&dtype=Video&view=pc&_t={}",
            encode_value_cx(&job.other_info),
            playing_time,
            play.duration,
            encode_value_cx(&job.job_id),
            encode_value_cx(&clip),
            class_id,
            encode_value_cx(&job.object_id),
            puid,
            enc,
            job.rt,
            now_ms(),
        );
        let url = format!("{API_VIDEO_PLAYREPORT}/{cpi}/{}?{qs}", play.dtoken);

        let mut last_err = None;
        for attempt in 0..2 {
            let resp = self.http.get(&url).send().await?;
            let status = resp.status();
            if status.as_u16() == 403 && attempt == 0 {
                // Common transient / enc quirk — brief pause then retry once.
                tokio::time::sleep(std::time::Duration::from_millis(400)).await;
                last_err = Some(ClientError::Api("视频上报 403，重试中".into()));
                continue;
            }
            if !status.is_success() {
                return Err(ClientError::Api(format!("视频上报 HTTP {status}")));
            }
            let body: Value = resp
                .json()
                .await
                .map_err(|e| ClientError::Parse(format!("video report: {e}")))?;
            if let Some(err) = body.get("error") {
                if !err.is_null() {
                    return Err(ClientError::Api(format!("视频上报失败: {err}")));
                }
            }
            return Ok(body);
        }
        Err(last_err.unwrap_or_else(|| ClientError::Api("视频上报失败".into())))
    }

    /// Full video completion loop (CXKitty MediaPlayResolver / execute_video_task).
    pub async fn complete_video(
        &self,
        ctx: &CardContext,
        object_id: &str,
        puid: i64,
        speed: f64,
        report_rate: i64,
        mut on_progress: impl FnMut(i64, i64),
        should_stop: impl Fn() -> bool,
    ) -> Result<(String, bool)> {
        let attachment = self.fetch_attachment(ctx).await?;
        let Some(job) = Self::parse_video_attachment(&attachment, object_id)? else {
            return Ok(("非任务点视频，已跳过".into(), false));
        };
        if job.already_passed {
            return Ok(("视频已完成".into(), true));
        }
        let play = self.fetch_video_play_info(&job.object_id, job.fid).await?;
        let duration = play.duration.max(1);
        let speed = if speed <= 0.0 { 1.0 } else { speed };
        let report_rate = report_rate.max(1);

        let mut playing_time: i64 = 0;
        let mut report_counter = report_rate;
        let title = play.title.clone();

        loop {
            if should_stop() {
                return Err(ClientError::Api("已停止".into()));
            }
            if report_counter >= report_rate || playing_time >= duration {
                report_counter = 0;
                let result = self
                    .video_play_report(ctx.cpi, ctx.class_id, puid, &job, &play, playing_time)
                    .await?;
                on_progress(playing_time, duration);
                if result.get("isPassed") == Some(&Value::Bool(true)) {
                    on_progress(duration, duration);
                    return Ok((format!("视频完成：{title}"), true));
                }
            }
            on_progress(playing_time, duration);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let step = (1.0 * speed).round() as i64;
            playing_time = (playing_time + step).min(duration);
            report_counter += step;
        }
    }
}
