<script>
  /**
   * 作业 panel (preserved from the original 作业 tab).
   *
   * Columns 作业 / 状态 / 截止 / 成绩. The status chip maps the backend
   * `status_code` (`UNDONE` / `DONE` / `EXPIRED` / `UNDONE_READ` / `OTHER`)
   * through the shared tone table so the colours match the baseline badges.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Table from '$lib/components/ui/table/index.js';
  import AsyncSection from '../app/AsyncSection.svelte';
  import Panel from '../app/Panel.svelte';
  import StatusBadge from '../app/StatusBadge.svelte';
  import { homeworkStatusTone } from '$lib/format.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const courses = app.courses;
  const homework = app.homework;

  const rows = $derived(homework.data ?? []);
  const canLoad = $derived(Boolean(courses.activeCourse) && !homework.loading);
</script>

<Panel
  title="作业"
  description={courses.activeCourse
    ? `课程：${courses.activeCourse.name}`
    : '选择课程后可拉取作业列表。'}
>
  {#snippet actions()}
    <Button
      variant="secondary"
      size="sm"
      onclick={() => homework.load()}
      disabled={!canLoad}
      aria-busy={homework.loading}
    >
      {homework.loading ? '拉取中…' : '拉取本课作业'}
    </Button>
  {/snippet}

  <AsyncSection
    status={homework.status}
    error={homework.error}
    onRetry={() => homework.retry()}
    loadingText="正在拉取作业…"
    emptyText="该课程没有作业。"
    idleText="选择课程后拉取作业列表。"
  >
    <div class="overflow-x-auto rounded-md border">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head>作业</Table.Head>
            <Table.Head class="w-28">状态</Table.Head>
            <Table.Head class="w-44">截止</Table.Head>
            <Table.Head class="w-24">成绩</Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each rows as row (row.work_name + row.end_time)}
            <Table.Row>
              <Table.Cell class="max-w-[26rem] truncate font-medium" title={row.work_name}>
                {row.work_name}
              </Table.Cell>
              <Table.Cell>
                <StatusBadge tone={homeworkStatusTone(row.status_code)}>{row.status}</StatusBadge>
              </Table.Cell>
              <Table.Cell class="text-muted-foreground">{row.end_time || '—'}</Table.Cell>
              <Table.Cell class="tabular-nums">{row.result_num || '—'}</Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  </AsyncSection>
</Panel>
