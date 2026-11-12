<script lang="ts">
	import { createColorHelper, type Color } from '#lib/shared/utils/styling.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';

	export let Icon: IconComponent; // Expects Svelte component, not string
	export let color: Color;

	let colorHelper = createColorHelper(color);
</script>

<div class="rounded-lg p-2">
	<Icon size={24} class={colorHelper.icon} />
</div>
