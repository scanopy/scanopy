<script lang="ts">
	import OsIcon from '$lib/features/daemons/components/OsIcon.svelte';
	import { hostOsIcon, hostOsLabel, hostOsTooltip, type HostOs } from '$lib/features/hosts/host-os';
	import type { AttributeSource } from '$lib/shared/utils/attribute-source';
	import { createColorHelper } from '$lib/shared/utils/styling';

	/**
	 * An operating system as a gray tag with its family's icon: the daemon OS icon, the vendor's
	 * logo, or a glyph. The tooltip lists what the source read and, given a source, how the value
	 * reached Scanopy.
	 */
	let { os, source = null }: { os: HostOs; source?: AttributeSource | null } = $props();

	const gray = createColorHelper('Gray');
	let icon = $derived(hostOsIcon(os.family));
</script>

<span
	class="inline-flex items-center gap-1 rounded px-2 py-0.5 text-xs font-medium {gray.bg} {gray.text}"
	title={hostOsTooltip(os, source)}
>
	{#if icon.kind === 'daemonOs'}
		<OsIcon os={icon.os} class="h-4 w-4 flex-shrink-0" />
	{:else}
		<icon.component class="h-4 w-4 flex-shrink-0" />
	{/if}
	<span class="truncate">{hostOsLabel(os)}</span>
</span>
