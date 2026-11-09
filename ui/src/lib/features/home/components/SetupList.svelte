<script lang="ts" module>
	/** Keys `setup_issue_shown` has fired for since the page loaded. */
	const shownKeys = new Set<string>();

	/** Local keys of the checklist and nudges this list replaced, cleared once per page load. */
	const RETIRED_STORAGE_PREFIXES = ['nudge-dismissed:', 'waiting-fun:'];
	const RETIRED_STORAGE_KEYS = ['home-checklist-dismissed'];
	let retiredKeysCleared = false;

	function clearRetiredStorageKeys() {
		if (retiredKeysCleared) return;
		retiredKeysCleared = true;
		try {
			for (let i = localStorage.length - 1; i >= 0; i--) {
				const key = localStorage.key(i);
				if (
					key &&
					(RETIRED_STORAGE_KEYS.includes(key) ||
						RETIRED_STORAGE_PREFIXES.some((prefix) => key.startsWith(prefix)))
				) {
					localStorage.removeItem(key);
				}
			}
		} catch {
			// Storage blocked: there is nothing to clear.
		}
	}
</script>

<script lang="ts">
	import { onMount } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import DaemonTroubleshootingModal from '$lib/shared/components/layout/DaemonTroubleshootingModal.svelte';
	import { setupItemTiers } from '$lib/shared/stores/metadata';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import {
		useDismissSetupSuggestionMutation,
		useOrganizationQuery
	} from '$lib/features/organizations/queries';
	import SetupListRow, { type SetupLookups } from './SetupListRow.svelte';
	import ReferralSourceModal from './ReferralSourceModal.svelte';
	import ProfileModal from './ProfileModal.svelte';
	import {
		entryId,
		groupByTier,
		type SetupItem,
		type SetupItemType,
		type SetupListEntry
	} from '../setup-list';

	let {
		entries,
		setup
	}: {
		/** The Setup list the server computed for this user. */
		entries: SetupListEntry[];
		/** The org's finished and dismissed item types. */
		setup: SetupItemType[];
	} = $props();

	const queryClient = useQueryClient();
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	const daemonsQuery = useDaemonsQuery();
	const sitesQuery = useSitesQuery();
	const credentialsQuery = useCredentialsQuery();
	const sessionsQuery = useActiveSessionsQuery();
	const dismissMutation = useDismissSetupSuggestionMutation();

	let groups = $derived(groupByTier(entries));
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	// The hosts an uncovered integration was found on, for its chip's name.
	let hostIds = $derived(
		entries.flatMap(({ item }) =>
			item.type === 'UncoveredIntegration' ? item.host_ids.slice(0, 1) : []
		)
	);
	const hostsQuery = useHostsByIds(() => hostIds);

	let lookups = $derived<SetupLookups>({
		daemon: (id) => (daemonsQuery.data ?? []).find((d) => d.id === id),
		site: (id) => (sitesQuery.data ?? []).find((s) => s.id === id),
		host: (id) => (hostsQuery.data ?? []).find((h) => h.id === id),
		credential: (id) => (credentialsQuery.data ?? []).find((c) => c.id === id)
	});

	/** The running first scan of the daemon a RunFirstScan item names. */
	function firstScanSession(item: SetupItem) {
		if (item.type !== 'RunFirstScan') return null;
		return (
			(sessionsQuery.data ?? []).find(
				(s) =>
					s.daemon_id === item.daemon_id &&
					(s.discovery_type?.type === 'Network' || s.discovery_type?.type === 'Unified')
			) ?? null
		);
	}

	let answering = $state<'AnswerReferralSource' | 'AnswerProfile' | null>(null);
	let showTroubleshooting = $state(false);

	function dismiss(itemType: SetupItemType) {
		dismissMutation.mutate(itemType);
	}

	onMount(clearRetiredStorageKeys);

	// Issues and Next steps report once per item per page load.
	$effect(() => {
		for (const entry of entries) {
			if (entry.tier === 'Suggestion') continue;
			const key = entryId(entry);
			if (shownKeys.has(key)) continue;
			shownKeys.add(key);
			trackEvent('setup_issue_shown', { item_type: entry.item.type, tier: entry.tier, key });
		}
	});

	// While the create-daemon wizard waits for a daemon, poll until the org records its first
	// connection, then tell the wizard and refresh the Daemons list, which still holds it unseen.
	let daemonStatus = $state<'idle' | 'waiting' | 'connected' | 'trouble'>('idle');
	onMount(() =>
		daemonSetupState.subscribe((s) => {
			daemonStatus = s.connectionStatus;
		})
	);
	let waitingForDaemon = $derived(daemonStatus === 'waiting' || daemonStatus === 'trouble');

	$effect(() => {
		if (!waitingForDaemon) return;
		const interval = setInterval(() => {
			organizationQuery.refetch();
			queryClient.invalidateQueries({ queryKey: queryKeys.dashboard.all });
		}, 5000);
		return () => clearInterval(interval);
	});

	$effect(() => {
		if (waitingForDaemon && setup.includes('InstallDaemon')) {
			daemonSetupState.set({ connectionStatus: 'connected' });
			queryClient.invalidateQueries({ queryKey: queryKeys.daemons.all });
		}
	});
</script>

<section class="space-y-6">
	{#each groups as group (group.tier)}
		<div>
			<h3 class="mb-3 flex items-baseline gap-2 text-base font-semibold text-primary">
				<span>{setupItemTiers.getName(group.tier)}</span>
				<span class="text-sm font-normal text-tertiary">({group.entries.length})</span>
			</h3>
			<ul class="space-y-2">
				{#each group.entries as entry (entryId(entry))}
					<SetupListRow
						{entry}
						{setup}
						{lookups}
						session={firstScanSession(entry.item)}
						{hasEmail}
						onDismiss={dismiss}
						onAnswer={(type) => (answering = type)}
						onTroubleshoot={() => (showTroubleshooting = true)}
					/>
				{/each}
			</ul>
		</div>
	{/each}
</section>

<ReferralSourceModal
	isOpen={answering === 'AnswerReferralSource'}
	onClose={() => (answering = null)}
/>
<ProfileModal isOpen={answering === 'AnswerProfile'} onClose={() => (answering = null)} />
<DaemonTroubleshootingModal
	isOpen={showTroubleshooting}
	onClose={() => (showTroubleshooting = false)}
/>
