<script>
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { onMount, onDestroy } from 'svelte';

  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Label } from '$lib/components/ui/label/index.js';
  import * as Card from '$lib/components/ui/card/index.js';
  import * as Tabs from '$lib/components/ui/tabs/index.js';
  import * as Table from '$lib/components/ui/table/index.js';
  import { Badge } from '$lib/components/ui/badge/index.js';
  import { Checkbox } from '$lib/components/ui/checkbox/index.js';
  import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
  import { Separator } from '$lib/components/ui/separator/index.js';
  import { cn } from '$lib/utils.js';

  let phone = $state('');
  let password = $state('');
  let busy = $state(false);
  let status = $state('尚未登录');
  let statusKind = $state('');
  let account = $state(null);
  let courses = $state([]);
  let selectedCourseId = $state(null);
  let tab = $state('homework'); // homework | exam | brush
  let homework = $state([]);
  let exams = $state([]);
  let chapters = $state([]);
  let taskPoints = $state([]);
  let selectedChapterId = $state(null);

  // 刷课
  let brushRunning = $state(false);
  let brushLogs = $state([]);
  let brushPointStatus = $state([]); // {title, kind, status, message}
  let brushMeta = $state({ chapter: '', playing: null, duration: null });
  let videoSpeed = $state(1.0);
  let unfinishedOnly = $state(true);
  let unlistenBrush = null;

  function setStatus(msg, kind = '') {
    status = msg;
    statusKind = kind;
  }

  function selectedCourse() {
    return courses.find((c) => c.course_id === selectedCourseId) || null;
  }

  function pushLog(line) {
    const ts = new Date().toLocaleTimeString('zh-CN', { hour12: false });
    brushLogs = [...brushLogs.slice(-400), `[${ts}] ${line}`];
  }

  function upsertPoint(ev) {
    if (!ev.pointTitle && !ev.point_title) return;
    const title = ev.pointTitle ?? ev.point_title;
    const kind = ev.pointKind ?? ev.point_kind ?? '';
    const st = ev.pointStatus ?? ev.point_status ?? 'running';
    const message = ev.message || '';
    const idx = brushPointStatus.findIndex((p) => p.title === title && p.kind === kind);
    const row = { title, kind, status: st, message };
    if (idx >= 0) {
      const copy = brushPointStatus.slice();
      copy[idx] = row;
      brushPointStatus = copy;
    } else {
      brushPointStatus = [...brushPointStatus, row];
    }
  }

  onMount(async () => {
    try {
      brushRunning = await invoke('brush_running');
    } catch (_) {
      /* not in tauri */
    }
    try {
      unlistenBrush = await listen('brush-progress', (event) => {
        const ev = event.payload || {};
        const kind = ev.kind || 'log';
        const msg = ev.message || '';
        if (kind === 'log' || kind === 'chapter' || kind === 'done' || kind === 'stopped' || kind === 'error') {
          pushLog(msg);
        }
        if (kind === 'point') {
          pushLog(msg);
          upsertPoint(ev);
        }
        if (ev.chapterLabel || ev.chapter_label) {
          brushMeta = {
            ...brushMeta,
            chapter: ev.chapterLabel ?? ev.chapter_label,
          };
        }
        if (ev.playing != null || ev.duration != null) {
          brushMeta = {
            ...brushMeta,
            playing: ev.playing ?? null,
            duration: ev.duration ?? null,
          };
        }
        if (kind === 'done' || kind === 'stopped' || kind === 'error') {
          brushRunning = false;
          if (kind === 'done') setStatus(msg, 'ok');
          else if (kind === 'stopped') setStatus(msg, '');
          else setStatus(msg, 'error');
        }
      });
    } catch (_) {
      /* browser preview */
    }
  });

  onDestroy(() => {
    if (unlistenBrush) unlistenBrush();
  });

  async function doLogin() {
    if (!phone.trim() || !password) {
      setStatus('请输入账号和密码', 'error');
      return;
    }
    busy = true;
    setStatus('正在登录…');
    try {
      account = await invoke('login', { phone: phone.trim(), password });
      setStatus(`登录成功：${account.name}`, 'ok');
      await loadCourses();
    } catch (e) {
      account = null;
      courses = [];
      homework = [];
      exams = [];
      chapters = [];
      taskPoints = [];
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function loadCourses() {
    busy = true;
    try {
      courses = await invoke('list_courses');
      selectedCourseId = courses[0]?.course_id ?? null;
      homework = [];
      exams = [];
      chapters = [];
      taskPoints = [];
      setStatus(`课程 ${courses.length} 门`, 'ok');
    } catch (e) {
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function loadHomework() {
    const course = selectedCourse();
    if (!course) return setStatus('请先选择课程', 'error');
    busy = true;
    setStatus(`拉取作业：${course.name}…`);
    try {
      homework = await invoke('list_homework', {
        courseId: course.course_id,
        classId: course.class_id,
        cpi: course.cpi,
        courseName: course.name,
      });
      setStatus(`作业 ${homework.length} 项`, 'ok');
    } catch (e) {
      homework = [];
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function loadExams() {
    const course = selectedCourse();
    if (!course) return setStatus('请先选择课程', 'error');
    busy = true;
    setStatus(`拉取考试：${course.name}…`);
    try {
      exams = await invoke('list_exams', {
        courseId: course.course_id,
        classId: course.class_id,
        cpi: course.cpi,
        courseName: course.name,
      });
      setStatus(`考试 ${exams.length} 项`, 'ok');
    } catch (e) {
      exams = [];
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function loadChapters() {
    const course = selectedCourse();
    if (!course) return setStatus('请先选择课程', 'error');
    busy = true;
    setStatus(`拉取章节：${course.name}…`);
    try {
      chapters = await invoke('list_chapters', {
        courseId: course.course_id,
        classId: course.class_id,
        cpi: course.cpi,
        key: course.key,
      });
      taskPoints = [];
      selectedChapterId = chapters[0]?.chapter_id ?? null;
      setStatus(`章节 ${chapters.length} 个`, 'ok');
    } catch (e) {
      chapters = [];
      taskPoints = [];
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function loadTaskPoints(ch) {
    const course = selectedCourse();
    if (!course || !ch) return;
    busy = true;
    selectedChapterId = ch.chapter_id;
    setStatus(`解析任务点：${ch.label} ${ch.name}…`);
    try {
      taskPoints = await invoke('list_task_points', {
        courseId: course.course_id,
        chapter: ch,
      });
      setStatus(`任务点 ${taskPoints.length} 个`, 'ok');
    } catch (e) {
      taskPoints = [];
      setStatus(String(e), 'error');
    } finally {
      busy = false;
    }
  }

  async function startBrush() {
    const course = selectedCourse();
    if (!course) return setStatus('请先选择课程', 'error');
    brushLogs = [];
    brushPointStatus = [];
    brushMeta = { chapter: '', playing: null, duration: null };
    pushLog(`准备刷课：${course.name}`);
    busy = true;
    try {
      const res = await invoke('start_brush', {
        course,
        options: {
          video_speed: Number(videoSpeed) || 1.0,
          video_report_rate: 58,
          unfinished_only: unfinishedOnly,
        },
      });
      brushRunning = true;
      setStatus(res.message || '刷课已开始', 'ok');
      pushLog(res.message || '刷课已开始');
    } catch (e) {
      brushRunning = false;
      setStatus(String(e), 'error');
      pushLog(`启动失败：${e}`);
    } finally {
      busy = false;
    }
  }

  async function stopBrush() {
    try {
      const res = await invoke('stop_brush');
      setStatus(res.message || '已请求停止', '');
      pushLog(res.message || '已请求停止');
    } catch (e) {
      setStatus(String(e), 'error');
    }
  }

  function onSelectCourse(id) {
    selectedCourseId = id;
    homework = [];
    exams = [];
    chapters = [];
    taskPoints = [];
  }

  /** Homework status_code → badge class */
  function hwBadgeClass(code) {
    switch (code) {
      case 'UNDONE':
        return 'badge-undone';
      case 'DONE':
        return 'badge-done';
      case 'EXPIRED':
        return 'badge-expired';
      case 'UNDONE_READ':
        return 'badge-skipped';
      default:
        return 'badge-other';
    }
  }

  /** Brush point status → badge class */
  function pointBadgeClass(st) {
    switch (st) {
      case 'done':
        return 'badge-done';
      case 'failed':
      case 'error':
        return 'badge-error';
      case 'skipped':
        return 'badge-skipped';
      case 'running':
        return 'badge-running';
      default:
        return 'badge-other';
    }
  }

  function pointLabel(st) {
    switch (st) {
      case 'done':
        return '已完成';
      case 'failed':
      case 'error':
        return '错误';
      case 'skipped':
        return '跳过';
      case 'running':
        return '进行中';
      default:
        return st || '未知';
    }
  }
</script>

<main class="space-y-4">
  <header class="space-y-1">
    <h1 class="text-xl font-semibold tracking-tight">学习通工具箱</h1>
    <p class="text-muted-foreground text-xs">
      单账号会话 · 作业 / 考试 / 刷课 · Rust + Svelte + Tauri 2 · GPL-3.0
    </p>
  </header>

  <Card.Root>
    <Card.Header class="pb-3">
      <Card.Title>登录</Card.Title>
      <Card.Description>使用学习通账号登录后可拉取课程与任务。</Card.Description>
    </Card.Header>
    <Card.Content>
      <div class="flex flex-wrap items-end gap-3">
        <div class="min-w-[160px] flex-1 space-y-1.5">
          <Label for="phone">手机号 / 学号</Label>
          <Input
            id="phone"
            bind:value={phone}
            autocomplete="username"
            disabled={busy}
            placeholder="账号"
          />
        </div>
        <div class="min-w-[160px] flex-1 space-y-1.5">
          <Label for="password">密码</Label>
          <Input
            id="password"
            type="password"
            bind:value={password}
            autocomplete="current-password"
            disabled={busy}
            placeholder="密码"
            onkeydown={(e) => e.key === 'Enter' && doLogin()}
          />
        </div>
        <Button onclick={doLogin} disabled={busy}>登录</Button>
      </div>
      {#if account}
        <p class="text-muted-foreground mt-3 text-xs">
          当前：{account.name} · {account.school || '未知学校'} · puid={account.puid}
        </p>
      {/if}
    </Card.Content>
  </Card.Root>

  {#if account}
    <Card.Root>
      <Card.Header class="pb-3">
        <div class="flex items-center justify-between gap-2">
          <Card.Title>课程</Card.Title>
          <Button variant="secondary" size="sm" onclick={loadCourses} disabled={busy || brushRunning}
            >刷新</Button
          >
        </div>
      </Card.Header>
      <Card.Content>
        {#if courses.length === 0}
          <p class="text-muted-foreground text-xs">暂无课程</p>
        {:else}
          <div class="flex flex-wrap gap-1.5">
            {#each courses as c}
              <Button
                size="sm"
                variant={selectedCourseId === c.course_id ? 'default' : 'outline'}
                disabled={busy || brushRunning}
                onclick={() => onSelectCourse(c.course_id)}
              >
                {c.name}
              </Button>
            {/each}
          </div>
        {/if}
      </Card.Content>
    </Card.Root>

    <Card.Root>
      <Card.Content class="pt-4">
        <Tabs.Root bind:value={tab}>
          <Tabs.List>
            <Tabs.Trigger value="homework">作业</Tabs.Trigger>
            <Tabs.Trigger value="exam">考试</Tabs.Trigger>
            <Tabs.Trigger value="brush">刷课</Tabs.Trigger>
          </Tabs.List>

          <Tabs.Content value="homework" class="space-y-3 pt-3">
            <Button onclick={loadHomework} disabled={busy || !selectedCourseId}>拉取本课作业</Button>
            {#if homework.length === 0}
              <p class="text-muted-foreground text-xs">选择课程后拉取作业列表。</p>
            {:else}
              <div class="rounded-md border">
                <Table.Root>
                  <Table.Header>
                    <Table.Row>
                      <Table.Head>作业</Table.Head>
                      <Table.Head>状态</Table.Head>
                      <Table.Head>截止</Table.Head>
                      <Table.Head>成绩</Table.Head>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {#each homework as w}
                      <Table.Row>
                        <Table.Cell>{w.work_name}</Table.Cell>
                        <Table.Cell>
                          <Badge class={hwBadgeClass(w.status_code)}>{w.status}</Badge>
                        </Table.Cell>
                        <Table.Cell class="text-muted-foreground">{w.end_time}</Table.Cell>
                        <Table.Cell>{w.result_num}</Table.Cell>
                      </Table.Row>
                    {/each}
                  </Table.Body>
                </Table.Root>
              </div>
            {/if}
          </Tabs.Content>

          <Tabs.Content value="exam" class="space-y-3 pt-3">
            <Button onclick={loadExams} disabled={busy || !selectedCourseId}>拉取本课考试</Button>
            {#if exams.length === 0}
              <p class="text-muted-foreground text-xs">选择课程后拉取考试列表。</p>
            {:else}
              <div class="rounded-md border">
                <Table.Root>
                  <Table.Header>
                    <Table.Row>
                      <Table.Head>考试</Table.Head>
                      <Table.Head>状态</Table.Head>
                      <Table.Head>截止</Table.Head>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {#each exams as e}
                      <Table.Row>
                        <Table.Cell>{e.name}</Table.Cell>
                        <Table.Cell>
                          <Badge variant="secondary">{e.status}</Badge>
                        </Table.Cell>
                        <Table.Cell class="text-muted-foreground">{e.expire_time || '—'}</Table.Cell>
                      </Table.Row>
                    {/each}
                  </Table.Body>
                </Table.Root>
              </div>
            {/if}
          </Tabs.Content>

          <Tabs.Content value="brush" class="space-y-3 pt-3">
            <div class="flex flex-wrap items-center gap-2">
              <Button
                onclick={loadChapters}
                disabled={busy || brushRunning || !selectedCourseId}>拉取章节</Button
              >
              <Button
                onclick={startBrush}
                disabled={busy || brushRunning || !selectedCourseId}>开始刷课</Button
              >
              <Button variant="secondary" onclick={stopBrush} disabled={!brushRunning}>停止</Button>
            </div>

            <div class="text-muted-foreground flex flex-wrap items-center gap-4 text-xs">
              <label class="flex items-center gap-2">
                <Checkbox bind:checked={unfinishedOnly} disabled={brushRunning} />
                仅未完成章节
              </label>
              <div class="flex items-center gap-2">
                <Label for="video-speed" class="text-xs">视频倍速</Label>
                <Input
                  id="video-speed"
                  type="number"
                  min="0.5"
                  max="2"
                  step="0.5"
                  bind:value={videoSpeed}
                  disabled={brushRunning}
                  class="h-7 w-[4.5rem]"
                />
              </div>
            </div>

            {#if brushRunning || brushLogs.length}
              <div class="flex flex-wrap items-center gap-2 text-xs">
                {#if brushRunning}
                  <Badge class="badge-running">进行中</Badge>
                {:else}
                  <Badge variant="secondary">已结束</Badge>
                {/if}
                {#if brushMeta.chapter}
                  <span class="text-muted-foreground">当前章节：{brushMeta.chapter}</span>
                {/if}
                {#if brushMeta.playing != null && brushMeta.duration != null}
                  <span class="text-muted-foreground"
                    >播放 {brushMeta.playing}/{brushMeta.duration}s</span
                  >
                {/if}
              </div>
            {/if}

            {#if brushPointStatus.length}
              <Separator />
              <h2 class="text-sm font-medium">任务点状态</h2>
              <div class="rounded-md border">
                <Table.Root>
                  <Table.Header>
                    <Table.Row>
                      <Table.Head>类型</Table.Head>
                      <Table.Head>标题</Table.Head>
                      <Table.Head>状态</Table.Head>
                      <Table.Head>说明</Table.Head>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {#each brushPointStatus as p}
                      <Table.Row>
                        <Table.Cell>
                          <Badge variant="outline">{p.kind}</Badge>
                        </Table.Cell>
                        <Table.Cell>{p.title}</Table.Cell>
                        <Table.Cell>
                          <Badge class={pointBadgeClass(p.status)}>{pointLabel(p.status)}</Badge>
                        </Table.Cell>
                        <Table.Cell class="text-muted-foreground text-xs">{p.message}</Table.Cell>
                      </Table.Row>
                    {/each}
                  </Table.Body>
                </Table.Root>
              </div>
            {/if}

            {#if brushLogs.length}
              <Separator />
              <h2 class="text-sm font-medium">刷课日志</h2>
              <ScrollArea class="h-[280px] rounded-md border">
                <pre class="brush-log m-0 border-0">{brushLogs.join('\n')}</pre>
              </ScrollArea>
            {/if}

            {#if chapters.length === 0}
              <p class="text-muted-foreground text-xs">
                选择课程后可「拉取章节」浏览，或直接「开始刷课」自动完成视频 / 文档任务点。测验、作业、考试类任务点会标记为「已跳过-非刷课类型」，不会作答。
              </p>
            {:else}
              <Separator />
              <h2 class="text-sm font-medium">章节一览</h2>
              <div class="rounded-md border">
                <Table.Root>
                  <Table.Header>
                    <Table.Row>
                      <Table.Head>章节</Table.Head>
                      <Table.Head>进度</Table.Head>
                      <Table.Head class="w-[5rem]"></Table.Head>
                    </Table.Row>
                  </Table.Header>
                  <Table.Body>
                    {#each chapters as ch}
                      <Table.Row
                        class={cn(selectedChapterId === ch.chapter_id && 'bg-muted/50')}
                      >
                        <Table.Cell style={`padding-left: ${0.75 + ch.layer * 0.75}rem`}>
                          <strong>{ch.label}</strong>
                          {ch.name}
                        </Table.Cell>
                        <Table.Cell class="text-muted-foreground"
                          >{ch.point_finished}/{ch.point_total}</Table.Cell
                        >
                        <Table.Cell>
                          <Button
                            variant="ghost"
                            size="sm"
                            disabled={busy || brushRunning}
                            onclick={() => loadTaskPoints(ch)}>任务点</Button
                          >
                        </Table.Cell>
                      </Table.Row>
                    {/each}
                  </Table.Body>
                </Table.Root>
              </div>
              {#if taskPoints.length}
                <h2 class="text-sm font-medium">任务点预览</h2>
                <div class="rounded-md border">
                  <Table.Root>
                    <Table.Header>
                      <Table.Row>
                        <Table.Head>类型</Table.Head>
                        <Table.Head>标题</Table.Head>
                        <Table.Head>章节</Table.Head>
                      </Table.Row>
                    </Table.Header>
                    <Table.Body>
                      {#each taskPoints as p}
                        <Table.Row>
                          <Table.Cell>
                            <Badge variant="outline">{p.kind}</Badge>
                          </Table.Cell>
                          <Table.Cell>{p.title}</Table.Cell>
                          <Table.Cell class="text-muted-foreground">{p.chapter_label}</Table.Cell>
                        </Table.Row>
                      {/each}
                    </Table.Body>
                  </Table.Root>
                </div>
              {/if}
            {/if}
          </Tabs.Content>
        </Tabs.Root>
      </Card.Content>
    </Card.Root>
  {/if}

  <Card.Root>
    <Card.Header class="pb-2">
      <Card.Title class="text-sm">状态</Card.Title>
    </Card.Header>
    <Card.Content>
      <p
        class={cn(
          'text-sm whitespace-pre-wrap',
          statusKind === 'error' && 'text-destructive',
          statusKind === 'ok' && 'text-emerald-700 dark:text-emerald-400',
          !statusKind && 'text-muted-foreground',
        )}
      >
        {status}
      </p>
    </Card.Content>
  </Card.Root>
</main>
