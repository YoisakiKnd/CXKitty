<script>
  /**
   * 章节进度 panel.
   *
   * Lists every chapter of the active course with its finished/total point count
   * and a status chip derived the same way as the original backend
   * (done > active > partial > pending, see `sync_chapter_items`).
   *
   * Selecting a chapter reveals its 任务点预览 (real `list_task_points` call),
   * which is how the original page let you inspect a chapter without running.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Table from '$lib/components/ui/table/index.js';
  import AsyncSection from '../app/AsyncSection.svelte';
  import Panel from '../app/Panel.svelte';
  import StatusBadge from '../app/StatusBadge.svelte';
  import {
    chapterStatusLabel,
    chapterStatusTone,
    indexText,
    pointStatusLabel,
    pointStatusTone,
  } from '$lib/format.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const courses = app.courses;
  const brush = app.brush;

  const items = $derived(brush.chapterItems);
</script>

<Panel
  title="章节进度"
  description={courses.activeCourse ? `课程：${courses.activeCourse.name}` : '选择课程后显示章节结构。'}
>
  {#snippet actions()}
    <Button
      variant="outline"
      size="sm"
      onclick={() => brush.loadChapters(courses.activeCourse)}
      disabled={!courses.activeCourse || brush.chaptersStatus === 'loading'}
    >
      拉取章节
    </Button>
  {/snippet}

  <AsyncSection
    status={brush.chaptersStatus}
    error={brush.chaptersError}
    onRetry={() => brush.loadChapters(courses.activeCourse)}
    loadingText="正在拉取章节…"
    emptyText="该课程没有章节数据。"
    idleText="选择课程后显示章节结构。"
  >
    <div class="space-y-3">
      <ul class="space-y-1.5">
        {#each items as chapter (chapter.key)}
          <li
            class="flex flex-wrap items-center justify-between gap-2 rounded-md border px-3 py-2 text-xs"
            style={`margin-left:${Math.min(chapter.layer ?? 0, 6) * 0.75}rem`}
          >
            <span class="min-w-0 flex-1 truncate" title={`${chapter.label} ${chapter.name}`}>
              <strong>{chapter.label}</strong>
              {chapter.name}
            </span>
            <span class="flex shrink-0 items-center gap-2">
              <span class="text-muted-foreground tabular-nums">
                {indexText(chapter.pointFinished, chapter.pointTotal)}
              </span>
              <StatusBadge tone={chapterStatusTone(chapter.status)}>
                {chapterStatusLabel(chapter.status)}
              </StatusBadge>
              <Button
                variant="ghost"
                size="sm"
                onclick={() => app.selectChapter(chapter.key)}
                aria-pressed={app.selectedChapterId === chapter.key}
              >
                任务点
              </Button>
            </span>
          </li>
        {/each}
      </ul>

      {#if app.selectedChapterId != null}
        <AsyncSection
          status={app.taskPoints.status}
          error={app.taskPoints.error}
          onRetry={() => app.taskPoints.retry()}
          loadingText="正在解析任务点…"
          emptyText="该章节没有任务点。"
          minHeight="2rem"
        >
          <div class="overflow-x-auto rounded-md border">
            <Table.Root>
              <Table.Header>
                <Table.Row>
                  <Table.Head class="w-24">类型</Table.Head>
                  <Table.Head>标题</Table.Head>
                  <Table.Head class="w-40">章节</Table.Head>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {#each app.taskPoints.data ?? [] as point}
                  <Table.Row>
                    <Table.Cell>
                      <StatusBadge tone="muted">
                        {point.kind === 'video'
                          ? '视频任务'
                          : point.kind === 'document'
                            ? '文档任务'
                            : point.kind === 'work'
                              ? '章节测验'
                              : point.kind === 'exam'
                                ? '考试'
                                : point.kind}
                      </StatusBadge>
                    </Table.Cell>
                    <Table.Cell class="max-w-[26rem] truncate" title={point.title}>{point.title}</Table.Cell>
                    <Table.Cell class="text-muted-foreground">{point.chapter_label}</Table.Cell>
                  </Table.Row>
                {/each}
              </Table.Body>
            </Table.Root>
          </div>
        </AsyncSection>
      {/if}

      {#if brush.points.length > 0}
        <div class="space-y-2">
          <h3 class="text-xs font-semibold">本次运行任务点状态</h3>
          <div class="overflow-x-auto rounded-md border">
            <Table.Root>
              <Table.Header>
                <Table.Row>
                  <Table.Head class="w-24">类型</Table.Head>
                  <Table.Head>标题</Table.Head>
                  <Table.Head class="w-24">状态</Table.Head>
                  <Table.Head>说明</Table.Head>
                </Table.Row>
              </Table.Header>
              <Table.Body>
                {#each brush.points as point (point.key)}
                  <Table.Row>
                    <Table.Cell>
                      <StatusBadge tone="muted">
                        {point.kind === 'video'
                          ? '视频任务'
                          : point.kind === 'document'
                            ? '文档任务'
                            : point.kind === 'work'
                              ? '章节测验'
                              : point.kind === 'exam'
                                ? '考试'
                                : point.kind}
                      </StatusBadge>
                    </Table.Cell>
                    <Table.Cell class="max-w-[26rem] truncate" title={point.title}>{point.title}</Table.Cell>
                    <Table.Cell>
                      <StatusBadge tone={pointStatusTone(point.status)}>
                        {pointStatusLabel(point.status)}
                      </StatusBadge>
                    </Table.Cell>
                    <Table.Cell class="text-muted-foreground max-w-[24rem] truncate" title={point.message}>
                      {point.message}
                    </Table.Cell>
                  </Table.Row>
                {/each}
              </Table.Body>
            </Table.Root>
          </div>
        </div>
      {/if}
    </div>
  </AsyncSection>
</Panel>
