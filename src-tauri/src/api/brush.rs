//! Brush-course (刷课) scheduler: walk unfinished chapters / task points sequentially.

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};

use super::chapters::{ChapterInfo, TaskPointInfo};
use super::client::{ChaoxingClient, ClientError, Result};
use super::courses::CourseInfo;
use super::task_point::attachment::CardContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrushOptions {
    /// Simulated playback speed (CXKitty video.speed, default 1.0).
    pub video_speed: f64,
    /// Seconds between progress reports (CXKitty video.report_rate, default 58).
    pub video_report_rate: i64,
    /// Only process chapters that are not fully finished (default true).
    pub unfinished_only: bool,
    /// Cooldown after each completed task point, in seconds. Matches the
    /// original `config.yml` `video.wait` / `document.wait` / `work.wait` (15).
    #[serde(default = "default_wait_seconds")]
    pub wait_seconds: i64,
}

fn default_wait_seconds() -> i64 {
    15
}

impl Default for BrushOptions {
    fn default() -> Self {
        Self {
            video_speed: 1.0,
            video_report_rate: 58,
            unfinished_only: true,
            wait_seconds: default_wait_seconds(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrushProgressEvent {
    /// log | course | chapter | point | wait | done | error | stopped
    pub kind: String,
    pub message: String,
    pub chapter_label: Option<String>,
    pub point_title: Option<String>,
    pub point_kind: Option<String>,
    pub point_status: Option<String>, // running | done | skipped | failed
    pub playing: Option<i64>,
    pub duration: Option<i64>,
    pub chapter_index: Option<usize>,
    pub chapter_total: Option<usize>,
    pub point_index: Option<usize>,
    pub point_total: Option<usize>,
    /// Current course being processed (multi-course runs).
    pub current_course: Option<String>,
    pub course_index: Option<usize>,
    pub course_total: Option<usize>,
    /// Cooldown countdown, mirroring `WebTask.wait_progress`.
    pub wait_progress: Option<f64>,
    pub wait_progress_text: Option<String>,
}

fn emit(app: &AppHandle, ev: BrushProgressEvent) {
    let _ = app.emit("brush-progress", ev);
}

fn log_ev(app: &AppHandle, message: impl Into<String>) {
    emit(
        app,
        BrushProgressEvent {
            kind: "log".into(),
            message: message.into(),
            chapter_label: None,
            point_title: None,
            point_kind: None,
            point_status: None,
            playing: None,
            duration: None,
            chapter_index: None,
            chapter_total: None,
            ..Default::default()
        },
    );
}

pub struct BrushRunner {
    pub stop: Arc<AtomicBool>,
}

impl BrushRunner {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            stop: Arc::new(AtomicBool::new(false)),
        }
    }

    #[allow(dead_code)]
    pub fn request_stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
    }

    fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Cooldown between task points, mirroring the original
    /// `WebTaskRunner.wait(seconds, message)`: log the message, then tick a
    /// 1-second countdown that drives the 冷却等待 progress bar. Stops early if
    /// the user cancels.
    async fn wait_cooldown(&self, app: &AppHandle, seconds: i64, reason: &str) {
        if seconds <= 0 {
            return;
        }
        log_ev(app, format!("{reason}，等待 {seconds}s"));
        for elapsed in 0..=seconds {
            if self.stopped() {
                break;
            }
            let ratio = (elapsed as f64 / seconds as f64).min(1.0);
            emit(
                app,
                BrushProgressEvent {
                    kind: "wait".into(),
                    message: format!("等待冷却 {elapsed}/{seconds}s"),
                    wait_progress: Some(ratio),
                    wait_progress_text: Some(format!("等待冷却 {elapsed}/{seconds}s")),
                    ..Default::default()
                },
            );
            if elapsed < seconds {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
        // Clear the bar once the cooldown ends.
        emit(
            app,
            BrushProgressEvent {
                kind: "wait".into(),
                message: String::new(),
                wait_progress: Some(0.0),
                wait_progress_text: Some("-".into()),
                ..Default::default()
            },
        );
    }

    /// Run every selected course sequentially (original `ClassSelector` command
    /// semantics: a comma-joined list of course indices).
    pub async fn run(
        &self,
        app: AppHandle,
        client: ChaoxingClient,
        courses: Vec<CourseInfo>,
        puid: i64,
        opts: BrushOptions,
    ) -> Result<()> {
        self.stop.store(false, Ordering::SeqCst);
        let course_total = courses.len();
        log_ev(&app, format!("开始刷课：共 {course_total} 门课程"));

        for (idx, course) in courses.iter().enumerate() {
            if self.stopped() {
                emit(
                    &app,
                    BrushProgressEvent {
                        kind: "stopped".into(),
                        message: "用户停止刷课".into(),
                        current_course: Some(course.name.clone()),
                        ..Default::default()
                    },
                );
                return Ok(());
            }

            emit(
                &app,
                BrushProgressEvent {
                    kind: "course".into(),
                    message: format!("开始处理课程：{} ({}/{})", course.name, idx + 1, course_total),
                    current_course: Some(course.name.clone()),
                    course_index: Some(idx + 1),
                    course_total: Some(course_total),
                    ..Default::default()
                },
            );

            if let Err(e) = self
                .run_course(&app, &client, course, puid, &opts, idx + 1, course_total)
                .await
            {
                log_ev(&app, format!("课程处理失败 {}: {e}", course.name));
            }
        }

        emit(
            &app,
            BrushProgressEvent {
                kind: "done".into(),
                message: format!("刷课结束：共 {course_total} 门课程"),
                course_total: Some(course_total),
                ..Default::default()
            },
        );
        Ok(())
    }

    /// Process one course: walk its chapters and task points.
    async fn run_course(
        &self,
        app: &AppHandle,
        client: &ChaoxingClient,
        course: &CourseInfo,
        puid: i64,
        opts: &BrushOptions,
        course_index: usize,
        course_total: usize,
    ) -> Result<()> {
        log_ev(
            app,
            format!(
                "开始刷课：{} (course={}, class={})",
                course.name, course.course_id, course.class_id
            ),
        );

        let chapters = client
            .list_chapters(course.key, course.cpi, course.course_id, course.class_id, puid)
            .await?;
        let chapter_total = chapters.len();
        log_ev(app, format!("共 {chapter_total} 个章节节点"));

        for (ci, chapter) in chapters.iter().enumerate() {
            if self.stopped() {
                emit(
                    app,
                    BrushProgressEvent {
                        kind: "stopped".into(),
                        message: "用户停止刷课".into(),
                        chapter_label: Some(chapter.label.clone()),
                        point_title: None,
                        point_kind: None,
                        point_status: None,
                        playing: None,
                        duration: None,
                        chapter_index: Some(ci + 1),
                        chapter_total: Some(chapter_total),
                        current_course: Some(course.name.clone()),
                        course_index: Some(course_index),
                        course_total: Some(course_total),
                        ..Default::default()
                    },
                );
                return Ok(());
            }

            let label = format!("{} {}", chapter.label, chapter.name);
            emit(
                app,
                BrushProgressEvent {
                    kind: "chapter".into(),
                    message: format!(
                        "章节 {label} ({}/{})",
                        chapter.point_finished, chapter.point_total
                    ),
                    chapter_label: Some(label.clone()),
                    point_title: None,
                    point_kind: None,
                    point_status: None,
                    playing: None,
                    duration: None,
                    chapter_index: Some(ci + 1),
                    chapter_total: Some(chapter_total),
                    current_course: Some(course.name.clone()),
                    course_index: Some(course_index),
                    course_total: Some(course_total),
                    ..Default::default()
                },
            );

            if opts.unfinished_only && is_chapter_finished(chapter) {
                log_ev(app, format!("已完成，跳过：{label}"));
                continue;
            }

            // Skip empty structural nodes with no jobs
            if chapter.jobs == 0 && chapter.point_total == 0 {
                continue;
            }

            let points = match client.list_task_points(course.course_id, chapter).await {
                Ok(p) => p,
                Err(e) => {
                    log_ev(app, format!("拉取任务点失败 {label}: {e}"));
                    continue;
                }
            };

            if points.is_empty() {
                continue;
            }

            let point_total = points.len();
            let mut refresh_once = true;

            for (pi, point) in points.iter().enumerate() {
                if self.stopped() {
                    emit(
                        app,
                        BrushProgressEvent {
                            kind: "stopped".into(),
                            message: "用户停止刷课".into(),
                            chapter_label: Some(label.clone()),
                            point_title: Some(point.title.clone()),
                            point_kind: Some(point.kind.clone()),
                            point_status: Some("skipped".into()),
                            playing: None,
                            duration: None,
                            chapter_index: Some(ci + 1),
                            chapter_total: Some(chapter_total),
                            point_index: Some(pi + 1),
                            point_total: Some(point_total),
                            ..Default::default()
                        },
                    );
                    return Ok(());
                }

                emit(
                    app,
                    BrushProgressEvent {
                        kind: "point".into(),
                        message: format!("处理 {}「{}」", point.kind, point.title),
                        chapter_label: Some(label.clone()),
                        point_title: Some(point.title.clone()),
                        point_kind: Some(point.kind.clone()),
                        point_status: Some("running".into()),
                        playing: None,
                        duration: None,
                        chapter_index: Some(ci + 1),
                        chapter_total: Some(chapter_total),
                        point_index: Some(pi + 1),
                        point_total: Some(point_total),
                        ..Default::default()
                    },
                );

                let ctx = CardContext {
                    course_id: course.course_id,
                    class_id: course.class_id,
                    knowledge_id: chapter.chapter_id,
                    cpi: course.cpi,
                    card_index: point.card_index,
                };

                let mut completed = false;
                match self
                    .run_one(
                        app,
                        client,
                        &ctx,
                        point,
                        puid,
                        &opts,
                        &mut refresh_once,
                        chapter,
                        course.course_id,
                        course.class_id,
                        course.cpi,
                    )
                    .await
                {
                    Ok((msg, status)) => {
                        completed = status == "done";
                        emit(
                            app,
                            BrushProgressEvent {
                                kind: "point".into(),
                                message: msg,
                                chapter_label: Some(label.clone()),
                                point_title: Some(point.title.clone()),
                                point_kind: Some(point.kind.clone()),
                                point_status: Some(status.into()),
                                playing: None,
                                duration: None,
                                chapter_index: Some(ci + 1),
                                chapter_total: Some(chapter_total),
                                point_index: Some(pi + 1),
                                point_total: Some(point_total),
                                ..Default::default()
                            },
                        );
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        if msg.contains("已停止") {
                            emit(
                                app,
                                BrushProgressEvent {
                                    kind: "stopped".into(),
                                    message: msg,
                                    chapter_label: Some(label.clone()),
                                    point_title: Some(point.title.clone()),
                                    point_kind: Some(point.kind.clone()),
                                    point_status: Some("skipped".into()),
                                    playing: None,
                                    duration: None,
                                    chapter_index: Some(ci + 1),
                                    chapter_total: Some(chapter_total),
                                    point_index: Some(pi + 1),
                                    point_total: Some(point_total),
                                    ..Default::default()
                                },
                            );
                            return Ok(());
                        }
                        emit(
                            app,
                            BrushProgressEvent {
                                kind: "point".into(),
                                message: format!("失败：{msg}"),
                                chapter_label: Some(label.clone()),
                                point_title: Some(point.title.clone()),
                                point_kind: Some(point.kind.clone()),
                                point_status: Some("failed".into()),
                                playing: None,
                                duration: None,
                                chapter_index: Some(ci + 1),
                                chapter_total: Some(chapter_total),
                                point_index: Some(pi + 1),
                                point_total: Some(point_total),
                                ..Default::default()
                            },
                        );
                    }
                }

                // Cooldown after a completed point, mirroring the original
                // `WebTaskRunner.wait(config.*_WAIT, "…已完成，等待 Ns")`.
                if completed {
                    self.wait_cooldown(app, opts.wait_seconds, "任务点已完成").await;
                }

                // Small gap between points
                tokio::time::sleep(std::time::Duration::from_millis(800)).await;
            }
        }

        log_ev(app, format!("课程完成：{}", course.name));
        Ok(())
    }

    async fn run_one(
        &self,
        app: &AppHandle,
        client: &ChaoxingClient,
        ctx: &CardContext,
        point: &TaskPointInfo,
        puid: i64,
        opts: &BrushOptions,
        refresh_once: &mut bool,
        chapter: &ChapterInfo,
        course_id: i64,
        class_id: i64,
        cpi: i64,
    ) -> Result<(String, &'static str)> {
        // Quiz / work / exam and other non-brush points: skip without answer APIs.
        match point.kind.as_str() {
            "video" | "document" => {}
            _ => return Ok(("已跳过-非刷课类型".into(), "skipped")),
        }

        // Probe attachment; on 章节未开放 try refresh once then re-fetch next loop.
        let probe = client.fetch_attachment(ctx).await;
        if let Err(ClientError::Api(ref msg)) = probe {
            if msg.contains("章节未开放") {
                if *refresh_once {
                    *refresh_once = false;
                    let _ = client
                        .refresh_chapter(course_id, class_id, chapter.chapter_id, cpi)
                        .await;
                    log_ev(app, "章节未开放，已尝试刷新");
                    return Ok(("章节未开放（已刷新，稍后重试）".into(), "skipped"));
                }
                return Err(ClientError::Api("章节未开放！".into()));
            }
        }
        *refresh_once = true;
        // Drop probe result — complete_* will fetch again (keeps parity with CXKitty).
        let _ = probe?;

        match point.kind.as_str() {
            "video" => {
                let object_id = point
                    .object_id
                    .as_deref()
                    .ok_or_else(|| ClientError::Api("视频缺少 object_id".into()))?;
                let stop = self.stop.clone();
                let app2 = app.clone();
                let title = point.title.clone();
                let (msg, ok) = client
                    .complete_video(
                        ctx,
                        object_id,
                        puid,
                        opts.video_speed,
                        opts.video_report_rate,
                        |playing, duration| {
                            emit(
                                &app2,
                                BrushProgressEvent {
                                    kind: "point".into(),
                                    message: format!(
                                        "播放中 {playing}/{duration}s · {title}"
                                    ),
                                    chapter_label: None,
                                    point_title: Some(title.clone()),
                                    point_kind: Some("video".into()),
                                    point_status: Some("running".into()),
                                    playing: Some(playing),
                                    duration: Some(duration),
                                    chapter_index: None,
                                    chapter_total: None,
                                    ..Default::default()
                                },
                            );
                        },
                        || stop.load(Ordering::SeqCst),
                    )
                    .await?;
                Ok((msg, if ok { "done" } else { "skipped" }))
            }
            "document" => {
                let object_id = point
                    .object_id
                    .as_deref()
                    .ok_or_else(|| ClientError::Api("文档缺少 object_id".into()))?;
                let (msg, ok) = client.complete_document(ctx, object_id).await?;
                Ok((msg, if ok { "done" } else { "skipped" }))
            }
            other => Ok((format!("已跳过-非刷课类型 ({other})"), "skipped")),
        }
    }
}

fn is_chapter_finished(ch: &ChapterInfo) -> bool {
    ch.point_total > 0 && ch.point_finished >= ch.point_total
}
