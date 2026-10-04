<script lang="ts">
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { hostOsIcon, hostOsLabel, hostOsTooltip, type HostOs } from '$lib/features/hosts/host-os';
	import type { AttributeSource } from '$lib/shared/utils/attribute-source';

	/**
	 * An operating system as a gray tag with its family's icon: the daemon OS icon, the vendor's
	 * logo, or a glyph. The tooltip lists what the source read and, given a source, how the value
	 * reached Scanopy.
	 */
	let { os, source = null }: { os: HostOs; source?: AttributeSource | null } = $props();
</script>

<Tag
	icon={hostOsIcon(os.family)}
	label={hostOsLabel(os)}
	color="Gray"
	title={hostOsTooltip(os, source)}
/>
