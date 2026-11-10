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
	import { entities, issueCategories, setupItemTiers } from '$lib/shared/stores/metadata';
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
	import GroupHeading from '$lib/shared/components/data/table/GroupHeading.svelte';
	import PaginationBar from '$lib/shared/components/data/controls/PaginationBar.svelte';
	import { paginationView } from '$lib/shared/components/data/controls/pagination';
	import { computeGroupOffsets } from '$lib/shared/components/data/controls/grouping';
	import { listPageState } from '$lib/shared/components/data/listPage.svelte';
	import {
		groupPageSlice,
		PAGE_SIZE_OPTIONS,
		type PageSizeOption
	} from '$lib/shared/components/data/types';
	import { useIssuesListQuery } from '../queries';
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
		issueCategoryOf,
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
	const daemonsQuery = useDaemonsQuery();
	const credentialsQuery = useCredentialsQuery();
	const sessionsQuery = useActiveSessionsQuery();
	// With Issues on more than one site, a chip per site narrows the list to the sites picked.
	// Every site starts picked; a site whose Issues all clear drops out of the chips. The last
	// picked chip stays on, so the list always names a site to show.
	let unpickedSites = new SvelteSet<string>();
	let pickedSites = $derived(new Set(issueSites.filter((id) => !unpickedSites.has(id))));
	const siteColor = entities.getColorHelper('Site').color;
	const SiteIcon = entities.getIconComponent('Site');

	// The Issues are paged and grouped by category on the server, like every entity list.
	const issuesPage = listPageState(() => isActive && issueCount > 0, PAGE_SIZE_OPTIONS[0]);
	let siteQuery = $derived.by(() => {
		// eslint-disable-next-line svelte/prefer-svelte-reactivity -- rebuilt on every change, never mutated
		const query = new URLSearchParams();
		if (pickedSites.size < issueSites.length) {
			for (const id of pickedSites) query.append('site_ids', id);
		}
		return query;
	});
	const issuesQuery = useIssuesListQuery(() => ({ ...issuesPage.params(), list: siteQuery }));
	let issues = $derived(issuesQuery.data?.items ?? []);
	let pagination = $derived(issuesQuery.data?.pagination ?? null);
	let pageSize = $derived(issuesPage.params().limit as PageSizeOption);
	let page = $derived(paginationView(pagination, pageSize, issues.length));
	let groupOffsets = $derived(computeGroupOffsets(pagination?.group_counts ?? null));
	// The page's rows by category, in the order the server listed them, which is category order.
	let issueGroups = $derived.by(() => {
		const groups: { category: string; entries: SetupListEntry[] }[] = [];
		for (const entry of issues) {
			const category = issueCategoryOf(entry.item);
			const last = groups.at(-1);
			if (last?.category === category) last.entries.push(entry);
			else groups.push({ category, entries: [entry] });
		}
		return groups;
	});
	let collapsedCategories = new SvelteSet<string>();
	function toggleCategory(category: string) {
		if (collapsedCategories.has(category)) collapsedCategories.delete(category);
		else collapsedCategories.add(category);
	}
	function goToPage(next: number, size: PageSizeOption = pageSize) {
		issuesPage.onPageChange(next, size);
	}

	function toggleSite(id: string) {
		if (unpickedSites.has(id)) unpickedSites.delete(id);
		else if (pickedSites.size > 1) unpickedSites.add(id);
		goToPage(1);
	}
	let open = $derived(openEntries(entries));
	let locked = $derived(lockedEntries(entries));
	let done = $derived(doneEntries(entries));
	let progress = $derived(setupProgress(entries));
	let showCompleted = $state(false);
	let issueColor = $derived(setupItemTiers.getColorHelper('Issue').rgb);
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);

	// The hosts the rows name in their chips: an uncovered integration's first host, and the
	// hosts a scan warning concerns.
	let hostIds = $derived([
		...new Set(
			issues.flatMap(({ item }) =>
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
		for (const entry of [...issues, ...entries]) {
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
	{#if issueCount > 0}
		<div>
			<div class="mb-3 flex flex-wrap items-center gap-x-4 gap-y-2">
				<!-- The badge counts every Issue, as the Home badge in the sidebar does. -->
				<h3 class="flex items-center gap-2 text-base font-semibold text-primary">
					<span>{setupItemTiers.getName('Issue')}</span>
					<NavBadge color={issueColor} count={issueCount} />
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
			<div class="space-y-4">
				{#each issueGroups as group (group.category)}
					{@const position = groupOffsets.get(group.category)}
					{@const collapsed = collapsedCategories.has(group.category)}
					<div class="space-y-2">
						<GroupHeading
							name={issueCategories.getName(group.category)}
							count={position?.count ?? group.entries.length}
							range={position
								? groupPageSlice(position, pagination?.offset ?? 0, issues.length)
								: null}
							{collapsed}
							onToggle={() => toggleCategory(group.category)}
						/>
						{#if !collapsed}
							<ul class="space-y-2">
								{#each group.entries as entry (entryId(entry))}
									{@render row(entry)}
								{/each}
							</ul>
						{/if}
					</div>
				{/each}
			</div>
			{#if page.totalCount > PAGE_SIZE_OPTIONS[0]}
				<div class="mt-3">
					<PaginationBar
						totalCount={page.totalCount}
						totalPages={page.totalPages}
						{pageSize}
						showingStart={page.showingStart}
						showingEnd={page.showingEnd}
						canGoPrev={page.canGoPrev}
						canGoNext={page.canGoNext}
						groupCount={pagination?.group_counts?.length ?? null}
						onPrevPage={() => goToPage(page.currentPage - 1)}
						onNextPage={() => goToPage(page.currentPage + 1)}
						onPageSizeChange={(size) => goToPage(1, size)}
					/>
				</div>
			{/if}
		</div>
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
<DaemonTroubleshootingModal
	isOpen={showTroubleshooting}
	onClose={() => (showTroubleshooting = false)}
/>
