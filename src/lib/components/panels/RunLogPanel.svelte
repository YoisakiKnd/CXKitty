<script>
  /**
   * 运行日志 panel.
   *
   * Matches the original collapsible log region: collapsed by default, a
   * read-only monospace log surface, and auto-scroll to the newest line.
   *
   * Implemented with native `<details>` / `<summary>` — it is keyboard
   * operable and announced correctly by assistive tech without adding any
   * dependency, and the body uses a read-only `<textarea>` like the baseline
   * (so the text is selectable and copyable but not editable).
   */
  import { cn } from '$lib/utils.js';
  import { getApp } from '$lib/stores/app.svelte.js';

  const app = getApp();
  const brush = app.brush;

  let open = $state(false);
  /** @type {HTMLTextAreaElement | undefined} */
  let logEl = $state();

  const LOG_PLACEHOLDER = '任务日志会显示在这里';

  // Keep the newest line visible while the task streams output.
  $effect(() => {
    // Touch the dependency explicitly so the effect re-runs per log entry.
    const count = brush.logs.length;
    if (!open || count === 0 || !logEl) return;
    logEl.scrollTop = logEl.scrollHeight;
  });

  const text = $derived(brush.logs.length ? brush.logs.join('\n') : LOG_PLACEHOLDER);
</script>

<details
  class="ring-foreground/10 bg-card text-card-foreground rounded-lg text-xs/relaxed ring-1"
  bind:open
>
  <summary
    class="focus-visible:ring-ring/50 flex cursor-pointer list-none items-center justify-between gap-2 rounded-lg px-4 py-3 font-medium select-none focus-visible:ring-2 focus-visible:outline-none"
  >
    <span class="flex items-center gap-2">
      运行日志
      {#if brush.logs.length}
        <span class="text-muted-foreground text-[0.6875rem] font-normal tabular-nums">
          {brush.logs.length} 行
        </span>
      {/if}
    </span>
    <span class="text-muted-foreground text-[0.6875rem] font-normal">{open ? '收起' : '展开'}</span>
  </summary>

  <div class="px-4 pb-4">
    <label class="sr-only" for="run-log">运行日志内容</label>
    <textarea
      id="run-log"
      bind:this={logEl}
      readonly
      spellcheck="false"
      class={cn(
        'rounded-md bg-zinc-950 p-3 font-mono text-[0.75rem] leading-relaxed text-zinc-100',
        'h-[320px] w-full resize-y break-all whitespace-pre-wrap sm:h-[520px]',
        'focus-visible:ring-ring/50 focus-visible:ring-2 focus-visible:outline-none',
      )}
      value={text}
    ></textarea>
  </div>
</details>
