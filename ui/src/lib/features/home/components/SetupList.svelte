<script lang="ts" module>
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
	import { setupItemTiers } from '$lib/shared/stores/metadata';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import {
		home_setup_hideCompleted,
		home_setup_progress,
		home_setup_showCompleted
	} from '$lib/paraglide/messages';
	import SetupListRow from './SetupListRow.svelte';
	import IssuesTable from './IssuesTable.svelte';
	import ReferralSourceModal from './ReferralSourceModal.svelte';
	import ProfileModal from './ProfileModal.svelte';
	import { reportShown, setupLookups } from '../setup-lookups.svelte';
	import {
		doneEntries,
		entryId,
		lockedEntries,
		openEntries,
		setupProgress,
		type SetupItem,
		type SetupItemType,
		type SetupListEntry
	} from '../setup-list';

	let {
		entries,
		issueCount,
		issueSites,
		isActive,
		setup
	}: {
		/** The Setup steps the server computed for this user. */
		entries: SetupListEntry[];
		/** The Issues the user sees, on every site. */
		issueCount: number;
		/** The sites those Issues are on. */
		issueSites: string[];
		/** Whether Home is on screen, which holds the Issues page until it is. */
		isActive: boolean;
		/** The org's done Setup item types. */
		setup: SetupItemType[];
	} = $props();

	const queryClient = useQueryClient();
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	const sessionsQuery = useActiveSessionsQuery();
	let open = $derived(openEntries(entries));
	let locked = $derived(lockedEntries(entries));
	let done = $derived(doneEntries(entries));
	let progress = $derived(setupProgress(entries));
	let showCompleted = $state(false);
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	const lookups = setupLookups(() => entries);

	/**
	 * The running scan a RunFirstScan item shows: its daemon's, or else any network scan in the
	 * org, such as one started on another daemon. A scan already running needs no prompt to run one.
	 */
	function firstScanSession(item: SetupItem) {
		if (item.type !== 'RunFirstScan') return null;
		const scans = (sessionsQuery.data ?? []).filter(
			(s) => s.discovery_type?.type === 'Network' || s.discovery_type?.type === 'Unified'
		);
		return scans.find((s) => s.daemon_id === item.daemon_id) ?? scans[0] ?? null;
	}

	let answering = $state<'AnswerReferralSource' | 'AnswerProfile' | null>(null);

	onMount(clearRetiredStorageKeys);

	// Open first steps (what the trial sweep reports) report once per item per page load. The
	// Issues table reports its own rows.
	$effect(() => reportShown(entries));

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

{#snippet row(entry: SetupListEntry)}
	<SetupListRow
		{entry}
		{setup}
		{lookups}
		session={firstScanSession(entry.item)}
		{hasEmail}
		onAnswer={(type) => (answering = type)}
		onTroubleshoot={() => {
			// Only a daemon Issue troubleshoots, and the Issues table opens that help itself.
		}}
	/>
{/snippet}

<section class="space-y-6">
	{#if issueCount > 0}
		<IssuesTable {issueSites} {isActive} {setup} />
	{/if}
	<!-- Locked items show disabled after the open ones. Once every item is done, the section goes. -->
	{#if progress.done < progress.total}
		<div>
			<div class="mb-3 flex items-center gap-3">
				<h3 class="text-base font-semibold text-primary">{setupItemTiers.getName('Setup')}</h3>
				<ProgressTrack progress={(progress.done / progress.total) * 100} size="sm" class="w-32" />
				<span class="text-sm text-tertiary">
					{home_setup_progress({ done: progress.done, total: progress.total })}
				</span>
			</div>
			<ul class="space-y-2">
				{#each open as entry (entryId(entry))}
					{@render row(entry)}
				{/each}
				{#each locked as entry (entryId(entry))}
					{@render row(entry)}
				{/each}
				{#if showCompleted}
					{#each done as entry (entryId(entry))}
						{@render row(entry)}
					{/each}
				{/if}
			</ul>
			{#if done.length > 0}
				<button
					type="button"
					class="mt-2 text-sm text-link hover:underline"
					onclick={() => (showCompleted = !showCompleted)}
				>
					{showCompleted
						? home_setup_hideCompleted()
						: home_setup_showCompleted({ count: done.length })}
				</button>
			{/if}
		</div>
	{/if}
</section>

<ReferralSourceModal
	isOpen={answering === 'AnswerReferralSource'}
	onClose={() => (answering = null)}
/>
<ProfileModal isOpen={answering === 'AnswerProfile'} onClose={() => (answering = null)} />
