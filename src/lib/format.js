/**
 * Pure mapping / formatting helpers shared by the app shell.
 *
 * All label and tone decisions live here so the components stay declarative and
 * the same value always renders the same way (no per-component drift).
 */

/* ------------------------------------------------------------------ courses */

/**
 * Course status. The Rust side returns `CourseInfo.state` as an integer
 * (`0 = 进行中`, `1 = 已结课`, see `src-tauri/src/api/courses.rs`), while the
 * original Python backend sent the enum *name* (`ClassStatus`). Map the numeric
 * form back to the same user-visible text.
 */
/** @param {unknown} state */
export function courseStateLabel(state) {
  if (state === 0) return '进行中';
  if (state === 1) return '已结课';
  if (state === null || state === undefined || state === '') return '未知';
  return String(state);
}

/**
 * @param {unknown} state
 * @returns {'ok' | 'muted' | 'warn'}
 */
export function courseStateTone(state) {
  if (state === 0) return 'ok';
  if (state === 1) return 'muted';
  return 'warn';
}

/* --------------------------------------------------------------- task state */

/**
 * @param {unknown} state
 * @returns {'IDLE' | 'RUNNING' | 'SUCCESS' | 'FAILED'}
 */
function normalizeTaskState(state) {
  const normalized = String(state ?? '').toUpperCase();
  if (normalized === 'RUNNING' || normalized === 'SUCCESS' || normalized === 'FAILED') {
    return normalized;
  }
  return 'IDLE';
}

/** @param {unknown} state */
export function taskStateLabel(state) {
  switch (normalizeTaskState(state)) {
    case 'RUNNING':
      return '运行中';
    case 'SUCCESS':
      return '已完成';
    case 'FAILED':
      return '失败';
    default:
      return '未启动';
  }
}

/**
 * @param {unknown} state
 * @returns {'info' | 'ok' | 'danger' | 'muted'}
 */
export function taskStateTone(state) {
  switch (normalizeTaskState(state)) {
    case 'RUNNING':
      return 'info';
    case 'SUCCESS':
      return 'ok';
    case 'FAILED':
      return 'danger';
    default:
      return 'muted';
  }
}

/* ------------------------------------------------------------ task points */

/** @param {unknown} status */
export function pointStatusLabel(status) {
  switch (status) {
    case 'done':
      return '已完成';
    case 'failed':
    case 'error':
      return '错误';
    case 'skipped':
      return '跳过';
    case 'running':
      return '进行中';
    default:
      return status ? String(status) : '未知';
  }
}

/**
 * @param {unknown} status
 * @returns {'ok' | 'danger' | 'muted' | 'info' | 'warn'}
 */
export function pointStatusTone(status) {
  switch (status) {
    case 'done':
      return 'ok';
    case 'failed':
    case 'error':
      return 'danger';
    case 'skipped':
      return 'muted';
    case 'running':
      return 'info';
    default:
      return 'warn';
  }
}

/* --------------------------------------------------------------- homework */

/**
 * Homework `status_code` → tone, mirroring the original badge colours.
 * @param {unknown} code
 */
export function homeworkStatusTone(code) {
  switch (code) {
    case 'DONE':
      return 'ok';
    case 'UNDONE':
      return 'danger';
    case 'EXPIRED':
      return 'danger';
    case 'UNDONE_READ':
      return 'muted';
    default:
      return 'warn';
  }
}

/* ------------------------------------------------------- chapter progress */

/**
 * Chapter status derivation, identical to the original backend
 * (`webapp/service.py::sync_chapter_items`):
 * finished → active → partial → pending.
 *
 * @param {{ point_finished: number, point_total: number }} chapter
 * @param {number} index zero-based position of the chapter in the list
 * @param {number} activeIndex zero-based index of the chapter being processed (-1 = none)
 */
export function chapterStatus(chapter, index, activeIndex) {
  const total = Number(chapter?.point_total ?? 0);
  const finished = Number(chapter?.point_finished ?? 0);
  if (total > 0 && finished >= total) return 'done';
  if (index === activeIndex) return 'active';
  if (finished > 0) return 'partial';
  return 'pending';
}

/**
 * @param {unknown} status
 * @returns {'ok' | 'info' | 'warn' | 'muted'}
 */
export function chapterStatusTone(status) {
  switch (status) {
    case 'done':
      return 'ok';
    case 'active':
      return 'info';
    case 'partial':
      return 'warn';
    default:
      return 'muted';
  }
}

/** @param {unknown} status */
export function chapterStatusLabel(status) {
  switch (status) {
    case 'done':
      return '已完成';
    case 'active':
      return '进行中';
    case 'partial':
      return '部分完成';
    default:
      return '未开始';
  }
}

/* ---------------------------------------------------------------- numbers */

/**
 * `a / b` progress text; falls back to `0 / 0` for missing values.
 * @param {unknown} done
 * @param {unknown} total
 */
export function indexText(done, total) {
  const d = Number(done ?? 0);
  const t = Number(total ?? 0);
  return `${Number.isFinite(d) ? d : 0} / ${Number.isFinite(t) ? t : 0}`;
}

/**
 * Clamp a 0–1 ratio, tolerating nullish / non-finite input.
 * @param {unknown} value
 */
export function clampRatio(value) {
  const n = Number(value);
  if (!Number.isFinite(n)) return 0;
  return Math.min(Math.max(n, 0), 1);
}

/**
 * `0.5` → `50%`, used for progress-bar labels.
 * @param {unknown} value
 */
export function ratioPercent(value) {
  return `${Math.round(clampRatio(value) * 100)}%`;
}

/** `HH:MM:SS` timestamp prefix for log lines. */
export function timeStamp(date = new Date()) {
  return date.toLocaleTimeString('zh-CN', { hour12: false });
}
