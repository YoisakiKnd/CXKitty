/**
 * Application store.
 *
 * Composes the domain stores (session / courses / brush) with the two
 * course-scoped list resources (作业 / 考试) and the chapter-scoped 任务点
 * preview, and owns the single app-wide status line.
 *
 * Provided to the component tree through context so panels stay presentational.
 */
import { getContext, setContext } from 'svelte';
import { api } from '$lib/api.js';
import { createResource } from './resource.svelte.js';
import { createSessionStore } from './session.svelte.js';
import { createCoursesStore } from './courses.svelte.js';
import { createBrushStore } from './brush.svelte.js';

const APP_KEY = Symbol('cxkitty-app');

export function createAppStore() {
  // The session store needs to refresh courses once a QR login lands, but
  // `refreshCourses` is defined below — pass a lazy hook so the dependency
  // stays one-directional at construction time.
  const session = createSessionStore({
    onQrSuccess: () => onQrLoginSuccess(),
  });
  const courses = createCoursesStore();
  const brush = createBrushStore();

  let status = $state('');
  let statusKind = $state(''); // '' | 'ok' | 'error'
  let selectedChapterId = $state(/** @type {number | null} */ (null));

  /** @param {string} message @param {'' | 'ok' | 'error'} [kind] */
  function setStatus(message, kind = '') {
    status = message;
    statusKind = kind;
  }

  function courseName() {
    return courses.activeCourse?.name ?? '';
  }

  /* ------------------------------------------------------------ 作业 list */
  const homework = createResource(
    async () => {
      const course = courses.activeCourse;
      if (!course) throw new Error('请先选择课程');
      setStatus(`拉取作业：${course.name}…`);
      const rows = await api.listHomework(course);
      const list = Array.isArray(rows) ? rows : [];
      setStatus(`作业 ${list.length} 项`, 'ok');
      return list;
    },
    { isEmpty: (rows) => !Array.isArray(rows) || rows.length === 0, initial: [] },
  );

  /* ------------------------------------------------------------ 考试 list */
  const exams = createResource(
    async () => {
      const course = courses.activeCourse;
      if (!course) throw new Error('请先选择课程');
      setStatus(`拉取考试：${course.name}…`);
      const rows = await api.listExams(course);
      const list = Array.isArray(rows) ? rows : [];
      setStatus(`考试 ${list.length} 项`, 'ok');
      return list;
    },
    { isEmpty: (rows) => !Array.isArray(rows) || rows.length === 0, initial: [] },
  );

  /* ------------------------------------------------------- 任务点 preview */
  const taskPoints = createResource(
    async () => {
      const course = courses.activeCourse;
      const chapter = brush.chapters.find((c) => c.chapter_id === selectedChapterId);
      if (!course || !chapter) throw new Error('请先选择章节');
      setStatus(`解析任务点：${chapter.label} ${chapter.name}…`);
      const rows = await api.listTaskPoints(course, chapter);
      const list = Array.isArray(rows) ? rows : [];
      setStatus(`任务点 ${list.length} 个`, 'ok');
      return list;
    },
    { isEmpty: (rows) => !Array.isArray(rows) || rows.length === 0, initial: [] },
  );

  /** Drop everything course-scoped; used when the account changes. */
  function clearCourseScoped() {
    homework.reset();
    exams.reset();
    taskPoints.reset();
    selectedChapterId = null;
  }

  /** Log in, then load courses (the original flow chained these two). */
  async function login() {
    const ok = await session.login();
    if (ok) {
      setStatus(`登录成功：${session.account?.name ?? ''}`, 'ok');
      clearCourseScoped();
      brush.reset();
      const list = await courses.load();
      if (courses.status === 'ready') {
        setStatus(`课程 ${list.length} 门`, 'ok');
        // Prime the 章节进度 panel with real data for the active course.
        await brush.loadChapters(courses.activeCourse);
      } else if (courses.status === 'empty') {
        setStatus('未获取到课程', 'error');
      } else {
        setStatus(courses.error, 'error');
      }
    } else if (session.formError) {
      courses.reset();
      clearCourseScoped();
      brush.reset();
      setStatus(session.formError, 'error');
    }
    return ok;
  }

  async function refreshCourses() {
    clearCourseScoped();
    const list = await courses.load();
    if (courses.status === 'ready') {
      setStatus(`课程 ${list.length} 门`, 'ok');
      await brush.loadChapters(courses.activeCourse);
    } else if (courses.status === 'empty') {
      setStatus('未获取到课程', 'error');
    } else {
      setStatus(courses.error, 'error');
    }
  }

  /**
   * Switch the active course.
   * Clears the course-scoped lists so stale rows from the previous course are
   * never shown against the new one, then primes the chapter panel.
   * @param {number} courseId
   */
  async function selectCourse(courseId) {
    if (courses.activeCourseId === courseId) return;
    courses.setActive(courseId);
    clearCourseScoped();
    await brush.loadChapters(courses.activeCourse);
  }

  /** @param {number} chapterId */
  function selectChapter(chapterId) {
    selectedChapterId = chapterId;
    taskPoints.reset();
  }

  /**
   * 开始刷课 — runs every checked course in order (the original's command was
   * the comma-joined 序号 list). Falls back to nothing when none are checked,
   * exactly like the original's `请先勾选课程` guard.
   */
  async function startTask() {
    const targets = courses.selectedCourses;
    if (targets.length === 0) {
      setStatus('请先勾选课程', 'error');
      return false;
    }
    const ok = await brush.start(targets);
    if (ok) {
      setStatus(`刷课已开始（${targets.length} 门课程）`, 'ok');
    } else if (brush.startError) {
      setStatus(brush.startError, 'error');
    }
    return ok;
  }

  async function stopTask() {
    await brush.stop();
    setStatus('已请求停止');
  }

  /** Restore the selected saved session, then load its courses. */
  async function useSavedSession() {
    const ok = await session.useSavedSession();
    if (ok) {
      setStatus(`本地会话已载入：${session.account?.name ?? ''}`, 'ok');
      clearCourseScoped();
      brush.reset();
      await refreshCourses();
    } else if (session.sessionsError) {
      setStatus(session.sessionsError, 'error');
    }
    return ok;
  }

  /** Called by the session store once a QR login is confirmed. */
  async function onQrLoginSuccess() {
    setStatus('二维码登录成功', 'ok');
    clearCourseScoped();
    brush.reset();
    await refreshCourses();
  }

  const app = {
    session,
    courses,
    brush,
    homework,
    exams,
    taskPoints,
    get status() {
      return status;
    },
    get statusKind() {
      return statusKind;
    },
    get selectedChapterId() {
      return selectedChapterId;
    },
    get courseName() {
      return courseName();
    },
    /** Whether the 任务控制 bar should be interactive. */
    get canStartTask() {
      return courses.selectedCount > 0 && !brush.running && !brush.starting && !courses.loading;
    },
    setStatus,
    clearCourseScoped,
    login,
    refreshCourses,
    selectCourse,
    selectChapter,
    startTask,
    stopTask,
    useSavedSession,
    onQrLoginSuccess,
  };

  setContext(APP_KEY, app);
  return app;
}

/** Read the app store from context. */
export function getApp() {
  return getContext(APP_KEY);
}
