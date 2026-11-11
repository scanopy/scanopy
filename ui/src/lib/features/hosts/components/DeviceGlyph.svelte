<script lang="ts">
	import type { components } from '#lib/api/schema';
	import { deviceTypes } from '#lib/shared/stores/metadata.ts';

	/**
	 * A device: a neutral square tile holding its device type's glyph. The square is the shape the
	 * topology gives every entity whose `glyph` metadata is `Device`.
	 */
	let {
		deviceType,
		size = 'sm'
	}: {
		deviceType: components['schemas']['DeviceType'];
		/** `sm` beside text and in a container header, `md` in a list or card's icon slot, `lg` on an icon card. */
		size?: 'sm' | 'md' | 'lg';
	} = $props();

	let Glyph = $derived(deviceTypes.getIconComponent(deviceType));
	let tile = $derived(
		size === 'lg'
			? 'h-12 w-12 rounded-lg border-2'
			: size === 'md'
				? 'h-7 w-7 rounded-md border'
				: 'h-5 w-5 rounded-sm border'
	);
	let glyphSize = $derived(size === 'lg' ? 'h-7 w-7' : size === 'md' ? 'h-4 w-4' : 'h-3.5 w-3.5');
</script>

<!-- One uniform neutral tone for fill and border, the glyph in the text colour: the shape tells a
     device from software and the glyph reads first, in either theme. -->
<span
	class="inline-flex shrink-0 items-center justify-center border-gray-200 bg-gray-200 dark:border-gray-700 dark:bg-gray-700 {tile}"
>
	<Glyph class="text-primary {glyphSize}" />
</span>
