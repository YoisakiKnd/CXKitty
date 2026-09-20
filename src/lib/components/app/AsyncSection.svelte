<script>
  /**
   * Three-state async section: loading / empty / error(+retry) / ready.
   *
   * Every remote-data region renders through this so that the acceptance
   * requirement (loading, empty, failure-with-retry) is satisfied in one place
   * instead of being re-implemented per view.
   */
  import { Button } from '$lib/components/ui/button/index.js';
  import { mutedText } from '$lib/variants.js';
  import { cn } from '$lib/utils.js';

  let {
    status = 'idle',
    error = '',
    loadingText = '加载中…',
    emptyText = '暂无数据',
    idleText = '',
    retryLabel = '重试',
    onRetry = undefined,
    minHeight = '4rem',
    class: className,
    children,
  } = $props();

  const showRetry = $derived(status === 'error' && typeof onRetry === 'function');
</script>

{#if status === 'loading'}
  <div
    class={cn('flex items-center gap-2 py-3', className)}
    style={`min-height:${minHeight}`}
    role="status"
    aria-live="polite"
  >
    <span
      class="border-muted-foreground/40 border-t-foreground size-3.5 shrink-0 animate-spin rounded-full border-2"
      aria-hidden="true"
    ></span>
    <span class={mutedText()}>{loadingText}</span>
  </div>
{:else if status === 'error'}
  <div
    class={cn('flex flex-wrap items-center gap-2 py-3', className)}
    style={`min-height:${minHeight}`}
    role="alert"
  >
    <span class="text-destructive text-xs">加载失败：{error || '未知错误'}</span>
    {#if showRetry}
      <Button variant="outline" size="sm" onclick={onRetry}>{retryLabel}</Button>
    {/if}
  </div>
{:else if status === 'empty'}
  <p class={cn(mutedText(), 'py-3', className)} style={`min-height:${minHeight}`}>{emptyText}</p>
{:else if status === 'idle' && idleText}
  <p class={cn(mutedText(), 'py-3', className)} style={`min-height:${minHeight}`}>{idleText}</p>
{:else}
  {@render children?.()}
{/if}
