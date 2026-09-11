//! Tauri commands — single global session toolbox.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::api::{
    AccountInfo, BrushOptions, BrushRunner, ChaoxingClient, ChapterInfo, CourseInfo, ExamItem,
    HomeworkItem, TaskPointInfo,
};

pub struct AppState {
    pub client: Arc<Mutex<ChaoxingClient>>,
    pub account: Arc<Mutex<Option<AccountInfo>>>,
    pub brush_running: Arc<AtomicBool>,
    pub brush_stop: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(
                ChaoxingClient::new().expect("failed to build HTTP client"),
            )),
            account: Arc::new(Mutex::new(None)),
            brush_running: Arc::new(AtomicBool::new(false)),
            brush_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    fn client_clone(&self) -> ChaoxingClient {
        self.client.lock().clone()
    }

    fn require_puid(&self) -> Result<i64, String> {
        self.account
            .lock()
            .as_ref()
            .map(|a| a.puid)
            .ok_or_else(|| "请先登录".to_string())
    }
}

#[derive(Debug, Serialize)]
pub struct BrushStartResult {
    pub started: bool,
    pub message: String,
}

#[tauri::command]
pub async fn login(
    state: State<'_, AppState>,
    phone: String,
    password: String,
) -> Result<AccountInfo, String> {
    let client = state.client_clone();
    let acc = client
        .login_passwd(&phone, &password)
        .await
        .map_err(|e| e.to_string())?;
    *state.account.lock() = Some(acc.clone());
    Ok(acc)
}

#[tauri::command]
pub async fn list_courses(state: State<'_, AppState>) -> Result<Vec<CourseInfo>, String> {
    let client = state.client_clone();
    client.list_courses().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_homework(
    state: State<'_, AppState>,
    course_id: i64,
    class_id: i64,
    cpi: i64,
    course_name: String,
) -> Result<Vec<HomeworkItem>, String> {
    let client = state.client_clone();
    client
        .list_homework(course_id, class_id, cpi, &course_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_exams(
    state: State<'_, AppState>,
    course_id: i64,
    class_id: i64,
    cpi: i64,
    course_name: String,
) -> Result<Vec<ExamItem>, String> {
    let client = state.client_clone();
    client
        .list_exams(course_id, class_id, cpi, &course_name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_chapters(
    state: State<'_, AppState>,
    course_id: i64,
    class_id: i64,
    cpi: i64,
    key: i64,
) -> Result<Vec<ChapterInfo>, String> {
    let puid = state.require_puid()?;
    let client = state.client_clone();
    client
        .list_chapters(key, cpi, course_id, class_id, puid)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_task_points(
    state: State<'_, AppState>,
    course_id: i64,
    chapter: ChapterInfo,
) -> Result<Vec<TaskPointInfo>, String> {
    let client = state.client_clone();
    client
        .list_task_points(course_id, &chapter)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_brush(
    app: AppHandle,
    state: State<'_, AppState>,
    course: CourseInfo,
    options: Option<BrushOptions>,
) -> Result<BrushStartResult, String> {
    if state.brush_running.load(Ordering::SeqCst) {
        return Err("刷课任务已在运行".into());
    }
    let puid = state.require_puid()?;
    let client = state.client_clone();
    let opts = options.unwrap_or_default();

    state.brush_stop.store(false, Ordering::SeqCst);
    state.brush_running.store(true, Ordering::SeqCst);

    let running = state.brush_running.clone();
    let stop_flag = state.brush_stop.clone();

    tauri::async_runtime::spawn(async move {
        use crate::api::brush::BrushProgressEvent;
        use tauri::Emitter;
        let runner = BrushRunner {
            stop: stop_flag.clone(),
        };
        let result = runner.run(app.clone(), client, course, puid, opts).await;
        running.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            let _ = app.emit(
                "brush-progress",
                BrushProgressEvent {
                    kind: "error".into(),
                    message: format!("刷课异常结束: {e}"),
                    chapter_label: None,
                    point_title: None,
                    point_kind: None,
                    point_status: None,
                    playing: None,
                    duration: None,
                    chapter_index: None,
                    chapter_total: None,
                    point_index: None,
                    point_total: None,
                },
            );
        }
    });

    Ok(BrushStartResult {
        started: true,
        message: "刷课已开始".into(),
    })
}

#[tauri::command]
pub async fn stop_brush(state: State<'_, AppState>) -> Result<BrushStartResult, String> {
    if !state.brush_running.load(Ordering::SeqCst) {
        return Ok(BrushStartResult {
            started: false,
            message: "当前没有运行中的刷课任务".into(),
        });
    }
    state.brush_stop.store(true, Ordering::SeqCst);
    Ok(BrushStartResult {
        started: false,
        message: "已请求停止刷课".into(),
    })
}

#[tauri::command]
pub async fn brush_running(state: State<'_, AppState>) -> Result<bool, String> {
    Ok(state.brush_running.load(Ordering::SeqCst))
}
