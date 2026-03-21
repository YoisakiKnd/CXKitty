import logging
import threading
from typing import Callable, Optional

import config

log_file_name = ""
_web_log_ctx = threading.local()


def set_web_log_hook(hook: Optional[Callable[[str], None]]) -> None:
    _web_log_ctx.hook = hook


def get_web_log_hook() -> Optional[Callable[[str], None]]:
    return getattr(_web_log_ctx, "hook", None)

def set_log_filename(phone: str):
    """设置日志文件名
    Args:
        phone: 当前会话的手机号
    """
    global log_file_name
    log_file_name = phone

class Logger:
    """日志记录类
    """
    def __init__(self, name: str, level=logging.DEBUG, fmt=None) -> None:
        """constructor
        Args:
            name: 模块名
            level: 日志级别
            fmt: 日志格式
        """
        self.level = level
        if fmt is None:
            self.fmt = "%(asctime)s [%(name)s] %(levelname)s -> %(message)s"
        else:
            self.fmt = fmt
        
        if not config.LOGS_PATH.is_dir():
            config.LOGS_PATH.mkdir(parents=True)
        self.logger = logging.getLogger(name)
        self.logger.setLevel(self.level)
        
        self.load_handler()
    
    def load_handler(self):
        """重载日志记录器实现
        """
        if log_file_name and (not self.logger.handlers):
            fh = logging.FileHandler(config.LOGS_PATH / f"cxkitty_{log_file_name}.log", encoding="utf8")
            fh.setLevel(self.level)
            fh.setFormatter(logging.Formatter(self.fmt))
            self.logger.addHandler(fh)

    def _emit_web_log(self, level: int, msg, exc_info=False) -> None:
        hook = get_web_log_hook()
        if hook is None:
            return
        record = self.logger.makeRecord(
            self.logger.name,
            level,
            fn="",
            lno=0,
            msg=msg,
            args=(),
            exc_info=None,
        )
        formatted = logging.Formatter(self.fmt).format(record)
        hook(formatted)
        if exc_info:
            hook("附带异常堆栈，请查看本地日志文件获取完整信息。")

    def debug(self, msg) -> None:
        """输出 debug 级别日志
        Args:
            msg: 日志信息
        """
        self.load_handler()
        self.logger.debug(msg)
        self._emit_web_log(logging.DEBUG, msg)

    def info(self, msg) -> None:
        """输出 info 级别日志
        Args:
            msg: 日志信息
        """
        self.load_handler()
        self.logger.info(msg)
        self._emit_web_log(logging.INFO, msg)

    def warning(self, msg) -> None:
        """输出 warning 级别日志
        Args:
            msg: 日志信息
        """
        self.load_handler()
        self.logger.warning(msg)
        self._emit_web_log(logging.WARNING, msg)

    def error(self, msg, exc_info=False) -> None:
        """输出 error 级别日志
        Args:
            msg: 日志信息
            exc_info: 是否输出异常调用栈
        """
        self.load_handler()
        self.logger.error(msg, exc_info=exc_info)
        self._emit_web_log(logging.ERROR, msg, exc_info=exc_info)

__all__ = ["set_log_filename", "set_web_log_hook", "get_web_log_hook", "Logger"]
