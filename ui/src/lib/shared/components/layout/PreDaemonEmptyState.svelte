<script lang="ts">
	import EmptyState from './EmptyState.svelte';
	import { Server } from '@lucide/svelte';
	import { openModal } from '#lib/shared/stores/modal-registry.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { isPlanLapsed } from '#lib/features/organizations/types.ts';
	import {
		daemons_installPromptViewer,
		gettingStarted_stepDaemonLabel
	} from '#lib/paraglide/messages.js';

	let { title }: { title: string } = $props();

	const can = useCan();
	const organizationQuery = useOrganizationQuery();
	let canInstall = $derived(can('Daemon', 'Write'));
	// A lapsed org can't install either, and its banner already says why.
	let isLapsed = $derived(organizationQuery.data != null && isPlanLapsed(organizationQuery.data));

	function handleClick() {
		window.location.hash = 'daemons';
		openModal('create-daemon');
	}
</script>

<EmptyState
	IconComponent={Server as IconComponent}
	{title}
	subtitle={!canInstall && !isLapsed ? daemons_installPromptViewer() : ''}
	onClick={handleClick}
	cta={canInstall ? gettingStarted_stepDaemonLabel() : ''}
/>
