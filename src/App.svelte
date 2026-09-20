<script>
  /**
   * Application shell.
   *
   * Region order follows the original single-page layout (账号 → 课程 → 功能页 →
   * 状态). The 作业 / 考试 / 刷课 tab group mirrors the pre-refactor page, so the
   * 刷课 页 documented in README stays a real navigable entry point; the
   * baseline's 章节进度 / 任务进度 / 运行日志 regions live inside it rather than
   * being dropped.
   *
   * All state lives in the app store (`$lib/stores/app.svelte.js`); this file is
   * layout plus lifecycle only.
   */
  import { onDestroy, onMount } from 'svelte';

  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import Panel from '$lib/components/app/Panel.svelte';
  import AccountPanel from '$lib/components/panels/AccountPanel.svelte';
  import CourseTablePanel from '$lib/components/panels/CourseTablePanel.svelte';
  import TaskControlPanel from '$lib/components/panels/TaskControlPanel.svelte';
  import ChapterProgressPanel from '$lib/components/panels/ChapterProgressPanel.svelte';
  import TaskProgressPanel from '$lib/components/panels/TaskProgressPanel.svelte';
  import RunLogPanel from '$lib/components/panels/RunLogPanel.svelte';
  import HomeworkPanel from '$lib/components/panels/HomeworkPanel.svelte';
  import ExamPanel from '$lib/components/panels/ExamPanel.svelte';
  import { createAppStore } from '$lib/stores/app.svelte.js';
  import { cn } from '$lib/utils.js';

  const app = createAppStore();

  let tab = $state('homework'); // homework | exam | brush

  onMount(() => {
    // The backend may already be running a task (e.g. the window was reloaded),
    // so reconcile state with the real `brush_running` command on mount.
    app.brush.sync();
  });

  onDestroy(() => {
    app.brush.teardown();
    // Stop the QR poll timer so it cannot outlive the component tree.
    app.session.teardown();
  });
</script>

<main class="space-y-4">
  <header class="space-y-1">
    <h1 class="text-xl font-semibold tracking-tight">学习通工具箱</h1>
    <p class="text-muted-foreground text-xs">
      单账号会话 · 作业 / 考试 / 刷课 · Rust + Svelte + Tauri 2 · GPL-3.0
    </p>
  </header>

  <AccountPanel />
  <CourseTablePanel />

  <Panel title="作业 / 考试 / 刷课" description="按「课程」中高亮的课程拉取列表或执行刷课。">
    <Tabs.Root bind:value={tab}>
      <Tabs.List>
        <Tabs.Trigger value="homework">作业</Tabs.Trigger>
        <Tabs.Trigger value="exam">考试</Tabs.Trigger>
        <Tabs.Trigger value="brush">刷课</Tabs.Trigger>
      </Tabs.List>
      <Tabs.Content value="homework" class="pt-3">
        <HomeworkPanel />
      </Tabs.Content>
      <Tabs.Content value="exam" class="pt-3">
        <ExamPanel />
      </Tabs.Content>
      <Tabs.Content value="brush" class="space-y-4 pt-3">
        <TaskControlPanel />
        <ChapterProgressPanel />
        <TaskProgressPanel />
        <RunLogPanel />
      </Tabs.Content>
    </Tabs.Root>
  </Panel>

  <Panel title="状态">
    <p
      class={cn(
        'text-xs whitespace-pre-wrap',
        app.statusKind === 'error' && 'text-destructive',
        app.statusKind === 'ok' && 'text-emerald-700 dark:text-emerald-400',
        !app.statusKind && 'text-muted-foreground',
      )}
      role="status"
      aria-live="polite"
    >
      {app.status || '尚未登录'}
    </p>
  </Panel>
</main>
