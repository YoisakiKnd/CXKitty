mod api;
mod commands;

use commands::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::login,
            commands::list_saved_sessions,
            commands::use_saved_session,
            commands::start_qr_login,
            commands::poll_qr_login,
            commands::list_courses,
            commands::list_homework,
            commands::list_exams,
            commands::list_chapters,
            commands::list_task_points,
            commands::start_brush,
            commands::stop_brush,
            commands::brush_running,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
