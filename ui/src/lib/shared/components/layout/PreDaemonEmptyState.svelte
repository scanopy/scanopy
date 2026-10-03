<script lang="ts">
	import EmptyState from './EmptyState.svelte';
	import { Server } from 'lucide-svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { useCurrentUserQuery } from '$lib/features/auth/queries';
	import {
		daemons_installPromptViewer,
		gettingStarted_stepDaemonLabel
	} from '$lib/paraglide/messages';

	let { title, isReadOnly = false }: { title: string; isReadOnly?: boolean } = $props();

	const currentUserQuery = useCurrentUserQuery();
	let isViewer = $derived(currentUserQuery.data?.permissions === 'Viewer');

	function handleClick() {
		window.location.hash = 'daemons';
		openModal('create-daemon');
	}
</script>

<EmptyState
	IconComponent={Server as IconComponent}
	{title}
	subtitle={isReadOnly && isViewer ? daemons_installPromptViewer() : ''}
	onClick={handleClick}
	cta={isReadOnly ? '' : gettingStarted_stepDaemonLabel()}
/>
