<script lang="ts">
	import { CloudOff } from '@lucide/svelte';
	import EmptyState from './EmptyState.svelte';
	import {
		common_loadFailedSubtitle,
		common_loadFailedTitle,
		common_tryAgain
	} from '#lib/paraglide/messages.js';

	// A page whose data failed to load. It stands in for the page's empty state, which would
	// tell the user there is nothing there. The page refetches on its own once the server answers
	// again (server-reachability.ts); Try again is for a failure that wasn't an outage.
	let {
		title = common_loadFailedTitle(),
		subtitle = common_loadFailedSubtitle(),
		onRetry
	}: {
		title?: string;
		subtitle?: string;
		onRetry: () => void;
	} = $props();
</script>

<!-- The button goes in as a child, not as the CTA, so the empty state's C shortcut (which
     presses Create) doesn't land on it. -->
<EmptyState IconComponent={CloudOff} {title} {subtitle}>
	<button onclick={onRetry} class="btn-secondary">{common_tryAgain()}</button>
</EmptyState>
