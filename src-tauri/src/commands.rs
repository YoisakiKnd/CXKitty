//! Tauri commands — single global session toolbox.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, State};

use crate::api::{
    AccountInfo, BrushOptions, BrushRunner, ChaoxingClient, ChapterInfo, CourseInfo, ExamItem,
    HomeworkItem, QrPollResult, QrSession, QrStartResult, SavedSession, TaskPointInfo,
};

pub struct AppState {
    pub client: Arc<Mutex<ChaoxingClient>>,
    pub account: Arc<Mutex<Option<AccountInfo>>>,
    pub brush_running: Arc<AtomicBool>,
    pub brush_stop: Arc<AtomicBool>,
    /// Active QR login session (mirrors main's `WebClientContext.qr_active`).
    pub qr_session: Arc<Mutex<Option<QrSession>>>,
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
            qr_session: Arc::new(Mutex::new(None)),
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
    // Persist so the session can be reused later (main: _save_session).
    crate::api::sessions::save_session(&client, &acc.phone, acc.puid, &acc.name, Some(&password));
    Ok(acc)
}

/* ------------------------------------------------------------ saved sessions */

/// `GET /api/sessions` — list locally saved sessions with masked identifiers.
#[tauri::command]
pub async fn list_saved_sessions() -> Result<Vec<SavedSession>, String> {
    Ok(crate::api::sessions::list_saved_sessions())
}

/// `POST /api/login/session` — restore a saved session, re-logging in with the
/// stored password when the cookies have expired.
#[tauri::command]
pub async fn use_saved_session(
    state: State<'_, AppState>,
    phone: String,
) -> Result<AccountInfo, String> {
    let client = state.client_clone();
    let session = crate::api::sessions::read_session(&phone)
        .ok_or_else(|| "未找到对应会话".to_string())?;

    client
        .ck_load(&session.ck)
        .map_err(|e| e.to_string())?;

    // Validate the restored cookies; fall back to the stored password.
    match client.fetch_account_info().await {
        Ok(acc) => {
            *state.account.lock() = Some(acc.clone());
            Ok(acc)
        }
        Err(_) => {
            let Some(passwd) = session.passwd.clone() else {
                return Err("会话已失效，且本地没有保存密码".into());
            };
            let acc = client
                .login_passwd(&session.phone, &passwd)
                .await
                .map_err(|e| format!("自动重登失败: {e}"))?;
            crate::api::sessions::save_session(
                &client,
                &acc.phone,
                acc.puid,
                &acc.name,
                Some(&passwd),
            );
            *state.account.lock() = Some(acc.clone());
            Ok(acc)
        }
    }
}

/* ----------------------------------------------------------------- QR login */

/// `POST /api/login/qr/start` — fetch the QR image and remember the session.
#[tauri::command]
pub async fn start_qr_login(state: State<'_, AppState>) -> Result<QrStartResult, String> {
    let client = state.client_clone();
    let result = client.qr_get().await.map_err(|e| e.to_string())?;
    *state.qr_session.lock() = Some(QrSession {
        uuid: extract_qr_field(&result.qr_url, "uuid"),
        enc: extract_qr_field(&result.qr_url, "enc"),
    });
    Ok(result)
}

/// `GET /api/login/qr/poll` — poll once; on success the account is stored and
/// the session persisted (without a password, exactly like the original).
#[tauri::command]
pub async fn poll_qr_login(state: State<'_, AppState>) -> Result<QrPollResult, String> {
    let session = state
        .qr_session
        .lock()
        .clone()
        .ok_or_else(|| "二维码登录尚未开始".to_string())?;
    let client = state.client_clone();

    let (ok, detail) = client
        .login_qr(&session)
        .await
        .map_err(|e| e.to_string())?;

    if !ok {
        return Ok(QrPollResult {
            state: "pending".into(),
            detail,
            account: None,
        });
    }

    let acc = client.fetch_account_info().await.map_err(|e| e.to_string())?;
    crate::api::sessions::save_session(&client, &acc.phone, acc.puid, &acc.name, None);
    *state.account.lock() = Some(acc.clone());
    *state.qr_session.lock() = None;
    Ok(QrPollResult {
        state: "success".into(),
        detail,
        account: Some(acc),
    })
}

/// Pull `uuid` / `enc` back out of the composed `toauthlogin` URL.
fn extract_qr_field(url: &str, key: &str) -> String {
    url.split(['?', '&'])
        .find_map(|part| part.strip_prefix(&format!("{key}=")))
        .unwrap_or_default()
        .to_string()
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
    courses: Vec<CourseInfo>,
    options: Option<BrushOptions>,
) -> Result<BrushStartResult, String> {
    if state.brush_running.load(Ordering::SeqCst) {
        return Err("刷课任务已在运行".into());
    }
    if courses.is_empty() {
        return Err("请先勾选课程".into());
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
        let result = runner.run(app.clone(), client, courses, puid, opts).await;
        running.store(false, Ordering::SeqCst);
        if let Err(e) = result {
            let _ = app.emit(
                "brush-progress",
                BrushProgressEvent {
                    kind: "error".into(),
                    message: format!("刷课异常结束: {e}"),
                    ..Default::default()
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
