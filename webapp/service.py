from __future__ import annotations

import json
import threading
import time
import traceback
import uuid
from collections import deque
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import config
from cxapi import (
    ChaoXingAPI,
    ChapterContainer,
    ClassSelector,
    ExamDto,
    PointDocumentDto,
    PointVideoDto,
    PointWorkDto,
)
from cxapi.exception import ChapterNotOpened, TaskPointError
from logger import Logger, set_web_log_hook
from resolver import DocumetResolver, QuestionResolver


def _dict_to_ck(dict_ck: dict[str, str]) -> str:
    return "".join(f"{k}={v};" for k, v in dict_ck.items())


def _ck_to_dict(ck: str) -> dict[str, str]:
    result = {}
    for field in ck.strip().split(";"):
        if not field:
            continue
        key, value = field.split("=")
        result[key] = value
    return result


def _mask_name(name: str) -> str:
    return name[0] + ("*" * (len(name) - 2) + name[-1] if len(name) > 2 else "*")


def _mask_phone(phone: str) -> str:
    return phone[:3] + "****" + phone[-4:]


def _sessions_load() -> list[dict[str, Any]]:
    sessions = []
    if not config.SESSIONS_PATH.is_dir():
        return sessions
    for file in config.SESSIONS_PATH.iterdir():
        if file.suffix != ".json":
            continue
        with open(file, "r", encoding="utf8") as fp:
            sessions.append(json.load(fp))
    return sessions


def _save_session(ck: dict[str, str], acc: Any, passwd: str | None = None) -> None:
    if not config.SESSIONS_PATH.is_dir():
        config.SESSIONS_PATH.mkdir(parents=True)
    file_path = Path(config.SESSIONS_PATH) / f"{acc.phone}.json"
    with open(file_path, "w", encoding="utf8") as fp:
        json.dump(
            {
                "phone": acc.phone,
                "puid": acc.puid,
                "passwd": passwd,
                "name": acc.name,
                "ck": _dict_to_ck(ck),
            },
            fp,
            ensure_ascii=False,
        )


@dataclass
class WebClientContext:
    client_id: str
    api: ChaoXingAPI = field(default_factory=ChaoXingAPI)
    classes: Any | None = None
    qr_active: bool = False


@dataclass
class WebTask:
    task_id: str
    client_id: str
    phone: str
    command: str
    state: str = "PENDING"
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    log_lines: deque[str] = field(default_factory=lambda: deque(maxlen=5000))
    current_course: str = ""
    current_chapter: str = ""
    current_task: str = ""
    current_task_type: str = ""
    chapter_index: int = 0
    chapter_total: int = 0
    point_index: int = 0
    point_total: int = 0
    completed_courses: int = 0
    total_courses: int = 0
    recent_events: deque[str] = field(default_factory=lambda: deque(maxlen=12))
    task_progress: float = 0.0
    task_progress_text: str = "-"
    wait_progress: float = 0.0
    wait_progress_text: str = "-"
    chapter_items: list[dict[str, Any]] = field(default_factory=list)

    def append_log(self, text: str) -> None:
        for line in str(text).splitlines():
            self.log_lines.append(line)
            self.recent_events.append(line)
        self.updated_at = time.time()

    def dump_log(self) -> str:
        return "\n".join(self.log_lines)

    def progress(self) -> dict[str, Any]:
        return {
            "current_course": self.current_course,
            "current_chapter": self.current_chapter,
            "current_task": self.current_task,
            "current_task_type": self.current_task_type,
            "chapter_index": self.chapter_index,
            "chapter_total": self.chapter_total,
            "point_index": self.point_index,
            "point_total": self.point_total,
            "completed_courses": self.completed_courses,
            "total_courses": self.total_courses,
            "recent_events": list(self.recent_events),
            "task_progress": self.task_progress,
            "task_progress_text": self.task_progress_text,
            "wait_progress": self.wait_progress,
            "wait_progress_text": self.wait_progress_text,
            "chapter_items": self.chapter_items,
        }


class WebTaskRunner:
    def __init__(self, task: WebTask, api: ChaoXingAPI) -> None:
        self.task = task
        self.api = api
        self.logger = Logger("WebTaskRunner")

    def log(self, message: str) -> None:
        self.task.append_log(message)

    def sync_chapter_items(self, chapter: ChapterContainer, active_index: int | None = None) -> None:
        items = []
        for index, item in enumerate(chapter.chapters):
            if item.point_total > 0 and item.point_finished == item.point_total:
                status = "done"
            elif index == active_index:
                status = "active"
            elif item.point_finished > 0:
                status = "partial"
            else:
                status = "pending"
            items.append(
                {
                    "label": item.label,
                    "name": item.name,
                    "progress_text": f"{item.point_finished}/{item.point_total}",
                    "status": status,
                }
            )
        self.task.chapter_items = items
        self.task.updated_at = time.time()

    def wait(self, seconds: int, message: str) -> None:
        self.log(message)
        if seconds <= 0:
            self.task.wait_progress = 0.0
            self.task.wait_progress_text = "-"
            return
        for elapsed in range(seconds + 1):
            ratio = min(elapsed / seconds, 1.0)
            self.task.wait_progress = ratio
            self.task.wait_progress_text = f"等待冷却 {elapsed}/{seconds}s"
            self.task.updated_at = time.time()
            if elapsed < seconds:
                time.sleep(1)
        self.task.wait_progress = 0.0
        self.task.wait_progress_text = "-"

    def on_captcha_after(self, times: int) -> None:
        self.log(f"正在识别验证码，第 {times} 次...")

    def on_captcha_before(self, status: bool, code: str) -> None:
        if status:
            self.log(f"验证码识别成功，提交通过: {code}")
        else:
            self.log(f"验证码识别成功但提交错误，准备重试: {code}")

    def on_face_detection_after(self, orig_url: str) -> None:
        self.log(f"正在准备人脸识别: {orig_url}")

    def on_face_detection_before(self, object_id: str, image_path: Path) -> None:
        self.log(f"人脸识别提交成功: objectId={object_id} path={image_path}")

    def run(self, classes: Any) -> None:
        self.task.state = "RUNNING"
        self.task.updated_at = time.time()
        self.task.total_courses = len(ClassSelector(self.task.command, classes))
        set_web_log_hook(self.task.append_log)
        self.api.session.reg_captcha_after(self.on_captcha_after)
        self.api.session.reg_captcha_before(self.on_captcha_before)
        self.api.session.reg_face_after(self.on_face_detection_after)
        self.api.session.reg_face_before(self.on_face_detection_before)
        self.log(f"任务开始: {self.task.command}")
        try:
            if config.FETCH_UPLOADED_FACE is True:
                if face_url := self.api.fetch_face():
                    self.api.save_face(face_url, config.FACE_PATH)
                    self.log("已同步账号预上传人脸")

            for task_obj in ClassSelector(self.task.command, classes):
                if isinstance(task_obj, ChapterContainer):
                    self.run_chapter(task_obj)
                    self.task.completed_courses += 1
                elif isinstance(task_obj, ExamDto):
                    self.run_exam(task_obj)
                    self.task.completed_courses += 1
                elif isinstance(task_obj, list):
                    self.log("当前 Web 版暂不支持在运行中二次选择考试，请使用高级命令直接指定考试。")

            self.task.state = "SUCCESS"
            self.log("任务执行完成")
        except Exception:
            self.task.state = "FAILED"
            self.log("任务执行失败")
            self.log(traceback.format_exc())
        finally:
            self.task.updated_at = time.time()
            set_web_log_hook(None)

    def run_chapter(self, chapter: ChapterContainer) -> None:
        self.task.current_course = chapter.name
        self.task.current_chapter = ""
        self.task.current_task = ""
        self.task.current_task_type = ""
        self.task.task_progress = 0.0
        self.task.task_progress_text = "-"
        self.task.chapter_total = len(chapter)
        self.task.chapter_items = []
        self.log(f"开始处理课程: {chapter.name}")
        chapter.fetch_point_status()
        self.sync_chapter_items(chapter)
        for index in range(len(chapter)):
            chapter_meta = chapter.chapters[index]
            self.task.chapter_index = index + 1
            self.task.current_chapter = f"{chapter_meta.label} {chapter_meta.name}"
            self.sync_chapter_items(chapter, index)
            self.log(
                f"章节 {chapter_meta.label} {chapter_meta.name} "
                f"({chapter_meta.point_finished}/{chapter_meta.point_total})"
            )
            if chapter.is_finished(index) and config.WORK["export"] is False:
                self.log("该章节已完成，跳过")
                continue

            refresh_flag = True
            points = chapter[index]
            self.task.point_total = len(points)
            for point_offset, task_point in enumerate(points, start=1):
                self.task.point_index = point_offset
                try:
                    task_point.fetch_attachment()
                except ChapterNotOpened:
                    if refresh_flag:
                        chapter.refresh_chapter(max(index - 1, 0))
                        refresh_flag = False
                        self.log("检测到章节未开放，已尝试刷新章节状态")
                        continue
                    raise

                refresh_flag = True
                self.run_task_point(task_point)
                chapter.fetch_point_status()
                self.sync_chapter_items(chapter, index)

        self.log(f"课程完成: {chapter.name}")

    def run_task_point(self, task_point: Any) -> None:
        try:
            if isinstance(task_point, PointWorkDto) and (config.WORK_EN or config.WORK["export"] is True):
                self.task.current_task_type = "章节测验"
                self.task.current_task = getattr(task_point, "title", "") or str(task_point.work_id)
                self.task.task_progress = 0.0
                self.task.task_progress_text = "准备处理章节测验"
                self.log(f"处理章节测验: {self.task.current_task}")
                if config.WORK["export"] is True:
                    task_point.parse_attachment()
                    export_path = config.EXPORT_PATH / f"work_{task_point.work_id}.json"
                    task_point.export(export_path)
                    self.log(f"已导出测验: {export_path}")
                if config.WORK_EN:
                    if not task_point.parse_attachment():
                        self.log("测验附件解析失败，已跳过")
                        return
                    task_point.fetch_all()
                    self.task.task_progress = 0.25
                    self.task.task_progress_text = "测验题目已拉取"
                    resolver = QuestionResolver(
                        exam_dto=task_point,
                        fallback_save=config.WORK["fallback_save"],
                        fallback_fuzzer=config.WORK["fallback_fuzzer"],
                    )
                    resolver.execute()
                    self.task.task_progress = 1.0
                    self.task.task_progress_text = "章节测验完成"
                    self.wait(config.WORK_WAIT, f"测验已完成，等待 {config.WORK_WAIT}s")

            elif isinstance(task_point, PointVideoDto) and config.VIDEO_EN:
                self.task.current_task_type = "视频任务"
                self.task.current_task = getattr(task_point, "title", "")
                self.task.task_progress = 0.0
                self.task.task_progress_text = "准备处理视频"
                self.log(f"处理视频任务: {self.task.current_task}")
                if not task_point.parse_attachment():
                    self.log("视频附件解析失败，已跳过")
                    return
                if not task_point.fetch():
                    self.log("视频任务获取失败，已跳过")
                    return
                self.execute_video_task(task_point)
                self.task.task_progress = 1.0
                self.task.task_progress_text = "视频任务完成"
                self.wait(config.VIDEO_WAIT, f"视频已完成，等待 {config.VIDEO_WAIT}s")

            elif isinstance(task_point, PointDocumentDto) and config.DOCUMENT_EN:
                self.task.current_task_type = "文档任务"
                self.task.current_task = getattr(task_point, "title", "")
                self.task.task_progress = 0.0
                self.task.task_progress_text = "准备处理文档"
                self.log(f"处理文档任务: {self.task.current_task}")
                if not task_point.parse_attachment():
                    self.log("文档附件解析失败，已跳过")
                    return
                resolver = DocumetResolver(document_dto=task_point)
                resolver.execute()
                self.task.task_progress = 1.0
                self.task.task_progress_text = "文档任务完成"
                self.wait(config.DOCUMENT_WAIT, f"文档已完成，等待 {config.DOCUMENT_WAIT}s")
        except (TaskPointError, NotImplementedError) as exc:
            self.log(f"任务点执行异常: {exc.__class__.__name__} {exc}")

    def execute_video_task(self, task_point: PointVideoDto) -> None:
        duration = max(int(task_point.duration), 1)
        speed = config.VIDEO["speed"]
        report_rate = config.VIDEO["report_rate"]
        playing_time = 0
        report_counter = report_rate
        self.log(f"开始播放视频: {task_point.title}")
        while True:
            if report_counter >= report_rate or playing_time >= duration:
                report_counter = 0
                result = task_point.play_report(playing_time)
                if result.get("isPassed") is True:
                    playing_time = duration
                    self.task.task_progress = 1.0
                    self.task.task_progress_text = f"{playing_time}/{duration}s"
                    self.task.updated_at = time.time()
                    self.log("视频播放完毕")
                    return
            self.task.task_progress = min(playing_time / duration, 1.0)
            self.task.task_progress_text = (
                f"{playing_time // 60:02d}:{playing_time % 60:02d} / "
                f"{duration // 60:02d}:{duration % 60:02d}"
            )
            self.task.updated_at = time.time()
            time.sleep(1)
            playing_time = min(playing_time + round(1 * speed), duration)
            report_counter += round(1 * speed)

    def run_exam(self, exam: ExamDto) -> None:
        self.task.current_course = f"考试 {exam.exam_id}"
        self.task.current_chapter = "-"
        self.task.current_task_type = "课程考试"
        self.task.current_task = str(exam.exam_id)
        self.task.task_progress = 0.0
        self.task.task_progress_text = "准备考试"
        self.log(f"开始处理考试: {exam.exam_id}")
        exam.get_meta()
        self.task.current_task = exam.title
        exam.start()
        self.task.task_progress = 0.3
        self.task.task_progress_text = "考试已开始"
        resolver = QuestionResolver(
            exam_dto=exam,
            fallback_save=False,
            fallback_fuzzer=config.EXAM["fallback_fuzzer"],
            persubmit_delay=config.EXAM["persubmit_delay"],
        )
        resolver.execute()
        self.task.task_progress = 1.0
        self.task.task_progress_text = "考试完成"
        self.log(f"考试处理结束: {exam.title}")


class WebAppService:
    def __init__(self) -> None:
        self._clients: dict[str, WebClientContext] = {}
        self._tasks: dict[str, WebTask] = {}
        self._lock = threading.Lock()

    def create_client(self) -> WebClientContext:
        client = WebClientContext(client_id=uuid.uuid4().hex)
        with self._lock:
            self._clients[client.client_id] = client
        return client

    def get_client(self, client_id: str) -> WebClientContext:
        client = self._clients.get(client_id)
        if client is None:
            raise KeyError(f"client not found: {client_id}")
        return client

    def list_saved_sessions(self) -> list[dict[str, Any]]:
        return [
            {
                "phone": session["phone"],
                "puid": session["puid"],
                "name": session["name"],
                "masked_name": _mask_name(session["name"]),
                "masked_phone": _mask_phone(session["phone"]),
                "has_password": session.get("passwd") is not None,
            }
            for session in _sessions_load()
        ]

    def get_account_summary(self, api: ChaoXingAPI) -> dict[str, Any]:
        acc = api.acc
        return {
            "puid": acc.puid,
            "name": acc.name,
            "sex": acc.sex.name,
            "phone": acc.phone,
            "school": acc.school,
            "stu_id": acc.stu_id,
        }

    def use_saved_session(self, client_id: str, phone: str) -> dict[str, Any]:
        client = self.get_client(client_id)
        for session in _sessions_load():
            if session["phone"] != phone:
                continue
            client.api.session.ck_load(_ck_to_dict(session["ck"]))
            if not client.api.accinfo():
                if session.get("passwd") is None:
                    raise ValueError("会话已失效，且本地没有保存密码")
                status, result = client.api.login_passwd(session["phone"], session["passwd"])
                if not status:
                    raise ValueError(f"自动重登失败: {result}")
                client.api.accinfo()
                _save_session(client.api.session.ck_dump(), client.api.acc, session["passwd"])
            client.classes = None
            return self.get_account_summary(client.api)
        raise ValueError("未找到对应会话")

    def login_with_password(self, client_id: str, phone: str, password: str) -> dict[str, Any]:
        client = self.get_client(client_id)
        status, result = client.api.login_passwd(phone, password)
        if not status:
            raise ValueError(str(result))
        client.api.accinfo()
        _save_session(client.api.session.ck_dump(), client.api.acc, password)
        client.classes = None
        return self.get_account_summary(client.api)

    def start_qr_login(self, client_id: str) -> dict[str, Any]:
        client = self.get_client(client_id)
        client.api.qr_get()
        client.qr_active = True
        return {"qr_url": client.api.qr_geturl()}

    def poll_qr_login(self, client_id: str) -> dict[str, Any]:
        client = self.get_client(client_id)
        if not client.qr_active:
            raise ValueError("二维码登录尚未开始")
        status = client.api.login_qr()
        if status.get("status") is True:
            client.api.accinfo()
            _save_session(client.api.session.ck_dump(), client.api.acc)
            client.qr_active = False
            client.classes = None
            return {"state": "success", "account": self.get_account_summary(client.api)}
        return {"state": "pending", "detail": status}

    def fetch_classes(self, client_id: str) -> list[dict[str, Any]]:
        client = self.get_client(client_id)
        classes = client.api.fetch_classes()
        client.classes = classes
        return [
            {
                "index": index,
                "course_id": item.course_id,
                "class_id": item.class_id,
                "cpi": item.cpi,
                "name": item.name,
                "teacher_name": item.teacher_name,
                "state": item.state.name,
            }
            for index, item in enumerate(classes.classes)
        ]

    def start_task(self, client_id: str, command: str) -> dict[str, Any]:
        client = self.get_client(client_id)
        if not command.strip():
            raise ValueError("课程命令不能为空")
        if not getattr(client.api, "acc", None):
            raise ValueError("请先登录")
        classes = client.classes or client.api.fetch_classes()
        client.classes = classes

        task = WebTask(
            task_id=uuid.uuid4().hex,
            client_id=client_id,
            phone=client.api.acc.phone,
            command=command,
        )
        task.append_log(f"任务已创建: {command}")
        runner = WebTaskRunner(task, client.api)
        thread = threading.Thread(target=runner.run, args=(classes,), daemon=True, name=f"web-task-{task.task_id}")
        with self._lock:
            self._tasks[task.task_id] = task
        thread.start()
        return {"task_id": task.task_id}

    def get_task(self, task_id: str) -> dict[str, Any]:
        task = self._tasks.get(task_id)
        if task is None:
            raise KeyError("task not found")
        return {
            "task_id": task.task_id,
            "state": task.state,
            "output": task.dump_log(),
            "updated_at": task.updated_at,
            "progress": task.progress(),
        }


service = WebAppService()
