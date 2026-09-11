//! Homework list ported from query-chaoxing-worklist (`getWorkParams` + `getAllWork`).

use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use super::client::{ChaoxingClient, ClientError, Result, UA_WEB};

const API_GET_ALL_WORK: &str = "https://mooc1.chaoxing.com/work/getAllWork";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomeworkItem {
    pub course_name: String,
    pub work_name: String,
    pub status: String,
    pub status_code: String,
    pub end_time: String,
    pub result_num: String,
    pub work_url: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkParams {
    pub course_id: String,
    pub clazzid: String,
    pub cpi: String,
    pub enc: String,
    pub openc: String,
}

fn map_status(label: &str) -> (&'static str, &'static str) {
    match label.trim() {
        "待做" => ("待做", "UNDONE"),
        "已过期" => ("已过期", "EXPIRED"),
        "已完成" => ("已完成", "DONE"),
        "待批阅" => ("待批阅", "UNDONE_READ"),
        other if other.is_empty() => ("其他", "OTHER"),
        _ => ("其他", "OTHER"),
    }
}

fn immediate_text(el: scraper::ElementRef<'_>) -> String {
    el.text()
        .collect::<Vec<_>>()
        .join("")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn attr_value(html: &Html, id: &str) -> Option<String> {
    let sel = Selector::parse(&format!("#{id}")).ok()?;
    html.select(&sel).next().and_then(|e| {
        e.value()
            .attr("value")
            .map(|s| s.to_string())
            .or_else(|| Some(immediate_text(e)))
    })
}

fn parse_hidden_params(html_text: &str) -> Option<(String, String, String, String, String)> {
    let html = Html::parse_document(html_text);
    let courseid = attr_value(&html, "courseid")?;
    let clazzid = attr_value(&html, "clazzid")?;
    let cpi = attr_value(&html, "cpi")?;
    let enc = attr_value(&html, "oldenc")
        .or_else(|| attr_value(&html, "enc"))
        .unwrap_or_default();
    let openc = attr_value(&html, "openc").unwrap_or_default();
    if courseid.is_empty() || clazzid.is_empty() {
        return None;
    }
    Some((courseid, clazzid, cpi, enc, openc))
}

impl ChaoxingClient {
    /// Resolve enc/openc for a course (worklist `getWorkParams` behavior).
    pub async fn get_work_params(
        &self,
        course_id: i64,
        class_id: i64,
        cpi: i64,
    ) -> Result<WorkParams> {
        let middle = format!(
            "https://mooc1.chaoxing.com/visit/stucoursemiddle?courseid={course_id}&clazzid={class_id}&cpi={cpi}&ismooc2=1"
        );

        // First request: allow redirects to land on student course page when possible.
        let resp = self
            .http
            .get(&middle)
            .header(reqwest::header::USER_AGENT, UA_WEB)
            .header("Referer", "https://mooc1.chaoxing.com/")
            .send()
            .await?;

        let mut html_text = resp.text().await?;
        let mut parsed = parse_hidden_params(&html_text);

        // Fallback: no-redirect dance like the Node worklist if hidden fields missing.
        if parsed.is_none() {
            let client_no_redir = reqwest::Client::builder()
                .cookie_provider(self.cookies.clone())
                .redirect(reqwest::redirect::Policy::none())
                .timeout(std::time::Duration::from_secs(30))
                .build()?;

            let r = client_no_redir
                .get(&middle)
                .header(reqwest::header::USER_AGENT, UA_WEB)
                .send()
                .await?;

            if let Some(loc) = r
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|v| v.to_str().ok())
            {
                let abs = if loc.starts_with("http") {
                    loc.to_string()
                } else {
                    format!("https://mooc1.chaoxing.com{loc}")
                };
                let stu = self
                    .http
                    .get(&abs)
                    .header(reqwest::header::USER_AGENT, UA_WEB)
                    .send()
                    .await?;
                html_text = stu.text().await?;
                parsed = parse_hidden_params(&html_text);
            }
        }

        let (courseid, clazzid, cpi_s, enc0, openc0) = parsed.ok_or_else(|| {
            ClientError::Parse("无法从课程页解析 work 参数（courseid/clazzid）".into())
        })?;

        // transfer to refresh enc (worklist behavior)
        let homepage = format!(
            "https://mooc1.chaoxing.com/mycourse/studentcourse?courseId={courseid}&clazzid={clazzid}&cpi={cpi_s}&openc={openc0}&enc={enc0}"
        );
        let transfer = format!(
            "https://mooc1.chaoxing.com/mycourse/transfer?moocId={courseid}&ut=s&clazzid={clazzid}&refer={}",
            urlencoding_encode(&homepage)
        );

        let client_no_redir = reqwest::Client::builder()
            .cookie_provider(self.cookies.clone())
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        let tr = client_no_redir
            .get(&transfer)
            .header(reqwest::header::USER_AGENT, UA_WEB)
            .send()
            .await?;

        let mut enc = enc0;
        let mut openc = openc0;
        let mut course_id_s = courseid;
        let mut clazzid_s = clazzid;
        let mut cpi_out = cpi_s;

        if let Some(loc) = tr
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|v| v.to_str().ok())
        {
            if let Ok(u) = Url::parse(loc).or_else(|_| {
                Url::parse(&format!("https://mooc1.chaoxing.com{loc}"))
            }) {
                for (k, v) in u.query_pairs() {
                    match k.as_ref() {
                        "courseId" | "courseid" => course_id_s = v.to_string(),
                        "clazzid" | "classId" => clazzid_s = v.to_string(),
                        "cpi" => cpi_out = v.to_string(),
                        "enc" => enc = v.to_string(),
                        "openc" => openc = v.to_string(),
                        _ => {}
                    }
                }
            }
        }

        Ok(WorkParams {
            course_id: course_id_s,
            clazzid: clazzid_s,
            cpi: cpi_out,
            enc,
            openc,
        })
    }

    pub async fn list_homework(
        &self,
        course_id: i64,
        class_id: i64,
        cpi: i64,
        course_name: &str,
    ) -> Result<Vec<HomeworkItem>> {
        let params = self.get_work_params(course_id, class_id, cpi).await?;

        let resp = self
            .http
            .get(API_GET_ALL_WORK)
            .header(reqwest::header::USER_AGENT, UA_WEB)
            .header("Referer", "https://mooc1.chaoxing.com/")
            .query(&[
                ("classId", params.clazzid.as_str()),
                ("courseId", params.course_id.as_str()),
                ("isdisplaytable", "2"),
                ("mooc", "1"),
                ("ut", "s"),
                ("enc", params.enc.as_str()),
                ("cpi", params.cpi.as_str()),
                ("openc", params.openc.as_str()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let html_text = resp.text().await?;
        Ok(parse_work_list(&html_text, course_name))
    }
}

fn urlencoding_encode(s: &str) -> String {
    // minimal encode for refer query
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn parse_work_list(html_text: &str, course_name: &str) -> Vec<HomeworkItem> {
    let html = Html::parse_document(html_text);
    let li_sel = Selector::parse(".ulDiv li").unwrap();
    let a_sel = Selector::parse(".titTxt a").unwrap();
    let span_pt5 = Selector::parse("span.pt5").unwrap();
    let strong_sel = Selector::parse("strong").unwrap();
    let result_sel = Selector::parse(".titOper span.fl").unwrap();

    let re_end = Regex::new(r"\d{4}[-/]\d{1,2}[-/]\d{1,2}").ok();

    let mut items = Vec::new();
    for li in html.select(&li_sel) {
        let Some(a) = li.select(&a_sel).next() else {
            continue;
        };
        let work_name = a
            .value()
            .attr("title")
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| immediate_text(a));
        let work_url = a.value().attr("href").map(|s| s.to_string());

        // status: last span.pt5 > strong text (worklist)
        let spans: Vec<_> = li.select(&span_pt5).collect();
        let status_raw = spans
            .last()
            .and_then(|s| s.select(&strong_sel).next())
            .map(immediate_text)
            .unwrap_or_default();
        let (status, status_code) = map_status(&status_raw);

        // end time: typically second span.pt5
        let mut end_time = "无限制".to_string();
        if spans.len() >= 2 {
            let t = immediate_text(spans[1]);
            if !t.is_empty() {
                end_time = if let Some(re) = &re_end {
                    re.find(&t)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or(t)
                } else {
                    t
                };
            }
        }

        let result_num = li
            .select(&result_sel)
            .next()
            .map(immediate_text)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "暂无成绩".into());

        items.push(HomeworkItem {
            course_name: course_name.to_string(),
            work_name,
            status: status.to_string(),
            status_code: status_code.to_string(),
            end_time,
            result_num,
            work_url,
        });
    }
    items
}
