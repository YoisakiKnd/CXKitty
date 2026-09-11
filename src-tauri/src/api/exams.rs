//! Exam list from CXKitty `ClassContainer.get_exam_by_index` (phone task-list SSR).

use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use url::Url;

use super::client::{ChaoxingClient, Result};

const PAGE_EXAM_LIST: &str = "https://mooc1-api.chaoxing.com/exam/phone/task-list";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExamItem {
    pub course_name: String,
    pub exam_id: i64,
    pub course_id: i64,
    pub class_id: i64,
    pub cpi: i64,
    pub enc_task: String,
    pub name: String,
    pub status: String,
    pub expire_time: Option<String>,
}

impl ChaoxingClient {
    pub async fn list_exams(
        &self,
        course_id: i64,
        class_id: i64,
        cpi: i64,
        course_name: &str,
    ) -> Result<Vec<ExamItem>> {
        let resp = self
            .http
            .get(PAGE_EXAM_LIST)
            .query(&[
                ("courseId", course_id.to_string()),
                ("classId", class_id.to_string()),
                ("cpi", cpi.to_string()),
            ])
            .send()
            .await?
            .error_for_status()?;

        let html_text = resp.text().await?;
        let html = Html::parse_document(&html_text);
        let ul_sel = Selector::parse("ul.nav").unwrap();
        let li_sel = Selector::parse("li").unwrap();
        let p_sel = Selector::parse("p").unwrap();
        let span_sel = Selector::parse("span").unwrap();
        let fr_sel = Selector::parse("span.fr").unwrap();

        let mut out = Vec::new();
        let Some(ul) = html.select(&ul_sel).next() else {
            return Ok(out);
        };

        for li in ul.select(&li_sel) {
            let Some(data) = li.value().attr("data") else {
                continue;
            };
            let Ok(u) = Url::parse(data).or_else(|_| Url::parse(&format!("https://mooc1.chaoxing.com{data}"))) else {
                continue;
            };
            let mut exam_id: Option<i64> = None;
            let mut enc_task = String::new();
            for (k, v) in u.query_pairs() {
                match k.as_ref() {
                    "taskrefId" => exam_id = v.parse().ok(),
                    "enc_task" => enc_task = v.to_string(),
                    _ => {}
                }
            }
            let Some(exam_id) = exam_id else { continue };
            let name = li
                .select(&p_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_else(|| "未命名考试".into());
            let status = li
                .select(&span_sel)
                .find(|s| !s.value().classes().any(|c| c == "fr"))
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let expire_time = li
                .select(&fr_sel)
                .next()
                .map(|e| e.text().collect::<String>().trim().to_string())
                .filter(|s| !s.is_empty());

            out.push(ExamItem {
                course_name: course_name.to_string(),
                exam_id,
                course_id,
                class_id,
                cpi,
                enc_task,
                name,
                status,
                expire_time,
            });
        }

        Ok(out)
    }
}
