from __future__ import annotations

import base64
import io
import threading
import webbrowser
from typing import Any

import qrcode
import uvicorn
from fastapi import FastAPI, HTTPException
from fastapi.middleware.cors import CORSMiddleware
from nicegui import ui
from pydantic import BaseModel

from webapp.service import service


app = FastAPI(title="CxKitty Web")
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)


class SessionLoginRequest(BaseModel):
    client_id: str
    phone: str


class PasswordLoginRequest(BaseModel):
    client_id: str
    phone: str
    password: str


class TaskStartRequest(BaseModel):
    client_id: str
    command: str


def _http_error(exc: Exception) -> HTTPException:
    return HTTPException(status_code=400, detail=str(exc))


def qr_url_to_data_uri(qr_url: str) -> str:
    image = qrcode.make(qr_url)
    buffer = io.BytesIO()
    image.save(buffer, format="PNG")
    encoded = base64.b64encode(buffer.getvalue()).decode("ascii")
    return f"data:image/png;base64,{encoded}"


@app.get("/api/sessions")
def api_sessions() -> list[dict[str, Any]]:
    return service.list_saved_sessions()


@app.post("/api/login/session")
def api_login_session(payload: SessionLoginRequest) -> dict[str, Any]:
    try:
        return {"account": service.use_saved_session(payload.client_id, payload.phone)}
    except Exception as exc:
        raise _http_error(exc)


@app.post("/api/login/password")
def api_login_password(payload: PasswordLoginRequest) -> dict[str, Any]:
    try:
        return {"account": service.login_with_password(payload.client_id, payload.phone, payload.password)}
    except Exception as exc:
        raise _http_error(exc)


@app.post("/api/login/qr/start")
def api_qr_start(payload: SessionLoginRequest) -> dict[str, Any]:
    try:
        return service.start_qr_login(payload.client_id)
    except Exception as exc:
        raise _http_error(exc)


@app.get("/api/login/qr/poll")
def api_qr_poll(client_id: str) -> dict[str, Any]:
    try:
        return service.poll_qr_login(client_id)
    except Exception as exc:
        raise _http_error(exc)


@app.get("/api/classes")
def api_classes(client_id: str) -> list[dict[str, Any]]:
    try:
        return service.fetch_classes(client_id)
    except Exception as exc:
        raise _http_error(exc)


@app.post("/api/tasks/start")
def api_start_task(payload: TaskStartRequest) -> dict[str, Any]:
    try:
        return service.start_task(payload.client_id, payload.command)
    except Exception as exc:
        raise _http_error(exc)


@app.get("/api/tasks/{task_id}")
def api_get_task(task_id: str) -> dict[str, Any]:
    try:
        return service.get_task(task_id)
    except Exception as exc:
        raise _http_error(exc)


@ui.page("/")
def index_page() -> None:
    client = service.create_client()
    state = {"client_id": client.client_id, "task_id": None, "qr_enabled": False}

    ui.add_head_html(
        """
        <style>
            body { background: linear-gradient(135deg, #f4efe7 0%, #edf5ff 100%); }
            .app-shell { max-width: 1320px; margin: 0 auto; padding: 24px; }
            .card-shell {
                border-radius: 24px;
                box-shadow: 0 20px 55px rgba(16, 24, 38, 0.08);
                background: rgba(255, 255, 255, 0.92);
                overflow: hidden;
            }
            .top-card {
                min-height: 640px;
            }
            .muted { color: #5d6878; }
            .course-table {
                max-height: 360px;
                overflow: auto;
                border-radius: 18px;
            }
            .status-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }
            .status-item {
                background: #f7fafc;
                border: 1px solid rgba(26, 54, 93, 0.08);
                border-radius: 16px;
                padding: 12px 14px;
            }
            .progress-card {
                background: #f7fafc;
                border: 1px solid rgba(26, 54, 93, 0.08);
                border-radius: 18px;
                padding: 14px;
            }
            .status-k { color: #6b7280; font-size: 12px; }
            .status-v { color: #111827; font-size: 15px; font-weight: 600; margin-top: 4px; }
            .state-badge {
                display: inline-flex;
                align-items: center;
                gap: 8px;
                border-radius: 999px;
                padding: 8px 14px;
                font-size: 13px;
                font-weight: 700;
                letter-spacing: 0.02em;
            }
            .state-idle {
                background: #e5e7eb;
                color: #374151;
            }
            .state-running {
                background: #dbeafe;
                color: #1d4ed8;
            }
            .state-success {
                background: #dcfce7;
                color: #15803d;
            }
            .state-failed {
                background: #fee2e2;
                color: #b91c1c;
            }
            .chapter-list {
                display: grid;
                gap: 10px;
                height: 520px;
                overflow: auto;
                padding-right: 4px;
                align-content: start;
            }
            .chapter-item {
                border-radius: 16px;
                padding: 12px 14px;
                border: 1px solid rgba(15, 23, 42, 0.08);
                background: #f8fbff;
            }
            .chapter-item.active {
                background: linear-gradient(135deg, #dbeafe 0%, #eff6ff 100%);
                border-color: rgba(59, 130, 246, 0.45);
            }
            .chapter-item.done {
                background: linear-gradient(135deg, #dcfce7 0%, #f0fdf4 100%);
                border-color: rgba(34, 197, 94, 0.35);
            }
            .chapter-item.partial {
                background: linear-gradient(135deg, #fef3c7 0%, #fff7ed 100%);
                border-color: rgba(245, 158, 11, 0.35);
            }
            .chapter-top {
                display: flex;
                justify-content: space-between;
                gap: 12px;
                align-items: center;
            }
            .chapter-label {
                font-size: 12px;
                font-weight: 700;
                color: #475569;
            }
            .chapter-name {
                font-size: 14px;
                font-weight: 600;
                color: #0f172a;
                margin-top: 4px;
            }
            .chapter-progress {
                font-size: 12px;
                color: #64748b;
                white-space: nowrap;
            }
            .log-box textarea {
                height: 520px !important;
                min-height: 520px !important;
                max-height: 520px !important;
                font-family: Consolas, "Courier New", monospace !important;
                background: #101826 !important;
                color: #d7e4ff !important;
                border-radius: 18px !important;
                padding: 14px !important;
                resize: none !important;
                overflow: auto !important;
            }
        </style>
        """
    )

    def notify_error(exc: Exception) -> None:
        ui.notify(str(exc), type="negative", multi_line=True)

    def selected_command() -> str:
        selected = sorted(int(row["index"]) for row in classes_table.selected or [])
        return ",".join(str(index) for index in selected)

    def render_task_badge(state_text: str, detail: str = "") -> str:
        normalized = state_text.upper()
        if normalized == "RUNNING":
            css_class = "state-running"
            label = "运行中"
        elif normalized == "SUCCESS":
            css_class = "state-success"
            label = "已完成"
        elif normalized == "FAILED":
            css_class = "state-failed"
            label = "失败"
        else:
            css_class = "state-idle"
            label = "未启动"
        suffix = f" · {detail}" if detail else ""
        return f'<div class="state-badge {css_class}">{label}{suffix}</div>'

    def refresh_selected_hint() -> None:
        command = selected_command()
        selected_hint.set_text(f"当前勾选: {command}" if command else "当前勾选: 无")

    def set_account(account: dict[str, Any]) -> None:
        account_panel.set_content(
            f"""
            <div><strong>{account['name']}</strong></div>
            <div>手机号: {account['phone']}</div>
            <div>学校: {account['school']}</div>
            <div>puid: {account['puid']}</div>
            """
        )

    def load_sessions() -> None:
        sessions = service.list_saved_sessions()
        session_select.options = {
            row["phone"]: f"{row['masked_phone']} | {row['masked_name']} | puid={row['puid']}"
            for row in sessions
        }
        if sessions and not session_select.value:
            session_select.value = sessions[0]["phone"]

    def refresh_classes() -> None:
        try:
            classes_table.rows = service.fetch_classes(state["client_id"])
            classes_table.update()
            refresh_selected_hint()
        except Exception as exc:
            notify_error(exc)

    def use_saved_session() -> None:
        try:
            if not session_select.value:
                raise ValueError("请先选择本地会话")
            account = service.use_saved_session(state["client_id"], str(session_select.value))
            set_account(account)
            refresh_classes()
            ui.notify("本地会话已载入", type="positive")
        except Exception as exc:
            notify_error(exc)

    def login_with_password() -> None:
        try:
            account = service.login_with_password(
                state["client_id"],
                str(phone_input.value or "").strip(),
                str(password_input.value or ""),
            )
            set_account(account)
            refresh_classes()
            ui.notify("登录成功", type="positive")
        except Exception as exc:
            notify_error(exc)

    def start_qr() -> None:
        try:
            qr_data = service.start_qr_login(state["client_id"])
            qr_image.set_source(qr_url_to_data_uri(qr_data["qr_url"]))
            qr_dialog.open()
            state["qr_enabled"] = True
        except Exception as exc:
            notify_error(exc)

    def poll_qr() -> None:
        if not state["qr_enabled"]:
            return
        try:
            result = service.poll_qr_login(state["client_id"])
            if result["state"] == "success":
                state["qr_enabled"] = False
                qr_dialog.close()
                set_account(result["account"])
                refresh_classes()
                ui.notify("二维码登录成功", type="positive")
        except Exception:
            pass

    def start_task() -> None:
        try:
            command = selected_command()
            if not command:
                raise ValueError("请先勾选课程")
            result = service.start_task(state["client_id"], command)
            state["task_id"] = result["task_id"]
            task_state.set_content(render_task_badge("RUNNING", command))
            log_area.set_value("任务已启动，正在等待日志...\n")
            ui.notify("任务已启动", type="positive")
        except Exception as exc:
            notify_error(exc)

    def refresh_task() -> None:
        if not state["task_id"]:
            return
        try:
            task = service.get_task(state["task_id"])
            progress = task.get("progress", {})
            detail = progress.get("current_task") or progress.get("current_chapter") or ""
            task_state.set_content(render_task_badge(task["state"], detail if task["state"] == "RUNNING" else ""))
            log_area.set_value(task["output"] or "任务已启动，正在等待日志...\n")
            progress_course.set_content(progress.get("current_course") or "-")
            progress_chapter.set_content(progress.get("current_chapter") or "-")
            progress_task_type.set_content(progress.get("current_task_type") or "-")
            progress_task_name.set_content(progress.get("current_task") or "-")
            progress_chapter_idx.set_content(
                f"{progress.get('chapter_index', 0)} / {progress.get('chapter_total', 0)}"
            )
            progress_point_idx.set_content(
                f"{progress.get('point_index', 0)} / {progress.get('point_total', 0)}"
            )
            progress_course_idx.set_content(
                f"{progress.get('completed_courses', 0)} / {progress.get('total_courses', 0)}"
            )
            task_progress_bar.set_value(float(progress.get("task_progress", 0.0) or 0.0))
            task_progress_text.set_text(progress.get("task_progress_text") or "-")
            wait_progress_bar.set_value(float(progress.get("wait_progress", 0.0) or 0.0))
            wait_progress_text.set_text(progress.get("wait_progress_text") or "-")
            chapter_html = []
            for item in progress.get("chapter_items", []):
                chapter_html.append(
                    f"""
                    <div class="chapter-item {item['status']}">
                        <div class="chapter-top">
                            <div class="chapter-label">{item['label']}</div>
                            <div class="chapter-progress">{item['progress_text']}</div>
                        </div>
                        <div class="chapter-name">{item['name']}</div>
                    </div>
                    """
                )
            chapter_panel.set_content(
                "".join(chapter_html) if chapter_html else '<div class="muted">章节状态会在任务开始后显示</div>'
            )
            ui.run_javascript(
                f'''(() => {{
                    const root = document.getElementById("{log_area.id}");
                    const textarea = root ? root.querySelector("textarea") : null;
                    if (textarea) textarea.scrollTop = textarea.scrollHeight;
                }})()'''
            )
        except Exception as exc:
            notify_error(exc)
            state["task_id"] = None

    with ui.column().classes("app-shell w-full gap-6"):
        with ui.row().classes("w-full items-center justify-between"):
            with ui.column().classes("gap-1"):
                ui.label("CxKitty Web").classes("text-3xl font-bold")
                ui.label("完全重构为 Web 工作流，不再依赖旧 CLI/TUI 包装层").classes("muted")
            ui.button("刷新本地会话", on_click=load_sessions).props("outline color=dark")

        with ui.row().classes("w-full gap-6 items-start wrap xl:no-wrap"):
            with ui.card().classes("card-shell top-card w-full xl:max-w-sm p-6 gap-4"):
                ui.label("账号").classes("text-xl font-semibold")
                session_select = ui.select({}, label="本地会话").classes("w-full")
                with ui.row().classes("w-full gap-2"):
                    ui.button("载入会话", on_click=use_saved_session).classes("flex-1")
                    ui.button("扫码登录", on_click=start_qr).props("outline").classes("flex-1")
                phone_input = ui.input("手机号").classes("w-full")
                password_input = ui.input("密码", password=True, password_toggle_button=True).classes("w-full")
                ui.button("密码登录", on_click=login_with_password).classes("w-full")
                account_panel = ui.html("<div>未登录</div>").classes("w-full rounded-xl bg-slate-50 p-4 text-sm")

            with ui.card().classes("card-shell top-card w-full p-6 gap-4 flex-1"):
                ui.label("任务").classes("text-xl font-semibold")
                classes_table = ui.table(
                    columns=[
                        {"name": "index", "label": "序号", "field": "index", "align": "left"},
                        {"name": "name", "label": "课程名", "field": "name", "align": "left"},
                        {"name": "teacher_name", "label": "老师", "field": "teacher_name", "align": "left"},
                        {"name": "course_id", "label": "课程ID", "field": "course_id", "align": "left"},
                        {"name": "state", "label": "状态", "field": "state", "align": "left"},
                    ],
                    rows=[],
                    row_key="index",
                    selection="multiple",
                    pagination=12,
                ).classes("course-table w-full")
                classes_table.on("selection", lambda _: refresh_selected_hint())
                selected_hint = ui.label("当前勾选: 无").classes("muted")
                with ui.row().classes("w-full items-center gap-3"):
                    ui.button("启动任务", on_click=start_task)
                    task_state = ui.html(render_task_badge("IDLE")).classes("shrink-0")

        with ui.row().classes("w-full gap-6 items-start wrap xl:no-wrap"):
            with ui.card().classes("card-shell w-full xl:max-w-md p-6 gap-4"):
                ui.label("章节进度").classes("text-xl font-semibold")
                chapter_panel = ui.html('<div class="muted">章节状态会在任务开始后显示</div>').classes("chapter-list w-full")
            with ui.card().classes("card-shell w-full p-6 gap-4 flex-1"):
                ui.label("任务进度").classes("text-xl font-semibold")
                with ui.element("div").classes("status-grid w-full"):
                    with ui.element("div").classes("status-item"):
                        ui.label("当前课程").classes("status-k")
                        progress_course = ui.html("-").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("当前章节").classes("status-k")
                        progress_chapter = ui.html("-").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("任务类型").classes("status-k")
                        progress_task_type = ui.html("-").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("当前任务").classes("status-k")
                        progress_task_name = ui.html("-").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("课程进度").classes("status-k")
                        progress_course_idx = ui.html("0 / 0").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("章节进度").classes("status-k")
                        progress_chapter_idx = ui.html("0 / 0").classes("status-v")
                    with ui.element("div").classes("status-item"):
                        ui.label("任务点进度").classes("status-k")
                        progress_point_idx = ui.html("0 / 0").classes("status-v")
                with ui.row().classes("w-full gap-4 wrap lg:no-wrap"):
                    with ui.element("div").classes("progress-card w-full"):
                        ui.label("当前任务进度").classes("status-k")
                        task_progress_text = ui.label("-").classes("status-v")
                        task_progress_bar = ui.linear_progress(value=0.0, show_value=False).classes("w-full")
                    with ui.element("div").classes("progress-card w-full"):
                        ui.label("冷却等待").classes("status-k")
                        wait_progress_text = ui.label("-").classes("status-v")
                        wait_progress_bar = ui.linear_progress(value=0.0, show_value=False, color="amber").classes("w-full")

        with ui.expansion("运行日志", icon="terminal", value=False).classes("card-shell w-full"):
            with ui.column().classes("w-full p-6 gap-4"):
                ui.label("任务日志默认折叠，需要时再展开查看").classes("muted")
                log_area = ui.textarea(value="任务日志会显示在这里").classes("log-box w-full").props("readonly outlined")
                log_area.props('input-style="height: 520px"')

    with ui.dialog() as qr_dialog, ui.card().classes("card-shell p-5 items-center gap-3"):
        ui.label("扫码登录").classes("text-lg font-semibold")
        qr_image = ui.image().classes("w-64 h-64")
        ui.button("关闭", on_click=qr_dialog.close).props("outline")

    load_sessions()
    ui.timer(2.0, poll_qr)
    ui.timer(2.0, refresh_task)


ui.run_with(app, title="CxKitty Web")


if __name__ == "__main__":
    threading.Timer(1.2, lambda: webbrowser.open("http://127.0.0.1:2333/")).start()
    uvicorn.run("app:app", host="0.0.0.0", port=2333, reload=False)
