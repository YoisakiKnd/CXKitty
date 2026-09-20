<script>
  /**
   * 任务进度 panel — the seven-field status grid plus the two progress bars.
   *
   * Fields (matching the original layout): 当前课程 / 当前章节 / 任务类型 /
   * 当前任务 / 课程进度 / 章节进度 / 任务点进度.
   *
   * Data provenance: every field is driven by real `brush-progress` events
   * (`currentCourse` / `chapterIndex` / `pointIndex` / `waitProgress` in
   * `BrushProgressEvent`), so nothing here is invented.
   */
  import ProgressBar from '../app/ProgressBar.svelte';
  import Panel from '../app/Panel.svelte';
  import StatusBadge from '../app/StatusBadge.svelte';
  import { indexText, ratioPercent } from '$lib/format.js';
  import { statusGrid } from '$lib/variants.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const brush = app.brush;

  /** The seven cells, in the original order. */
  const fields = $derived([
    { label: '当前课程', value: brush.courseName || '—' },
    { label: '当前章节', value: brush.currentChapter || '—' },
    { label: '任务类型', value: brush.currentTaskType || '—' },
    { label: '当前任务', value: brush.currentTask || '—' },
    {
      label: '课程进度',
      value: `${indexText(brush.completedCourses, brush.totalCourses)} 门`,
    },
    { label: '章节进度', value: indexText(brush.chapterIndex, brush.chapterTotal) },
    { label: '任务点进度', value: indexText(brush.pointIndex, brush.pointTotal) },
  ]);
</script>

<Panel title="任务进度" description="任务运行期间由后端进度事件实时更新。">
  {#snippet actions()}
    <StatusBadge tone={brush.running ? 'info' : 'muted'}>
      {brush.running ? '实时更新中' : '未运行'}
    </StatusBadge>
  {/snippet}

  <div class="space-y-4">
    <dl class={statusGrid()}>
      {#each fields as field (field.label)}
        <div class="min-w-0 space-y-1">
          <dt class="text-muted-foreground text-[0.6875rem] font-medium">{field.label}</dt>
          <dd class="truncate text-xs font-medium" title={field.value}>{field.value}</dd>
        </div>
      {/each}
    </dl>

    <div class="grid gap-4 sm:grid-cols-2">
      <ProgressBar
        label="当前任务进度"
        value={brush.taskProgress}
        text={brush.taskProgressText}
        tone="primary"
      />
      <ProgressBar
        label="冷却等待"
        value={brush.waitProgress}
        text={brush.waitProgressText}
        tone="wait"
      />
    </div>

    {#if brush.taskProgress > 0}
      <p class="text-muted-foreground text-[0.6875rem]">
        当前任务进度：{ratioPercent(brush.taskProgress)}
      </p>
    {/if}
  </div>
</Panel>
