<script>
  /**
   * 账号 panel — the three login routes the original page offered, plus the
   * account summary: 密码登录, 本地会话（列表 / 载入 / 刷新）and 扫码登录.
   * All three are backed by real commands (see `src-tauri/src/commands.rs`).
   */
  import { onMount } from 'svelte';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import Panel from '../app/Panel.svelte';
  import QrLoginDialog from './QrLoginDialog.svelte';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const session = app.session;

  // The original page loaded the saved-session list on startup.
  onMount(() => {
    session.loadSavedSessions();
  });

  /** @param {Event} event */
  function onKeydown(event) {
    if (event.key === 'Enter' && !session.submitting) app.login();
  }
</script>

<Panel
  title="账号"
  description="使用学习通手机号与密码登录，登录态保存在后端 Cookie 会话中。"
>
  <form class="space-y-4" onsubmit={(e) => e.preventDefault()} novalidate>
    <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-[1fr_1fr_auto] lg:items-start">
      <div class="space-y-1.5">
        <Label for="account-phone">手机号 / 学号</Label>
        <Input
          id="account-phone"
          value={session.phone}
          oninput={(e) => session.setPhone(e.currentTarget.value)}
          autocomplete="username"
          inputmode="numeric"
          disabled={session.submitting}
          aria-invalid={session.fieldErrors.phone ? 'true' : undefined}
          aria-describedby={session.fieldErrors.phone ? 'account-phone-error' : undefined}
          placeholder="请输入手机号"
          onkeydown={onKeydown}
        />
        {#if session.fieldErrors.phone}
          <p id="account-phone-error" class="text-destructive text-xs" role="alert">
            {session.fieldErrors.phone}
          </p>
        {/if}
      </div>

      <div class="space-y-1.5">
        <Label for="account-password">密码</Label>
        <div class="relative">
          <Input
            id="account-password"
            type={session.showPassword ? 'text' : 'password'}
            value={session.password}
            oninput={(e) => session.setPassword(e.currentTarget.value)}
            autocomplete="current-password"
            disabled={session.submitting}
            aria-invalid={session.fieldErrors.password ? 'true' : undefined}
            aria-describedby={session.fieldErrors.password ? 'account-password-error' : undefined}
            placeholder="请输入密码"
            class="pr-14"
            onkeydown={onKeydown}
          />
          <button
            type="button"
            class="text-muted-foreground hover:text-foreground focus-visible:ring-ring/50 absolute inset-y-0 right-0 flex items-center rounded-md px-2 text-[0.6875rem] focus-visible:ring-2 focus-visible:outline-none disabled:opacity-50"
            onclick={() => session.togglePassword()}
            disabled={session.submitting}
            aria-pressed={session.showPassword}
            aria-label={session.showPassword ? '隐藏密码' : '显示密码'}
          >
            {session.showPassword ? '隐藏' : '显示'}
          </button>
        </div>
        {#if session.fieldErrors.password}
          <p id="account-password-error" class="text-destructive text-xs" role="alert">
            {session.fieldErrors.password}
          </p>
        {/if}
      </div>

      <div class="lg:pt-5">
        <Button
          type="submit"
          onclick={() => app.login()}
          disabled={session.submitting}
          class="w-full lg:w-auto"
        >
          {session.submitting ? '登录中…' : '密码登录'}
        </Button>
      </div>
    </div>

    {#if session.formError}
      <p class="text-destructive text-xs" role="alert">{session.formError}</p>
    {/if}

    <Separator />

    <div class="space-y-2">
      <div class="grid gap-3 sm:grid-cols-2 lg:grid-cols-[1fr_auto_auto] lg:items-start">
        <div class="space-y-1.5">
          <Label for="saved-session">本地会话</Label>
          <select
            id="saved-session"
            class="border-input bg-input/20 focus-visible:border-ring focus-visible:ring-ring/30 h-7 w-full rounded-md border px-2 text-xs outline-none focus-visible:ring-2 disabled:pointer-events-none disabled:opacity-50"
            value={session.selectedSession}
            onchange={(e) => session.setSelectedSession(e.currentTarget.value)}
            disabled={session.sessionsLoading || session.restoring || session.savedSessions.length === 0}
          >
            {#if session.savedSessions.length === 0}
              <option value="">（无本地会话）</option>
            {:else}
              {#each session.savedSessions as row (row.phone)}
                <option value={row.phone}>
                  {row.maskedPhone} | {row.maskedName} | puid={row.puid}
                </option>
              {/each}
            {/if}
          </select>
        </div>

        <div class="lg:pt-5">
          <Button
            variant="outline"
            onclick={() => app.useSavedSession()}
            disabled={session.restoring || !session.selectedSession || session.sessionsLoading}
          >
            {session.restoring ? '载入中…' : '载入会话'}
          </Button>
        </div>

        <div class="lg:pt-5">
          <Button
            variant="outline"
            onclick={() => session.loadSavedSessions()}
            disabled={session.sessionsLoading || session.restoring}
          >
            {session.sessionsLoading ? '刷新中…' : '刷新本地会话'}
          </Button>
        </div>
      </div>

      {#if session.sessionsError}
        <p class="text-destructive text-xs" role="alert">{session.sessionsError}</p>
      {/if}

      <div class="flex flex-wrap items-center gap-2 pt-1">
        <Button
          variant="outline"
          size="sm"
          onclick={() => session.startQr()}
          disabled={session.qrStarting || session.submitting}
        >
          {session.qrStarting ? '获取二维码中…' : '扫码登录'}
        </Button>
        {#if session.qrError}
          <span class="text-destructive text-xs" role="alert">{session.qrError}</span>
        {/if}
      </div>
    </div>

    {#if session.account}
      <div
        class="bg-muted/40 grid gap-x-4 gap-y-1 rounded-md border px-3 py-2 text-xs sm:grid-cols-2 lg:grid-cols-4"
      >
        <p><span class="text-muted-foreground">姓名：</span>{session.account.name || '—'}</p>
        <p><span class="text-muted-foreground">手机号：</span>{session.account.phone || '—'}</p>
        <p><span class="text-muted-foreground">学校：</span>{session.account.school || '未知学校'}</p>
        <p><span class="text-muted-foreground">puid：</span>{session.account.puid}</p>
      </div>
    {/if}
  </form>

  <QrLoginDialog />
</Panel>
