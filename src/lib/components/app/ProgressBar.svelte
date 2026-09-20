<script>
  /**
   * Labelled progress bar with an accessible `progressbar` role.
   * Used by 当前任务进度 and 冷却等待.
   */
  import { clampRatio } from '$lib/format.js';
  import { progressFill, progressTrack } from '$lib/variants.js';
  import { cn } from '$lib/utils.js';

  let {
    value = 0,
    label = '',
    text = '-',
    tone = 'primary',
    class: className,
  } = $props();

  const ratio = $derived(clampRatio(value));
  const percent = $derived(Math.round(ratio * 100));
</script>

<div class={cn('space-y-1.5', className)}>
  <div class="flex items-center justify-between gap-2">
    <span class="text-muted-foreground text-xs font-medium">{label}</span>
    <span class="text-muted-foreground text-xs tabular-nums">{text}</span>
  </div>

  <div
    class={progressTrack()}
    role="progressbar"
    aria-label={label}
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={percent}
    aria-valuetext={text}
  >
    <div class={progressFill({ tone })} style={`width:${percent}%`}></div>
  </div>
</div>
