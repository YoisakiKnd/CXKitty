/**
 * 刷课 (brush) state machine.
 *
 * Translates the real `brush-progress` event stream (`src-tauri/src/api/brush.rs`)
 * into the same progress model the original page showed: task state, current
 * course/chapter/task, chapter & point indices, per-point status table, chapter
 * status list and the run log.
 *
 * Fields the event stream does not carry are reported as *unavailable* rather
 * than being invented — see `waitProgress` and `capabilities`.
 */
import { api, DEFAULT_BRUSH_OPTIONS, onBrushProgress } from '$lib/api.js';
import { chapterStatus, clampRatio, timeStamp } from '$lib/format.js';

const MAX_LOGS = 500;

/**
 * A single `brush-progress` payload (`BrushProgressEvent` in
 * `src-tauri/src/api/brush.rs`, serialised as camelCase). Both spellings are
 * declared because the backend is the source of truth and the reader below
 * tolerates either.
 *
 * @typedef {object} BrushEvent
 * @property {string} [kind] log | point | chapter | done | error | stopped
 * @property {string} [message]
 * @property {string} [chapter_label]
 * @property {string} [chapterLabel]
 * @property {string} [point_title]
 * @property {string} [pointTitle]
 * @property {string} [point_kind]
 * @property {string} [pointKind]
 * @property {string} [point_status]
 * @property {string} [pointStatus]
 * @property {boolean} [playing]
 * @property {number} [duration]
 * @property {number} [chapter_index]
 * @property {number} [chapterIndex]
 * @property {number} [chapter_total]
 * @property {number} [chapterTotal]
 * @property {number} [point_index]
 * @property {number} [pointIndex]
 * @property {number} [point_total]
 * @property {number} [pointTotal]
 * @property {string} [current_course]
 * @property {string} [currentCourse]
 * @property {number} [course_index]
 * @property {number} [courseIndex]
 * @property {number} [course_total]
 * @property {number} [courseTotal]
 * @property {number} [wait_progress]
 * @property {number} [waitProgress]
 * @property {string} [wait_progress_text]
 * @property {string} [waitProgressText]
 */

/** `point_kind` from the backend → the label the original UI showed. */
const POINT_TYPE_LABELS = {
  video: '视频任务',
  document: '文档任务',
  work: '章节测验',
  exam: '考试',
};

/**
 * @param {string} [kind]
 * @returns {string}
 */
function pointTypeLabel(kind) {
  if (!kind) return '';
  return POINT_TYPE_LABELS[/** @type {keyof typeof POINT_TYPE_LABELS} */ (kind)] ?? String(kind);
}

export function createBrushStore() {
  let running = $state(false);
  let taskState = $state('IDLE'); // IDLE | RUNNING | SUCCESS | FAILED
  let logs = $state(/** @type {string[]} */ ([]));
  let points = $state(/** @type {any[]} */ ([]));

  // Chapters backing the 章节进度 panel. Populated by a real `list_chapters`
  // call before starting, mirroring the original runner which fetched the
  // chapter container itself.
  let chapters = $state(/** @type {any[]} */ ([]));
  let chaptersStatus = $state('idle'); // idle | loading | ready | empty | error
  let chaptersError = $state('');

  let activeChapterIndex = $state(-1);
  let courseName = $state('');
  let currentChapter = $state('');
  let currentTaskType = $state('');
  let currentTask = $state('');
  let chapterIndex = $state(0);
  let chapterTotal = $state(0);
  let pointIndex = $state(0);
  let pointTotal = $state(0);
  let taskProgress = $state(0);
  let taskProgressText = $state('-');
  let waitProgress = $state(0);
  let waitProgressText = $state('-');
  let thisCourseIndex = $state(0);
  let courseTotalCount = $state(0);

  let starting = $state(false);
  let stopping = $state(false);
  let startError = $state('');

  let videoSpeed = $state(DEFAULT_BRUSH_OPTIONS.video_speed);
  let unfinishedOnly = $state(DEFAULT_BRUSH_OPTIONS.unfinished_only);

  /** @type {null | (() => void)} */
  let unlisten = null;
  let listening = false;

  /** @param {unknown} line */
  function pushLog(line) {
    const text = String(line ?? '');
    if (!text) return;
    const entry = `[${timeStamp()}] ${text}`;
    logs = logs.length >= MAX_LOGS ? [...logs.slice(-(MAX_LOGS - 1)), entry] : [...logs, entry];
  }

  /** @param {BrushEvent} event */
  function upsertPoint(event) {
    const title = event.point_title ?? event.pointTitle;
    if (!title) return;
    const kind = event.point_kind ?? event.pointKind ?? '';
    const status = event.point_status ?? event.pointStatus ?? 'running';
    const cIdx = event.chapter_index ?? event.chapterIndex ?? 0;
    const pIdx = event.point_index ?? event.pointIndex ?? 0;
    const row = {
      key: `${cIdx}:${pIdx}:${kind}:${title}`,
      title,
      kind,
      status,
      message: event.message || '',
      chapterIndex: cIdx,
      pointIndex: pIdx,
    };
    const index = points.findIndex((p) => p.key === row.key);
    if (index >= 0) {
      const next = points.slice();
      next[index] = row;
      points = next;
    } else {
      points = [...points, row];
    }
  }

  /**
   * Apply a point completion to the chapter list so 章节进度 reflects real
   * progress without re-fetching (the event stream is the only live source).
   */
  /**
   * @param {number} cIdx 1-based chapter index from the event stream
   * @param {string} status
   */
  function applyPointToChapter(cIdx, status) {
    if (!cIdx || (status !== 'done' && status !== 'skipped')) return;
    const index = cIdx - 1;
    if (index < 0 || index >= chapters.length) return;
    const chapter = chapters[index];
    const total = Number(chapter.point_total ?? 0);
    const finished = Number(chapter.point_finished ?? 0);
    if (total > 0 && finished >= total) return; // already accounted for
    const next = chapters.slice();
    next[index] = { ...chapter, point_finished: finished + 1 };
    chapters = next;
  }

  /** @param {BrushEvent} event */
  function handleEvent(event) {
    const kind = event.kind || 'log';
    const message = event.message || '';
    const cIdx = event.chapter_index ?? event.chapterIndex ?? null;
    const cTotal = event.chapter_total ?? event.chapterTotal ?? null;
    const pIdx = event.point_index ?? event.pointIndex ?? null;
    const pTotal = event.point_total ?? event.pointTotal ?? null;
    const courseIdx = event.course_index ?? event.courseIndex ?? null;
    const courseTotal = event.course_total ?? event.courseTotal ?? null;
    const currentCourse = event.current_course ?? event.currentCourse ?? null;

    if (
      kind === 'log' ||
      kind === 'course' ||
      kind === 'chapter' ||
      kind === 'done' ||
      kind === 'stopped' ||
      kind === 'error'
    ) {
      pushLog(message);
    }

    // 课程切换: reset per-course progress so the panels never show the
    // previous course's numbers against the new one.
    if (kind === 'course') {
      if (currentCourse) courseName = currentCourse;
      if (courseIdx != null) thisCourseIndex = courseIdx;
      if (courseTotal != null) courseTotalCount = courseTotal;
      currentChapter = '';
      currentTaskType = '';
      currentTask = '';
      chapterIndex = 0;
      pointIndex = 0;
      pointTotal = 0;
      taskProgress = 0;
      taskProgressText = '-';
      activeChapterIndex = -1;
      return;
    }

    // 冷却等待: a dedicated countdown stream that drives the amber bar.
    if (kind === 'wait') {
      waitProgress = clampRatio(event.wait_progress ?? event.waitProgress ?? 0);
      waitProgressText =
        (event.wait_progress_text ?? event.waitProgressText) || '-';
      return;
    }
    if (kind === 'point') {
      pushLog(message);
      upsertPoint(event);
    }

    const chapterLabel = event.chapter_label ?? event.chapterLabel;
    if (chapterLabel) currentChapter = chapterLabel;

    const pointTitle = event.point_title ?? event.pointTitle;
    if (pointTitle) currentTask = pointTitle;
    if (event.point_kind || event.pointKind) {
      currentTaskType = pointTypeLabel(event.point_kind ?? event.pointKind);
    }

    if (cIdx != null) {
      chapterIndex = cIdx;
      if (kind === 'chapter' || kind === 'point') activeChapterIndex = cIdx - 1;
    }
    if (cTotal != null) chapterTotal = cTotal;
    if (pIdx != null) pointIndex = pIdx;
    if (pTotal != null) pointTotal = pTotal;

    // 当前任务进度: video playback ratio, else 0 → 1 on completion.
    const playing = event.playing ?? null;
    const duration = event.duration ?? null;
    if (playing != null && duration != null && Number(duration) > 0) {
      taskProgress = Math.min(Number(playing) / Number(duration), 1);
      taskProgressText = `${playing}/${duration}s`;
    } else if (kind === 'point') {
      const status = event.point_status ?? event.pointStatus;
      if (status === 'done') {
        taskProgress = 1;
        taskProgressText = currentTaskType ? `${currentTaskType}完成` : '任务点完成';
      } else if (status === 'skipped') {
        taskProgress = 1;
        taskProgressText = '已跳过';
      } else if (status === 'failed') {
        taskProgress = 0;
        taskProgressText = '失败';
      } else {
        taskProgress = 0;
        taskProgressText = currentTaskType ? `准备处理${currentTaskType}` : '处理中';
      }
      if (cIdx != null) applyPointToChapter(cIdx, status ?? '');
    } else if (kind === 'chapter') {
      taskProgress = 0;
      taskProgressText = '-';
    }

    if (kind === 'done') {
      running = false;
      taskState = 'SUCCESS';
      taskProgress = 0;
      taskProgressText = '-';
      waitProgress = 0;
      waitProgressText = '-';
      activeChapterIndex = -1;
    } else if (kind === 'stopped') {
      running = false;
      taskState = 'IDLE';
      taskProgress = 0;
      taskProgressText = '-';
      waitProgress = 0;
      waitProgressText = '-';
      activeChapterIndex = -1;
    } else if (kind === 'error') {
      running = false;
      taskState = 'FAILED';
      taskProgress = 0;
      taskProgressText = '-';
      waitProgress = 0;
      waitProgressText = '-';
      activeChapterIndex = -1;
    }
  }

  async function subscribe() {
    if (listening) return;
    listening = true;
    unlisten = await onBrushProgress(handleEvent);
  }

  /** Restore state on mount (the backend may already be running a task). */
  async function sync() {
    await subscribe();
    try {
      running = await api.brushRunning();
      if (running) {
        taskState = 'RUNNING';
      }
    } catch {
      // No backend (browser preview) — stay idle.
    }
  }

  function teardown() {
    if (unlisten) unlisten();
    unlisten = null;
    listening = false;
  }

  /**
   * Load the chapter list for the 章节进度 panel.
   * @param {any} course
   */
  async function loadChapters(course) {
    if (!course) return;
    chaptersStatus = 'loading';
    chaptersError = '';
    try {
      const result = await api.listChapters(course);
      chapters = Array.isArray(result) ? result : [];
      chaptersStatus = chapters.length === 0 ? 'empty' : 'ready';
    } catch (e) {
      chapters = [];
      chaptersError = e instanceof Error ? e.message : String(e);
      chaptersStatus = 'error';
    }
  }

  /**
   * Start a brush task over one or more courses (run sequentially by the
   * backend, matching the original comma-joined 序号 command).
   *
   * The chapter list of the first course is fetched up front so the progress
   * panel has real data to render from the moment the task starts.
   *
   * @param {any[]} courses
   */
  async function start(courses) {
    if (starting || running) return false;
    const targets = Array.isArray(courses) ? courses.filter(Boolean) : [];
    if (targets.length === 0) {
      startError = '请先勾选课程';
      return false;
    }
    const speed = Number(videoSpeed);
    if (!Number.isFinite(speed) || speed <= 0) {
      startError = '视频倍速需为大于 0 的数字';
      return false;
    }

    startError = '';
    starting = true;
    await subscribe();

    logs = [];
    points = [];
    courseName = targets[0].name ?? '';
    currentChapter = '';
    currentTaskType = '';
    currentTask = '';
    chapterIndex = 0;
    pointIndex = 0;
    pointTotal = 0;
    taskProgress = 0;
    taskProgressText = '-';
    waitProgress = 0;
    waitProgressText = '-';
    activeChapterIndex = -1;
    thisCourseIndex = 1;
    courseTotalCount = targets.length;

    // Fetch chapters first so the panel shows the real structure immediately.
    await loadChapters(targets[0]);
    chapterTotal = chapters.length;

    pushLog(
      targets.length > 1
        ? `准备刷课：共 ${targets.length} 门课程（${targets.map((c) => c.name).join('、')}）`
        : `准备刷课：${targets[0].name}`,
    );

    try {
      const result = await api.startBrush(targets, {
        video_speed: speed,
        video_report_rate: DEFAULT_BRUSH_OPTIONS.video_report_rate,
        unfinished_only: unfinishedOnly,
        wait_seconds: DEFAULT_BRUSH_OPTIONS.wait_seconds,
      });
      running = true;
      taskState = 'RUNNING';
      pushLog(result?.message || '刷课已开始');
      return true;
    } catch (e) {
      const message = e instanceof Error ? e.message : String(e);
      running = false;
      taskState = 'FAILED';
      startError = message;
      pushLog(`启动失败：${message}`);
      return false;
    } finally {
      starting = false;
    }
  }

  async function stop() {
    if (stopping) return;
    stopping = true;
    try {
      const result = await api.stopBrush();
      pushLog(result?.message || '已请求停止');
    } catch (e) {
      pushLog(`停止失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      stopping = false;
    }
  }

  /** Clear the run artefacts (logs, point table) without touching the backend. */
  function clear() {
    logs = [];
    points = [];
    currentChapter = '';
    currentTaskType = '';
    currentTask = '';
    chapterIndex = 0;
    pointIndex = 0;
    pointTotal = 0;
    taskProgress = 0;
    taskProgressText = '-';
  }

  /** Drop everything, e.g. after sign-out. */
  function reset() {
    running = false;
    taskState = 'IDLE';
    chapters = [];
    chaptersStatus = 'idle';
    chaptersError = '';
    courseName = '';
    startError = '';
    clear();
  }

  /**
   * Course counters for the 任务进度 panel.
   *
   * `start_brush` now takes the whole checked-course list and runs it
   * sequentially, so the totals come from the backend's own `course` events
   * (`course_index` / `course_total`) rather than being assumed.
   */
  const totalCourses = $derived(courseTotalCount || (courseName ? 1 : 0));
  const completedCourses = $derived(
    courseTotalCount > 0
      ? taskState === 'SUCCESS'
        ? courseTotalCount
        : Math.max(thisCourseIndex - 1, 0)
      : courseName && taskState === 'SUCCESS'
        ? 1
        : 0,
  );

  /**
   * Chapter rows with the derived status (done / active / partial / pending).
   * The status rule itself lives in `format.js::chapterStatus` so the list and
   * any other consumer can never drift apart.
   */
  const chapterItems = $derived(
    chapters.map((chapter, index) => {
      const total = Number(chapter.point_total ?? 0);
      const finished = Number(chapter.point_finished ?? 0);
      return {
        key: chapter.chapter_id,
        label: chapter.label,
        name: chapter.name,
        layer: chapter.layer ?? 0,
        pointFinished: finished,
        pointTotal: total,
        progressText: `${finished}/${total}`,
        status: chapterStatus(chapter, index, activeChapterIndex),
      };
    }),
  );

  return {
    get running() {
      return running;
    },
    get taskState() {
      return taskState;
    },
    get logs() {
      return logs;
    },
    get points() {
      return points;
    },
    get chapters() {
      return chapters;
    },
    get chaptersStatus() {
      return chaptersStatus;
    },
    get chaptersError() {
      return chaptersError;
    },
    get chapterItems() {
      return chapterItems;
    },
    get courseName() {
      return courseName;
    },
    get currentChapter() {
      return currentChapter;
    },
    get currentTaskType() {
      return currentTaskType;
    },
    get currentTask() {
      return currentTask;
    },
    get chapterIndex() {
      return chapterIndex;
    },
    get chapterTotal() {
      return chapterTotal;
    },
    get pointIndex() {
      return pointIndex;
    },
    get pointTotal() {
      return pointTotal;
    },
    get totalCourses() {
      return totalCourses;
    },
    get completedCourses() {
      return completedCourses;
    },
    get taskProgress() {
      return taskProgress;
    },
    get taskProgressText() {
      return taskProgressText;
    },
    get waitProgress() {
      return waitProgress;
    },
    get waitProgressText() {
      return waitProgressText;
    },
    get courseIndex() {
      return thisCourseIndex;
    },
    get courseTotalCount() {
      return courseTotalCount;
    },
    get starting() {
      return starting;
    },
    get stopping() {
      return stopping;
    },
    get startError() {
      return startError;
    },
    get videoSpeed() {
      return videoSpeed;
    },
    get unfinishedOnly() {
      return unfinishedOnly;
    },
    /** Whether the run log has anything to show yet. */
    get hasLogs() {
      return logs.length > 0;
    },
    /** @param {number} value */
    setVideoSpeed(value) {
      videoSpeed = value;
      if (startError) startError = '';
    },
    /** @param {boolean} value */
    setUnfinishedOnly(value) {
      unfinishedOnly = value;
    },
    sync,
    teardown,
    loadChapters,
    start,
    stop,
    clear,
    reset,
  };
}
