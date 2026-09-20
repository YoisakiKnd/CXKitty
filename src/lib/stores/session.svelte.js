/**
 * Login + account state.
 *
 * Owns the credential form, the login lifecycle (validation → in-flight guard →
 * error surfacing), the account summary, the list of locally saved sessions and
 * the QR login flow.
 *
 * All three login routes are backed by real commands in `src-tauri/src/commands.rs`
 * (`login`, `use_saved_session`, `start_qr_login` / `poll_qr_login`); nothing here
 * is faked, and a missing Tauri host surfaces an explicit error instead of a
 * silently dead control.
 */
import { api, isDesktop } from '$lib/api.js';

/** Poll interval for QR login, matching the original page's 2s timer. */
const QR_POLL_MS = 2000;

/**
 * @param {{ onQrSuccess?: () => void | Promise<void> }} [options]
 */
export function createSessionStore(options = {}) {
  const onQrSuccess = options.onQrSuccess;
  let phone = $state('');
  let password = $state('');
  let showPassword = $state(false);
  let submitting = $state(false);
  let account = $state(/** @type {null | { puid: number, name: string, phone: string, school: string, stu_id: string | null }} */ (null));
  let fieldErrors = $state(/** @type {{ phone?: string, password?: string }} */ ({}));
  let formError = $state('');

  // ---------------------------------------------------------- saved sessions
  let savedSessions = $state(/** @type {any[]} */ ([]));
  let sessionsLoading = $state(false);
  let sessionsError = $state('');
  let selectedSession = $state('');
  let restoring = $state(false);

  // --------------------------------------------------------------- QR login
  let qrImage = $state('');
  let qrOpen = $state(false);
  let qrStarting = $state(false);
  let qrError = $state('');
  /** @type {ReturnType<typeof setInterval> | null} */
  let qrTimer = null;

  /** @param {string} value */
  function setPhone(value) {
    phone = value;
    if (fieldErrors.phone) fieldErrors = { ...fieldErrors, phone: undefined };
    if (formError) formError = '';
  }

  /** @param {string} value */
  function setPassword(value) {
    password = value;
    if (fieldErrors.password) fieldErrors = { ...fieldErrors, password: undefined };
    if (formError) formError = '';
  }

  function togglePassword() {
    showPassword = !showPassword;
  }

  /**
   * Required + format validation. Mirrors the backend expectation (学习通 uses
   * a phone number as the account) without loosening it: empty input is
   * rejected locally instead of being sent to the server.
   */
  function validate() {
    /** @type {{ phone?: string, password?: string }} */
    const errors = {};
    const trimmed = phone.trim();
    if (!trimmed) {
      errors.phone = '请输入手机号';
    } else if (!/^\d{6,20}$/.test(trimmed)) {
      errors.phone = '手机号应为 6–20 位数字';
    }
    if (!password) {
      errors.password = '请输入密码';
    } else if (password.length < 4) {
      errors.password = '密码至少 4 位';
    }
    fieldErrors = errors;
    return Object.keys(errors).length === 0;
  }

  /**
   * Perform the login. Resolves to `true` on success so the caller can chain
   * follow-up loads (fetching courses) without duplicating the guard.
   */
  async function login() {
    if (submitting) return false;
    formError = '';
    if (!validate()) return false;
    if (!isDesktop()) {
      formError = '未检测到桌面运行环境：请通过 npm run tauri dev 启动应用。';
      return false;
    }
    submitting = true;
    try {
      const result = await api.login(phone.trim(), password);
      account = result;
      password = '';
      await loadSavedSessions();
      return true;
    } catch (e) {
      account = null;
      formError = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      submitting = false;
    }
  }

  /** Load the locally saved sessions and preselect the first one. */
  async function loadSavedSessions() {
    if (sessionsLoading) return;
    sessionsLoading = true;
    sessionsError = '';
    try {
      const rows = await api.listSavedSessions();
      savedSessions = Array.isArray(rows) ? rows : [];
      if (savedSessions.length && !savedSessions.some((r) => r.phone === selectedSession)) {
        selectedSession = savedSessions[0].phone;
      }
    } catch (e) {
      savedSessions = [];
      sessionsError = e instanceof Error ? e.message : String(e);
    } finally {
      sessionsLoading = false;
    }
  }

  /**
   * Restore the selected saved session (免密登录). Resolves to `true` when the
   * caller should refresh dependent data.
   */
  async function useSavedSession() {
    if (restoring) return false;
    sessionsError = '';
    if (!selectedSession) {
      sessionsError = '请先选择本地会话';
      return false;
    }
    if (!isDesktop()) {
      sessionsError = '未检测到桌面运行环境：请通过 npm run tauri dev 启动应用。';
      return false;
    }
    restoring = true;
    try {
      account = await api.useSavedSession(selectedSession);
      return true;
    } catch (e) {
      sessionsError = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      restoring = false;
    }
  }

  /** @param {string} value */
  function setSelectedSession(value) {
    selectedSession = value;
    if (sessionsError) sessionsError = '';
  }

  function stopQr() {
    if (qrTimer !== null) {
      clearInterval(qrTimer);
      qrTimer = null;
    }
  }

  /** Fetch a fresh QR code and begin polling for confirmation. */
  async function startQr() {
    if (qrStarting) return;
    qrError = '';
    if (!isDesktop()) {
      qrError = '未检测到桌面运行环境：请通过 npm run tauri dev 启动应用。';
      return;
    }
    qrStarting = true;
    try {
      const result = await api.startQrLogin();
      // `QrStartResult` is serialised camelCase (`qrImage` / `qrUrl`).
      qrImage = result?.qrImage || '';
      if (!qrImage) throw new Error('未获取到二维码图片');
      qrOpen = true;
      stopQr();
      qrTimer = setInterval(pollQr, QR_POLL_MS);
    } catch (e) {
      qrError = e instanceof Error ? e.message : String(e);
    } finally {
      qrStarting = false;
    }
  }

  /**
   * One poll. On success the dialog closes and the account is adopted; a
   * `pending` reply is the normal "not scanned yet" case and stays silent,
   * mirroring the original which swallowed poll errors.
   */
  async function pollQr() {
    try {
      const result = await api.pollQrLogin();
      if (result?.state === 'success') {
        stopQr();
        qrOpen = false;
        qrImage = '';
        if (result.account) account = result.account;
        // The backend persisted this account, so refresh the local session list.
        await loadSavedSessions();
        await onQrSuccess?.();
      }
    } catch {
      // Transient poll failures are ignored, same as the original.
    }
  }

  function closeQr() {
    stopQr();
    qrOpen = false;
  }

  /** Tear down the poll timer (component teardown). */
  function teardown() {
    stopQr();
  }

  return {
    get phone() {
      return phone;
    },
    get password() {
      return password;
    },
    get showPassword() {
      return showPassword;
    },
    get submitting() {
      return submitting;
    },
    get account() {
      return account;
    },
    get fieldErrors() {
      return fieldErrors;
    },
    get formError() {
      return formError;
    },
    get loggedIn() {
      return account != null;
    },
    get savedSessions() {
      return savedSessions;
    },
    get sessionsLoading() {
      return sessionsLoading;
    },
    get sessionsError() {
      return sessionsError;
    },
    get selectedSession() {
      return selectedSession;
    },
    get restoring() {
      return restoring;
    },
    get qrImage() {
      return qrImage;
    },
    get qrOpen() {
      return qrOpen;
    },
    get qrStarting() {
      return qrStarting;
    },
    get qrError() {
      return qrError;
    },
    setPhone,
    setPassword,
    togglePassword,
    login,
    loadSavedSessions,
    useSavedSession,
    setSelectedSession,
    startQr,
    closeQr,
    teardown,
  };
}
