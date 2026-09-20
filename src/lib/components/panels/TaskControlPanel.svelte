<script>
  /**
   * 任务 panel — start/stop control plus the task-state badge.
   *
   * The badge mirrors the original IDLE / RUNNING / SUCCESS / FAILED states
   * (`render_task_badge`) and is driven by the real brush event stream.
   *
   * Scope: like the original's comma-joined 序号 command, `start_brush` runs
   * every *checked* course in list order. 当前勾选 shows that same selection.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import { Checkbox } from '$lib/components/ui/checkbox/index.js';
  import StatusBadge from '../app/StatusBadge.svelte';
  import Panel from '../app/Panel.svelte';
  import { taskStateLabel, taskStateTone } from '$lib/format.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const courses = app.courses;
  const brush = app.brush;
</script>

<Panel title="任务" description="按「当前勾选」的课程顺序启动刷课任务。">
  {#snippet actions()}
    <StatusBadge tone={taskStateTone(brush.taskState)}>{taskStateLabel(brush.taskState)}</StatusBadge>
  {/snippet}

  <div class="space-y-3">
    <div class="flex flex-wrap items-center gap-2">
      <Button onclick={() => app.startTask()} disabled={!app.canStartTask}>
        {brush.starting ? '启动中…' : '开始刷课'}
      </Button>
      <Button variant="secondary" onclick={() => app.stopTask()} disabled={!brush.running || brush.stopping}>
        {brush.stopping ? '停止中…' : '停止'}
      </Button>
      <Button
        variant="ghost"
        onclick={() => brush.clear()}
        disabled={brush.running || (!brush.hasLogs && brush.points.length === 0)}
      >
        清空记录
      </Button>
    </div>

    <div class="flex flex-wrap items-end gap-4">
      <div class="flex items-center gap-2 pb-1">
        <Checkbox
          checked={brush.unfinishedOnly}
          onCheckedChange={(value) => brush.setUnfinishedOnly(value === true)}
          disabled={brush.running}
          aria-label="仅未完成章节"
        />
        <span class="text-xs">仅未完成章节</span>
      </div>

      <div class="space-y-1.5">
        <Label for="video-speed">视频倍速</Label>
        <Input
          id="video-speed"
          type="number"
          min="0.5"
          max="2"
          step="0.5"
          value={brush.videoSpeed}
          oninput={(e) => brush.setVideoSpeed(Number(e.currentTarget.value))}
          disabled={brush.running}
          aria-describedby="video-speed-hint"
          class="h-7 w-24"
        />
        <p id="video-speed-hint" class="text-muted-foreground text-[0.6875rem]">范围 0.5–2</p>
      </div>

      <div class="min-w-0 pb-1 text-xs">
        <span class="text-muted-foreground">当前课程：</span>
        <span class="font-medium">{courses.activeCourse?.name ?? '（未选择）'}</span>
      </div>

      <div class="min-w-0 pb-1 text-xs">
        <span class="text-muted-foreground">当前勾选：</span>
        <span class="tabular-nums">{courses.selectedCommand() || '（未勾选）'}</span>
      </div>
    </div>

    {#if brush.startError}
      <p class="text-destructive text-xs" role="alert">启动失败：{brush.startError}</p>
    {/if}

    <p class="text-muted-foreground text-[0.6875rem] leading-snug">
      开始刷课会对「当前勾选」的课程按序号顺序依次执行；未勾选任何课程时不可启动。
    </p>
  </div>
</Panel>
