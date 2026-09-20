<script>
  /**
   * 考试 panel (preserved from the original 考试 tab).
   *
   * Columns 考试 / 状态 / 截止. The baseline rendered the exam status with a
   * neutral (secondary) badge because the backend returns a plain label with no
   * code to colour by, so the chip stays `muted` here as well.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import * as Table from '$lib/components/ui/table/index.js';
  import AsyncSection from '../app/AsyncSection.svelte';
  import Panel from '../app/Panel.svelte';
  import StatusBadge from '../app/StatusBadge.svelte';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const courses = app.courses;
  const exams = app.exams;

  const rows = $derived(exams.data ?? []);
  const canLoad = $derived(Boolean(courses.activeCourse) && !exams.loading);
</script>

<Panel
  title="考试"
  description={courses.activeCourse
    ? `课程：${courses.activeCourse.name}`
    : '选择课程后可拉取考试列表。'}
>
  {#snippet actions()}
    <Button
      variant="secondary"
      size="sm"
      onclick={() => exams.load()}
      disabled={!canLoad}
      aria-busy={exams.loading}
    >
      {exams.loading ? '拉取中…' : '拉取本课考试'}
    </Button>
  {/snippet}

  <AsyncSection
    status={exams.status}
    error={exams.error}
    onRetry={() => exams.retry()}
    loadingText="正在拉取考试…"
    emptyText="该课程没有考试。"
    idleText="选择课程后拉取考试列表。"
  >
    <div class="overflow-x-auto rounded-md border">
      <Table.Root>
        <Table.Header>
          <Table.Row>
            <Table.Head>考试</Table.Head>
            <Table.Head class="w-28">状态</Table.Head>
            <Table.Head class="w-44">截止</Table.Head>
          </Table.Row>
        </Table.Header>
        <Table.Body>
          {#each rows as row (row.exam_id)}
            <Table.Row>
              <Table.Cell class="max-w-[26rem] truncate font-medium" title={row.name}>
                {row.name}
              </Table.Cell>
              <Table.Cell>
                <StatusBadge tone="muted">{row.status}</StatusBadge>
              </Table.Cell>
              <Table.Cell class="text-muted-foreground">{row.expire_time || '—'}</Table.Cell>
            </Table.Row>
          {/each}
        </Table.Body>
      </Table.Root>
    </div>
  </AsyncSection>
</Panel>
