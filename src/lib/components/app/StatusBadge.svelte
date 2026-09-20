<script>
  /**
   * Tone-driven status chip.
   * One component for every badge in the app so colour/typography never drift.
   */
  import { badgeTone } from '$lib/variants.js';
  import { cn } from '$lib/utils.js';

  let {
    tone = 'neutral',
    class: className,
    children,
    ...restProps
  } = $props();
</script>

<span class={cn(badgeTone({ tone }), className)} {...restProps}>
  {@render children?.()}
</span>
