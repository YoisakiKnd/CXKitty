<script>
  /**
   * Card shell shared by every panel.
   *
   * Gives all sections the same header/action/content spacing so the page reads
   * as one design instead of a pile of independently styled cards.
   */
  import * as Card from '$lib/components/ui/card/index.js';
  import { cn } from '$lib/utils.js';

  let {
    title = '',
    description = '',
    class: className,
    contentClass = '',
    actions,
    children,
  } = $props();

  const hasHeader = $derived(Boolean(title || description || actions));
</script>

<Card.Root class={className}>
  {#if hasHeader}
    <Card.Header class="pb-3">
      <div class="flex flex-wrap items-start justify-between gap-2">
        <div class="min-w-0 space-y-1">
          {#if title}<Card.Title>{title}</Card.Title>{/if}
          {#if description}<Card.Description>{description}</Card.Description>{/if}
        </div>
        {#if actions}
          <div class="flex shrink-0 flex-wrap items-center gap-2">{@render actions()}</div>
        {/if}
      </div>
    </Card.Header>
  {/if}
  <Card.Content class={cn(contentClass)}>
    {@render children?.()}
  </Card.Content>
</Card.Root>
