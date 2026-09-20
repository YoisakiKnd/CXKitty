/**
 * Request layer.
 *
 * A single thin wrapper around Tauri's `invoke` so that:
 *  - every backend call goes through one place (no scattered `invoke` strings),
 *  - errors are always real `Error` instances carrying the backend message,
 *  - running in a plain browser (no Tauri host) fails with an explicit message
 *    instead of a blank screen or a silently swallowed rejection.
 *
 * Argument names mirror the Rust command signatures (`src-tauri/src/commands.rs`);
 * Tauri maps camelCase JS keys onto snake_case Rust parameters, and nested
 * option structs are passed through verbatim.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** True when a Tauri host is actually present (i.e. `invoke` can work). */
export function isDesktop() {
  return typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;
}

/**
 * Coerce anything thrown by the backend into an `Error` with a readable message.
 * @param {unknown} value
 */
export function toError(value) {
  if (value instanceof Error) return value;
  if (typeof value === 'string') return new Error(value);
  if (value && typeof value === 'object' && 'message' in value) {
    return new Error(String(value.message));
  }
  return new Error(String(value));
}

/**
 * Invoke a backend command.
 * @param {string} command
 * @param {Record<string, unknown>} [args]
 */
export async function call(command, args) {
  if (!isDesktop()) {
    throw new Error('未检测到桌面运行环境：请通过 npm run tauri dev 启动应用。');
  }
  try {
    return await invoke(command, args);
  } catch (e) {
    throw toError(e);
  }
}

/**
 * Subscribe to the brush progress stream. Resolves to an unlisten function.
 * @param {(event: any) => void} handler
 * @returns {Promise<() => void>}
 */
export async function onBrushProgress(handler) {
  if (!isDesktop()) return () => {};
  try {
    const unlisten = await listen('brush-progress', (event) => handler(event.payload || {}));
    return typeof unlisten === 'function' ? unlisten : () => {};
  } catch {
    return () => {};
  }
}

/**
 * Typed-ish facade over the backend commands.
 * @typedef {{ course_id: number, class_id: number, cpi: number, key: number, name: string, teacher_name: string, state: number }} Course
 * @typedef {{ chapter_id: number, name: string, label: string, layer: number, jobs: number, point_total: number, point_finished: number }} Chapter
 */
export const api = {
  /**
   * @param {string} phone
   * @param {string} password
   * @returns {Promise<{ puid: number, name: string, sex: number, phone: string, school: string, stu_id: string | null }>}
   */
  login: (phone, password) => call('login', { phone, password }),

  /** Saved local sessions, masked (`list_saved_sessions`). */
  listSavedSessions: () =>
    call('list_saved_sessions'),

  /**
   * Restore a saved session by phone, re-logging in with the stored password
   * when the cookies have expired.
   * @param {string} phone
   */
  useSavedSession: (phone) => call('use_saved_session', { phone }),

  /** Start QR login: resolves to `{ qrImage, qrUrl }` (serde camelCase). */
  startQrLogin: () => call('start_qr_login'),

  /**
   * Poll QR login once. `state` is `pending` until the phone confirms; on
   * `success` the reply also carries the account summary.
   * @returns {Promise<{ state: 'pending' | 'success', detail: string | null, account: { puid: number, name: string, phone: string, school: string, stu_id: string | null } | null }>}
   */
  pollQrLogin: () => call('poll_qr_login'),

  /** @returns {Promise<Course[]>} */
  listCourses: () => call('list_courses'),

  /** @param {Course} course */
  listHomework: (course) =>
    call('list_homework', {
      courseId: course.course_id,
      classId: course.class_id,
      cpi: course.cpi,
      courseName: course.name,
    }),

  /** @param {Course} course */
  listExams: (course) =>
    call('list_exams', {
      courseId: course.course_id,
      classId: course.class_id,
      cpi: course.cpi,
      courseName: course.name,
    }),

  /** @param {Course} course */
  listChapters: (course) =>
    call('list_chapters', {
      courseId: course.course_id,
      classId: course.class_id,
      cpi: course.cpi,
      key: course.key,
    }),

  /**
   * @param {Course} course
   * @param {Chapter} chapter
   */
  listTaskPoints: (course, chapter) =>
    call('list_task_points', { courseId: course.course_id, chapter }),

  /**
   * Start a brush run over one or more courses, sequentially (mirrors the
   * original comma-joined 序号 command).
   * @param {Course[]} courses
   * @param {{ video_speed: number, video_report_rate: number, unfinished_only: boolean, wait_seconds: number }} options
   */
  startBrush: (courses, options) => call('start_brush', { courses, options }),

  stopBrush: () => call('stop_brush'),

  /** @returns {Promise<boolean>} */
  brushRunning: () => call('brush_running'),
};

/** Brush option defaults, matching `BrushOptions::default()` on the Rust side. */
export const DEFAULT_BRUSH_OPTIONS = {
  video_speed: 1.0,
  video_report_rate: 58,
  unfinished_only: true,
  wait_seconds: 15,
};
