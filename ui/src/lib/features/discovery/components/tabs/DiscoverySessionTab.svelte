<script lang="ts">
	import TabHeader from '$lib/shared/components/layout/TabHeader.svelte';
	import EmptyState from '$lib/shared/components/layout/EmptyState.svelte';
	import PreDaemonEmptyState from '$lib/shared/components/layout/PreDaemonEmptyState.svelte';
	import { hasDaemon } from '$lib/shared/onboarding/checklist';
	import type { FieldConfig } from '$lib/shared/components/data/types';
	import DataControls from '$lib/shared/components/data/DataControls.svelte';
	import { useActiveSessionsQuery, useCancelDiscoveryMutation } from '../../queries';
	import DiscoverySessionCard from '../cards/DiscoverySessionCard.svelte';
	import { type DiscoveryUpdatePayload } from '../../types/api';
	import { formatTimestamp } from '$lib/shared/utils/formatting';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import type { TabProps } from '$lib/shared/types';
	import type { components } from '$lib/api/schema';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import {
		common_daemon,
		common_name,
		common_phase,
		discovery_activeSessionsTitle,
		discovery_discoveryType,
		discovery_finishedAt,
		discovery_noActiveSessions,
		discovery_noActiveSessionsSubtitle,
		discovery_notStarted,
		discovery_startedAt,
		discovery_unknownDaemon,
		common_firstDiscoveryEmailHint
	} from '$lib/paraglide/messages';

	let { isReadOnly = false }: TabProps = $props();

	type OnboardingOperation = components['schemas']['OnboardingOperation'];

	// Queries
	const daemonsQuery = useDaemonsQuery();
	const sessionsQuery = useActiveSessionsQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();

	// Email hint visibility
	let onboarding = $derived((organizationQuery.data?.onboarding ?? []) as OnboardingOperation[]);
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	let hasCompletedFirstDiscovery = $derived(onboarding.includes('FirstDiscoveryCompleted'));
	let showEmailHint = $derived(hasEmail && !hasCompletedFirstDiscovery);

	// Mutations
	const cancelDiscoveryMutation = useCancelDiscoveryMutation();

	// Derived data
	let daemonsData = $derived(daemonsQuery.data ?? []);
	let sessionsList = $derived(sessionsQuery.data ?? []);
	let isLoading = $derived(daemonsQuery.isPending || sessionsQuery.isPending);

	function handleCancelDiscovery(sessionId: string) {
		cancelDiscoveryMutation.mutate(sessionId);
	}

	let discoveryFields = $derived.by((): FieldConfig<DiscoveryUpdatePayload>[] => [
		{
			key: 'name',
			label: common_name(),
			type: 'string',
			searchable: true
		},
		{
			key: 'discovery_type',
			label: discovery_discoveryType(),
			type: 'string',
			searchable: true,
			filterable: true,
			getValue: (item) => item.discovery_type.type
		},
		{
			key: 'daemon',
			label: common_daemon(),
			type: 'string',
			searchable: true,
			filterable: true,
			getValue: (item) => {
				const daemon = daemonsData.find((d) => d.id == item.daemon_id);
				return daemon ? daemon.name : discovery_unknownDaemon();
			}
		},
		{
			key: 'phase',
			label: common_phase(),
			type: 'string',
			searchable: true,
			filterable: true
		},
		{
			key: 'started_at',
			label: discovery_startedAt(),
			type: 'string',
			searchable: true,
			getValue: (item) =>
				item.started_at ? formatTimestamp(item.started_at) : discovery_notStarted()
		},
		{
			key: 'finished_at',
			label: discovery_finishedAt(),
			type: 'string',
			searchable: true,
			getValue: (item) =>
				item.finished_at ? formatTimestamp(item.finished_at) : discovery_notStarted()
		}
	]);
</script>

<div class="space-y-6">
	<!-- Header -->
	<TabHeader title={discovery_activeSessionsTitle()} />
	{#if showEmailHint}
		<InlineInfo title={common_firstDiscoveryEmailHint()} dismissableKey="discovery-email-hint" />
	{/if}
	{#if !hasDaemon(onboarding)}
		<PreDaemonEmptyState title="Install a daemon to start running discoveries on your network." />
	{:else if isLoading}
		<Loading />
	{:else if sessionsList.length === 0}
		<!-- Empty state -->
		<EmptyState
			title={discovery_noActiveSessions()}
			subtitle={discovery_noActiveSessionsSubtitle()}
		/>
	{:else}
		<DataControls
			items={sessionsList}
			fields={discoveryFields}
			storageKey="scanopy-discovery-session-table-state"
			getItemId={(item) => item.session_id}
		>
			{#snippet children(item: DiscoveryUpdatePayload, viewMode: 'card' | 'list')}
				<DiscoverySessionCard
					session={item}
					{viewMode}
					onCancel={isReadOnly ? undefined : handleCancelDiscovery}
				/>
			{/snippet}
		</DataControls>
	{/if}
</div>
