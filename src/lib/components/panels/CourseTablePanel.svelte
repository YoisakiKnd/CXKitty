<script>
  /**
   * 课程 panel — the multi-select, paginated course table from the original page.
   *
   * Columns 序号 / 课程名 / 老师 / 课程ID / 状态, page size 12, header checkbox that
   * selects the current page, and the 当前勾选 hint showing the comma-joined 序号
   * string the original backend used as the task command.
   *
   * Clicking a row makes it the *active* course (which drives 作业/考试/刷课);
   * the checkbox is the multi-selection used for the command string.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { Checkbox } from '$lib/components/ui/checkbox/index.js';
  import * as Table from '$lib/components/ui/table/index.js';
  import AsyncSection from '../app/AsyncSection.svelte';
  import Panel from '../app/Panel.svelte';
  import StatusBadge from '../app/StatusBadge.svelte';
  import { courseStateLabel, courseStateTone } from '$lib/format.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const courses = app.courses;

  const pageIds = $derived(courses.pageRows.map((row) => row.course_id));
  const allOnPageSelected = $derived(
    pageIds.length > 0 && pageIds.every((id) => courses.selectedIds.has(id)),
  );
  const someOnPageSelected = $derived(pageIds.some((id) => courses.selectedIds.has(id)));
</script>

<Panel
  title="课程"
  description={`共 ${courses.courses.length} 门课程，每页 ${courses.pageSize} 门。`}
>
  {#snippet actions()}
    <span class="text-muted-foreground text-xs tabular-nums">
      第 {courses.page} / {courses.totalPages} 页
    </span>
    <Button variant="outline" size="sm" onclick={() => courses.prevPage()} disabled={courses.page <= 1}>
      上一页
    </Button>
    <Button
      variant="outline"
      size="sm"
      onclick={() => courses.nextPage()}
      disabled={courses.page >= courses.totalPages}
    >
      下一页
    </Button>
    <Button variant="secondary" size="sm" onclick={() => app.refreshCourses()} disabled={courses.loading}>
      刷新
    </Button>
  {/snippet}

  <AsyncSection
    status={courses.status}
    error={courses.error}
    onRetry={() => app.refreshCourses()}
    loadingText="正在拉取课程…"
    emptyText="账号下没有可用课程。"
    idleText="登录后自动拉取课程。"
  >
    <div class="space-y-3">
      <div class="overflow-x-auto rounded-md border">
        <Table.Root>
          <Table.Header>
            <Table.Row>
              <Table.Head class="w-12">
                <Checkbox
                  checked={allOnPageSelected}
                  indeterminate={!allOnPageSelected && someOnPageSelected}
                  onCheckedChange={(value) => courses.setAllOnPage(value === true)}
                  aria-label="选择本页全部课程"
                />
              </Table.Head>
              <Table.Head class="w-16">序号</Table.Head>
              <Table.Head>课程名</Table.Head>
              <Table.Head>老师</Table.Head>
              <Table.Head>课程ID</Table.Head>
              <Table.Head class="w-24">状态</Table.Head>
            </Table.Row>
          </Table.Header>
          <Table.Body>
            {#each courses.pageRows as row (row.course_id)}
              {@const isActive = courses.activeCourseId === row.course_id}
              <Table.Row
                class={isActive ? 'bg-muted/50' : ''}
                data-state={isActive ? 'selected' : undefined}
              >
                <Table.Cell>
                  <Checkbox
                    checked={courses.selectedIds.has(row.course_id)}
                    onCheckedChange={(value) => courses.toggle(row.course_id, value === true)}
                    aria-label={`选择课程 ${row.name}`}
                  />
                </Table.Cell>
                <Table.Cell class="text-muted-foreground tabular-nums">{row.index}</Table.Cell>
                <Table.Cell class="max-w-[22rem] truncate font-medium" title={row.name}>
                  <button
                    type="button"
                    class="focus-visible:ring-ring/50 rounded text-left hover:underline focus-visible:ring-2 focus-visible:outline-none"
                    onclick={() => app.selectCourse(row.course_id)}
                    aria-pressed={isActive}
                    title="设为此课程（作业 / 考试 / 刷课以此课程为准）"
                  >
                    {row.name}
                  </button>
                </Table.Cell>
                <Table.Cell class="text-muted-foreground">{row.teacher_name || '—'}</Table.Cell>
                <Table.Cell class="text-muted-foreground tabular-nums">{row.course_id}</Table.Cell>
                <Table.Cell>
                  <StatusBadge tone={courseStateTone(row.state)}>
                    {courseStateLabel(row.state)}
                  </StatusBadge>
                </Table.Cell>
              </Table.Row>
            {/each}
          </Table.Body>
        </Table.Root>
      </div>

      <p class="text-muted-foreground text-xs">
        当前勾选：<span class="text-foreground tabular-nums"
          >{courses.selectedCommand() || '（未勾选）'}</span
        >
        {#if courses.selectedCount > 0}
          · 已选 {courses.selectedCount} 门
        {/if}
      </p>
    </div>
  </AsyncSection>
</Panel>
