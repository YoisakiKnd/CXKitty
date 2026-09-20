<script>
	import { Tabs as TabsPrimitive } from "bits-ui";
	import { cn } from "$lib/utils.js";

	let {
		ref = $bindable(null),
		class: className,
		...restProps
	} = $props();
</script>

<TabsPrimitive.Content
	bind:ref
	data-slot="tabs-content"
	class={cn(
		"text-xs/relaxed focus-visible:ring-ring/30 flex-1 outline-none focus-visible:ring-2",
		className
	)}
	{...restProps}
/>