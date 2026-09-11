//! Chapters + task-point status for 刷课 (CXKitty ChapterContainer).

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::client::{ChaoxingClient, ClientError, Result};

const API_CHAPTER_LST: &str = "https://mooc1-api.chaoxing.com/gas/clazz";
const API_CHAPTER_POINT: &str = "https://mooc1-api.chaoxing.com/job/myjobsnodesmap";
const API_CHAPTER_CARDS: &str = "https://mooc1-api.chaoxing.com/gas/knowledge";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChapterInfo {
    pub chapter_id: i64,
    pub name: String,
    pub label: String,
    pub layer: i32,
    pub jobs: i32,
    pub point_total: i32,
    pub point_finished: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPointInfo {
    pub kind: String, // video | document | work | other
    pub title: String,
    pub chapter_id: i64,
    pub chapter_label: String,
    pub object_id: Option<String>,
    pub work_id: Option<String>,
    pub job_id: Option<String>,
    pub card_index: i32,
}

fn now_ms() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string()
}

fn as_i64(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn as_i32(v: &Value) -> Option<i32> {
    as_i64(v).map(|n| n as i32)
}

/// CXKitty `inf_enc_sign`
fn inf_enc_sign(params: &mut Vec<(String, String)>) {
    let mut pairs: Vec<_> = params.clone();
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    // CXKitty uses urlencode of original dict order then &DESKey=...
    // Python urllib.parse.urlencode preserves insertion order.
    let query = params
        .iter()
        .map(|(k, v)| format!("{}={}", k, urlencoding_form(v)))
        .collect::<Vec<_>>()
        .join("&");
    let signed = format!("{query}&DESKey=Z(AfY@XS");
    let mut hasher = Md5::new();
    hasher.update(signed.as_bytes());
    let inf_enc = format!("{:x}", hasher.finalize());
    params.push(("inf_enc".into(), inf_enc));
    let _ = pairs;
}

fn urlencoding_form(s: &str) -> String {
    // approximate application/x-www-form-urlencoded for ASCII-heavy values
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

impl ChaoxingClient {
    pub async fn list_chapters(
        &self,
        class_key: i64,
        cpi: i64,
        course_id: i64,
        class_id: i64,
        puid: i64,
    ) -> Result<Vec<ChapterInfo>> {
        let fields = "id,bbsid,classscore,isstart,allowdownload,chatid,name,state,isfiled,visiblescore,begindate,coursesetting.fields(id,courseid,hiddencoursecover,coursefacecheck),course.fields(id,name,infocontent,objectid,app,bulletformat,mappingcourseid,imageurl,teacherfactor,jobcount,knowledge.fields(id,name,indexOrder,parentnodeid,status,layer,label,jobcount,begintime,endtime,attachment.fields(id,type,objectid,extension).type(video)))";

        let resp = self
            .http
            .get(API_CHAPTER_LST)
            .query(&[
                ("id", class_key.to_string()),
                ("personid", cpi.to_string()),
                ("fields", fields.to_string()),
                ("view", "json".into()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("chapter list: {e}")))?;

        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| ClientError::Api("章节列表为空".into()))?;
        if data.is_empty() {
            return Err(ClientError::Api("章节列表拉取失败".into()));
        }

        let knowledge = &data[0]["course"]["data"][0]["knowledge"]["data"];
        let Some(arr) = knowledge.as_array() else {
            return Ok(Vec::new());
        };

        let mut chapters: Vec<ChapterInfo> = arr
            .iter()
            .filter_map(|cha| {
                Some(ChapterInfo {
                    chapter_id: as_i64(cha.get("id")?)?,
                    name: cha
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string(),
                    label: cha
                        .get("label")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    layer: as_i32(cha.get("layer")?).unwrap_or(0),
                    jobs: as_i32(cha.get("jobcount")?).unwrap_or(0),
                    point_total: 0,
                    point_finished: 0,
                })
            })
            .collect();

        chapters.sort_by(|a, b| {
            let pa: Vec<i32> = a.label.split('.').filter_map(|x| x.parse().ok()).collect();
            let pb: Vec<i32> = b.label.split('.').filter_map(|x| x.parse().ok()).collect();
            pa.cmp(&pb)
        });

        // point status
        if !chapters.is_empty() {
            let nodes = chapters
                .iter()
                .map(|c| c.chapter_id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let resp = self
                .http
                .post(API_CHAPTER_POINT)
                .form(&[
                    ("view", "json"),
                    ("nodes", &nodes),
                    ("clazzid", &class_id.to_string()),
                    ("time", &now_ms()),
                    ("userid", &puid.to_string()),
                    ("cpi", &cpi.to_string()),
                    ("courseid", &course_id.to_string()),
                ])
                .send()
                .await?;
            if resp.status().is_success() {
                if let Ok(map) = resp.json::<Value>().await {
                    for c in &mut chapters {
                        if let Some(pd) = map.get(c.chapter_id.to_string()) {
                            let unfinished = as_i32(pd.get("unfinishcount").unwrap_or(&Value::Null))
                                .unwrap_or(0);
                            let total =
                                as_i32(pd.get("totalcount").unwrap_or(&Value::Null)).unwrap_or(0);
                            let finished =
                                as_i32(pd.get("finishcount").unwrap_or(&Value::Null)).unwrap_or(0);
                            c.point_total = if unfinished != 0 && total == 0 {
                                unfinished
                            } else {
                                total
                            };
                            c.point_finished = finished;
                        }
                    }
                }
            }
        }

        Ok(chapters)
    }

    pub async fn list_task_points(
        &self,
        course_id: i64,
        chapter: &ChapterInfo,
    ) -> Result<Vec<TaskPointInfo>> {
        let mut params = vec![
            ("id".into(), chapter.chapter_id.to_string()),
            ("courseid".into(), course_id.to_string()),
            (
                "fields".into(),
                "id,parentnodeid,indexorder,label,layer,name,begintime,createtime,lastmodifytime,status,jobUnfinishedCount,clickcount,openlock,card.fields(id,knowledgeid,title,knowledgeTitile,description,cardorder).contentcard(all)".into(),
            ),
            ("view".into(), "json".into()),
            ("token".into(), "4faa8662c59590c6f43ae9fe5b002b42".into()),
            ("_time".into(), now_ms()),
        ];
        inf_enc_sign(&mut params);

        let resp = self
            .http
            .get(API_CHAPTER_CARDS)
            .query(&params)
            .send()
            .await?
            .error_for_status()?;

        let body: Value = resp
            .json()
            .await
            .map_err(|e| ClientError::Parse(format!("chapter cards: {e}")))?;

        let cards = body
            .pointer("/data/0/card/data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        let mut points = Vec::new();
        for (card_index, card) in cards.iter().enumerate() {
            let Some(desc) = card.get("description").and_then(|v| v.as_str()) else {
                continue;
            };
            // crude iframe scrape
            for cap in iframe_iter(desc) {
                let module = cap.module.as_str();
                let data: Value = serde_json::from_str(&cap.data).unwrap_or(Value::Null);
                let kind = match module {
                    "insertvideo" => "video",
                    "insertdoc" => "document",
                    "work" => "work",
                    other => other,
                };
                let title = data
                    .get("name")
                    .or_else(|| data.get("filename"))
                    .or_else(|| data.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or(kind)
                    .to_string();
                points.push(TaskPointInfo {
                    kind: kind.to_string(),
                    title,
                    chapter_id: chapter.chapter_id,
                    chapter_label: chapter.label.clone(),
                    object_id: data
                        .get("objectid")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    work_id: data
                        .get("workid")
                        .and_then(|v| v.as_str().map(|s| s.to_string()).or_else(|| {
                            v.as_i64().map(|n| n.to_string())
                        })),
                    job_id: data
                        .get("_jobid")
                        .or_else(|| data.get("jobid"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    card_index: card_index as i32,
                });
            }
        }
        Ok(points)
    }
}

struct IFrameCap {
    module: String,
    data: String,
}

fn iframe_iter(html: &str) -> Vec<IFrameCap> {
    let mut out = Vec::new();
    let lower = html;
    let mut rest = lower;
    while let Some(start) = rest.find("<iframe") {
        let after = &rest[start..];
        let end = after.find('>').unwrap_or(after.len());
        let tag = &after[..end];
        let module = attr_of(tag, "module").unwrap_or_default();
        let data = attr_of(tag, "data").unwrap_or_default();
        if !module.is_empty() && !data.is_empty() {
            out.push(IFrameCap {
                module,
                data: html_unescape_attr(&data),
            });
        }
        rest = &after[end.min(after.len())..];
        if rest.is_empty() {
            break;
        }
        rest = &rest[1.min(rest.len())..];
    }
    out
}

fn attr_of(tag: &str, name: &str) -> Option<String> {
    // module="..." or module='...'
    let patterns = [
        format!("{name}=\""),
        format!("{name}='"),
    ];
    for (i, p) in patterns.iter().enumerate() {
        if let Some(pos) = tag.find(p) {
            let rest = &tag[pos + p.len()..];
            let quote = if i == 0 { '"' } else { '\'' };
            if let Some(end) = rest.find(quote) {
                return Some(rest[..end].to_string());
            }
        }
    }
    None
}

fn html_unescape_attr(s: &str) -> String {
    s.replace("&quot;", "\"")
        .replace("&#34;", "\"")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
