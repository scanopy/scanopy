<script lang="ts">
	import type { DaemonOS } from '../utils';
	import { hostOsIcon, hostOsOfDaemonOs } from '#lib/features/hosts/host-os.ts';
	import { createIconComponent } from '#lib/shared/utils/styling.ts';

	interface Props {
		os: DaemonOS;
		class?: string;
		/** Pixel width and height, as Lucide icons take it; a `class` sizing still overrides it. */
		size?: number;
	}

	let { os, class: className = '', size = 16 }: Props = $props();

	// A daemon draws the logo of the OS family it runs on, the same one a host's OS tag draws.
	let family = $derived(hostOsOfDaemonOs(os)?.family);
	let Icon = $derived(family ? hostOsIcon(family) : createIconComponent(null));
	// Bare, without the round frame: the OS sits beside its name in a selector or a row. Spread, as
	// the icon type is lucide's and does not declare `framed`.
	const bare = { framed: false };
</script>

<Icon {size} {...bare} class={className} />
