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
	import { SvelteSet } from 'svelte/reactivity';
	import { entities, setupItemTiers } from '$lib/shared/stores/metadata';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import { useConfigQuery } from '$lib/shared/stores/config-query';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { useHostsByIds } from '$lib/features/hosts/queries';
	import { useSitesQuery } from '$lib/features/sites/queries';
	import { hostIdsOf } from '$lib/features/discovery/utils/warnings';
	import { useCredentialsQuery } from '$lib/features/credentials/queries';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import NavBadge from '$lib/shared/components/layout/NavBadge.svelte';
	import ProgressTrack from '$lib/shared/components/data/ProgressTrack.svelte';
	import {
		home_setup_hideCompleted,
		home_setup_progress,
		home_setup_showCompleted
	} from '$lib/paraglide/messages';
	import SetupListRow, { type SetupLookups } from './SetupListRow.svelte';
	import ReferralSourceModal from './ReferralSourceModal.svelte';
	import ProfileModal from './ProfileModal.svelte';
	import {
		doneEntries,
		entryId,
		issueEntries,
		issueSiteIds,
		issuesOnSites,
		openEntries,
		setupProgress,
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
		/** The org's done Setup item types. */
		setup: SetupItemType[];
	} = $props();

	const queryClient = useQueryClient();
	const configQuery = useConfigQuery();
	const organizationQuery = useOrganizationQuery();
	const daemonsQuery = useDaemonsQuery();
	const credentialsQuery = useCredentialsQuery();
	const sessionsQuery = useActiveSessionsQuery();
	let allIssues = $derived(issueEntries(entries));
	// With Issues on more than one site, a chip per site narrows the list to the sites picked.
	// Every site starts picked; a site whose Issues all clear drops out of the chips.
	let issueSites = $derived(issueSiteIds(allIssues));
	let unpickedSites = new SvelteSet<string>();
	let pickedSites = $derived(new Set(issueSites.filter((id) => !unpickedSites.has(id))));
	let issues = $derived(issueSites.length > 1 ? issuesOnSites(allIssues, pickedSites) : allIssues);
	const siteColor = entities.getColorHelper('Site').color;
	const SiteIcon = entities.getIconComponent('Site');
	function toggleSite(id: string) {
		if (unpickedSites.has(id)) unpickedSites.delete(id);
		else unpickedSites.add(id);
	}
	let open = $derived(openEntries(entries));
	let done = $derived(doneEntries(entries));
	let progress = $derived(setupProgress(entries));
	let showCompleted = $state(false);
	let issueColor = $derived(setupItemTiers.getColorHelper('Issue').rgb);
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	// The hosts the rows name in their chips: an uncovered integration's first host, and the
	// hosts a scan warning concerns.
	let hostIds = $derived([
		...new Set(
			entries.flatMap(({ item }) =>
				item.type === 'UncoveredIntegration'
					? item.host_ids.slice(0, 1)
					: item.type === 'RunWarnings'
						? hostIdsOf(item)
						: []
			)
		)
	]);
	const hostsQuery = useHostsByIds(() => hostIds);
	const sitesQuery = useSitesQuery();

	let lookups = $derived<SetupLookups>({
		daemon: (id) => (daemonsQuery.data ?? []).find((d) => d.id === id),
		host: (id) => (hostsQuery.data ?? []).find((h) => h.id === id),
		site: (id) => (sitesQuery.data ?? []).find((s) => s.id === id),
		credential: (id) => (credentialsQuery.data ?? []).find((c) => c.id === id)
	});

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
	let showTroubleshooting = $state(false);

	onMount(clearRetiredStorageKeys);

	// Issues and Open first steps (what the trial sweep reports) report once per item per page load.
	$effect(() => {
		for (const entry of entries) {
			if (!entry.alerts_when_stuck) continue;
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

{#snippet row(entry: SetupListEntry)}
	<SetupListRow
		{entry}
		{setup}
		{lookups}
		session={firstScanSession(entry.item)}
		{hasEmail}
		onAnswer={(type) => (answering = type)}
		onTroubleshoot={() => (showTroubleshooting = true)}
	/>
{/snippet}

<section class="space-y-6">
	{#if allIssues.length > 0}
		<div>
			<div class="mb-3 flex flex-wrap items-center gap-x-4 gap-y-2">
				<!-- The badge counts every Issue, as the Home badge in the sidebar does. -->
				<h3 class="flex items-center gap-2 text-base font-semibold text-primary">
					<span>{setupItemTiers.getName('Issue')}</span>
					<NavBadge color={issueColor} count={allIssues.length} />
				</h3>
				{#if issueSites.length > 1}
					<div class="flex flex-wrap items-center gap-1.5">
						{#each issueSites as siteId (siteId)}
							{@const picked = pickedSites.has(siteId)}
							<button
								type="button"
								onclick={() => toggleSite(siteId)}
								aria-pressed={picked}
								class="transition-opacity {picked ? 'opacity-100' : 'opacity-50 hover:opacity-75'}"
							>
								<Tag
									label={(sitesQuery.data ?? []).find((s) => s.id === siteId)?.name ?? ''}
									color={siteColor}
									icon={SiteIcon}
								/>
							</button>
						{/each}
					</div>
				{/if}
			</div>
			<ul class="space-y-2">
				{#each issues as entry (entryId(entry))}
					{@render row(entry)}
				{/each}
			</ul>
		</div>
	{/if}
	<!-- Locked items count in the total and never show. Once every item is done, the section goes. -->
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
<DaemonTroubleshootingModal
	isOpen={showTroubleshooting}
	onClose={() => (showTroubleshooting = false)}
/>
