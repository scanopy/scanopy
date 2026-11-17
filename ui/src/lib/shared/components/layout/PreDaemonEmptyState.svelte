<script lang="ts">
	import EmptyState from './EmptyState.svelte';
	import DaemonTroubleshootingModal from './DaemonTroubleshootingModal.svelte';
	import { Server } from '@lucide/svelte';
	import { navigateToEntity, openModal } from '#lib/shared/stores/modal-registry.ts';
	import type { IconComponent } from '#lib/shared/utils/types.ts';
	import { useCan } from '#lib/features/auth/access.ts';
	import { useOrganizationQuery } from '#lib/features/organizations/queries.ts';
	import { isPlanLapsed } from '#lib/features/organizations/types.ts';
	import { useDaemonsQuery } from '#lib/features/daemons/queries.ts';
	import { awaitingDaemon } from '#lib/features/daemons/utils.ts';
	import {
		common_openName,
		daemons_awaitingConnectionSubtitle,
		daemons_awaitingConnectionTitle,
		daemons_installPromptViewer,
		gettingStarted_stepDaemonLabel,
		home_setup_troubleshootConnection
	} from '#lib/paraglide/messages.js';

	let { title }: { title: string } = $props();

	const can = useCan();
	const organizationQuery = useOrganizationQuery();
	const daemonsQuery = useDaemonsQuery();
	let canInstall = $derived(can('Daemon', 'Write'));
	// A lapsed org can't install either, and its banner already says why.
	let isLapsed = $derived(organizationQuery.data != null && isPlanLapsed(organizationQuery.data));

	// A daemon created but never started is the one to fix, not a reason to create another.
	let pending = $derived(awaitingDaemon(daemonsQuery.data ?? []));
	let troubleshooting = $state(false);

	function handleClick() {
		window.location.hash = 'daemons';
		openModal('create-daemon');
	}
</script>

{#if pending}
	<EmptyState
		IconComponent={Server as IconComponent}
		title={daemons_awaitingConnectionTitle({ daemon: pending.name })}
		subtitle={daemons_awaitingConnectionSubtitle()}
	>
		<div class="flex flex-wrap justify-center gap-2">
			<button type="button" class="btn-primary" onclick={() => (troubleshooting = true)}>
				{home_setup_troubleshootConnection()}
			</button>
			<button
				type="button"
				class="btn-secondary"
				onclick={() => navigateToEntity('Daemon', pending.id, pending)}
			>
				{common_openName({ name: pending.name })}
			</button>
		</div>
	</EmptyState>
	<DaemonTroubleshootingModal isOpen={troubleshooting} onClose={() => (troubleshooting = false)} />
{:else}
	<EmptyState
		IconComponent={Server as IconComponent}
		{title}
		subtitle={!canInstall && !isLapsed ? daemons_installPromptViewer() : ''}
		onClick={handleClick}
		cta={canInstall ? gettingStarted_stepDaemonLabel() : ''}
	/>
{/if}
