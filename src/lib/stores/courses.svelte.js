/**
 * Course list state: loading, multi-selection, pagination.
 *
 * Selection and pagination deliberately live outside the component so that the
 * "当前勾选" hint, the task control bar and the course table all read the same
 * source of truth — matching the original single-page behaviour where the table
 * was the only owner of the selection.
 */
import { api } from '$lib/api.js';

export const COURSE_PAGE_SIZE = 12;

export function createCoursesStore() {
  let courses = $state(/** @type {any[]} */ ([]));
  let status = $state('idle'); // idle | loading | ready | empty | error
  let error = $state('');
  let selected = $state(/** @type {Set<number>} */ (new Set()));
  let page = $state(1);
  let activeCourseId = $state(/** @type {number | null} */ (null));

  const totalPages = () => Math.max(1, Math.ceil(courses.length / COURSE_PAGE_SIZE));

  const pageCourses = () =>
    courses.slice((page - 1) * COURSE_PAGE_SIZE, page * COURSE_PAGE_SIZE);

  /** Rows for the table, carrying the 1-based 序号 used by the original UI. */
  const pageRows = () =>
    pageCourses().map((course, offset) => ({
      ...course,
      index: (page - 1) * COURSE_PAGE_SIZE + offset + 1,
    }));

  /**
   * Sorted, comma-joined 序号 string — identical to the original
   * `selected_command()` so the value is directly comparable with the baseline.
   */
  function selectedCommand() {
    const indexes = courses
      .map((course, offset) => ({ course, index: offset + 1 }))
      .filter((row) => selected.has(row.course.course_id))
      .map((row) => row.index)
      .sort((a, b) => a - b);
    return indexes.join(',');
  }

  /** @param {number} courseId @param {boolean} checked */
  function toggle(courseId, checked) {
    const next = new Set(selected);
    if (checked) next.add(courseId);
    else next.delete(courseId);
    selected = next;
  }

  /** @param {boolean} checked */
  function setAllOnPage(checked) {
    const next = new Set(selected);
    for (const row of pageRows()) {
      if (checked) next.add(row.course_id);
      else next.delete(row.course_id);
    }
    selected = next;
  }

  function clearSelection() {
    selected = new Set();
  }

  /** @param {number} value */
  function goToPage(value) {
    page = Math.min(Math.max(1, value), totalPages());
  }

  function nextPage() {
    goToPage(page + 1);
  }

  function prevPage() {
    goToPage(page - 1);
  }

  async function load() {
    status = 'loading';
    error = '';
    try {
      const result = await api.listCourses();
      courses = Array.isArray(result) ? result : [];
      selected = new Set();
      page = 1;
      activeCourseId = courses[0]?.course_id ?? null;
      status = courses.length === 0 ? 'empty' : 'ready';
      return courses;
    } catch (e) {
      courses = [];
      selected = new Set();
      page = 1;
      activeCourseId = null;
      error = e instanceof Error ? e.message : String(e);
      status = 'error';
      return [];
    }
  }

  /** Used when the session is dropped, so stale courses are not shown. */
  function reset() {
    courses = [];
    selected = new Set();
    page = 1;
    activeCourseId = null;
    status = 'idle';
    error = '';
  }

  return {
    get courses() {
      return courses;
    },
    get status() {
      return status;
    },
    get error() {
      return error;
    },
    get loading() {
      return status === 'loading';
    },
    get page() {
      return page;
    },
    get pageSize() {
      return COURSE_PAGE_SIZE;
    },
    get totalPages() {
      return totalPages();
    },
    get pageCourses() {
      return pageCourses();
    },
    get pageRows() {
      return pageRows();
    },
    get selectedIds() {
      return selected;
    },
    get selectedCount() {
      return selected.size;
    },
    /**
     * Every checked course, in list order — the brush run covers all of them
     * sequentially, matching the original comma-joined 序号 command.
     */
    get selectedCourses() {
      return courses.filter((c) => selected.has(c.course_id));
    },
    get activeCourseId() {
      return activeCourseId;
    },
    /** First selected course (single-course commands use this), else the active row. */
    get primaryCourse() {
      if (selected.size > 0) {
        return courses.find((c) => selected.has(c.course_id)) ?? null;
      }
      return courses.find((c) => c.course_id === activeCourseId) ?? null;
    },
    /** The course used by 作业/考试/刷课 panels: the active row. */
    get activeCourse() {
      return courses.find((c) => c.course_id === activeCourseId) ?? null;
    },
    selectedCommand,
    toggle,
    setAllOnPage,
    clearSelection,
    goToPage,
    nextPage,
    prevPage,
    /** @param {number | null} id */
    setActive(id) {
      activeCourseId = id;
    },
    load,
    retry: load,
    reset,
  };
}
