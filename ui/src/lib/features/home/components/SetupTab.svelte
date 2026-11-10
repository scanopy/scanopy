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
	import { onMount, untrack } from 'svelte';
	import { useQueryClient } from '@tanstack/svelte-query';
	import { queryKeys } from '$lib/api/query-client';
	import Loading from '$lib/shared/components/feedback/Loading.svelte';
	import LoadFailedState from '$lib/shared/components/layout/LoadFailedState.svelte';
	import PageTitle from '$lib/shared/components/layout/PageTitle.svelte';
	import { loadFailure } from '$lib/shared/utils/load-failure';
	import { setupItemTiers } from '$lib/shared/stores/metadata';
	import { useConfigQuery, isCloud } from '$lib/shared/stores/config-query';
	import type { TabProps } from '$lib/shared/types';
	import { daemonSetupState } from '$lib/features/daemons/stores/daemon-setup';
	import { useActiveSessionsQuery } from '$lib/features/discovery/queries';
	import { useOrganizationQuery } from '$lib/features/organizations/queries';
	import { isPlanLapsed } from '$lib/features/organizations/types';
	import { useDashboardQuery } from '$lib/features/home/queries';
	import {
		common_dismiss,
		home_demoEmbedSubtitle,
		home_demoEmbedTitle,
		home_demoNetworkTopology,
		home_setup_hideCompleted,
		home_setup_progress,
		home_setup_showCompleted
	} from '$lib/paraglide/messages';
	import SetupListRow from './SetupListRow.svelte';
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
		type SetupListEntry
	} from '../setup-list';

	let { isActive = false }: TabProps = $props();

	const queryClient = useQueryClient();
	const dashboardQuery = useDashboardQuery();
	const organizationQuery = useOrganizationQuery();
	const configQuery = useConfigQuery();
	const sessionsQuery = useActiveSessionsQuery();

	let organization = $derived(organizationQuery.data);
	let pageLoad = $derived(loadFailure(dashboardQuery, organizationQuery));
	let setup = $derived(organization?.setup ?? []);
	// A lapsed org keeps its plan's feature flags but can act on none of them, so it gets no steps.
	let entries = $derived(
		organization && isPlanLapsed(organization) ? [] : (dashboardQuery.data?.setup_items ?? [])
	);
	let open = $derived(openEntries(entries));
	let locked = $derived(lockedEntries(entries));
	let done = $derived(doneEntries(entries));
	let progress = $derived(setupProgress(entries));
	let showCompleted = $state(false);
	let hasEmail = $derived(configQuery.data?.has_email_service ?? false);
	const lookups = setupLookups(() => entries);

	let answering = $state<'AnswerReferralSource' | 'AnswerProfile' | null>(null);

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

	let demoTopologyDismissed = $state(false);
	onMount(() => {
		clearRetiredStorageKeys();
		try {
			demoTopologyDismissed = localStorage.getItem('home-demo-topology-dismissed') === 'true';
		} catch {
			// Storage blocked: the embed shows.
		}
	});

	function dismissDemoTopology() {
		demoTopologyDismissed = true;
		try {
			localStorage.setItem('home-demo-topology-dismissed', 'true');
		} catch {
			// Storage blocked: the embed comes back on the next load.
		}
	}

	// Refetch when the tab opens. Untracked: reading `refetch` off a query subscribes to every
	// change of its result, and each refetch changes it, so the effect would refetch in a loop.
	$effect(() => {
		if (isActive) {
			untrack(() => {
				organizationQuery.refetch();
				dashboardQuery.refetch();
			});
		}
	});

	// Open first steps (what the trial sweep reports) report once per item per page load.
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
			// Only a daemon Issue troubleshoots, and the Issues tab opens that help itself.
		}}
	/>
{/snippet}

<div class="space-y-6">
	<PageTitle
		title={setupItemTiers.getName('Setup')}
		aside={progress.total > 0
			? home_setup_progress({ done: progress.done, total: progress.total })
			: null}
	/>

	{#if pageLoad.failed}
		<LoadFailedState onRetry={pageLoad.retry} />
	{:else if dashboardQuery.isPending || organizationQuery.isPending}
		<Loading />
	{:else}
		<!-- Locked steps show disabled after the open ones. -->
		<section>
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
		</section>

		<!-- Demo topology, shown until the first scan finishes. -->
		{#if configQuery.data && isCloud(configQuery.data) && !setup.includes('RunFirstScan') && !demoTopologyDismissed}
			<section>
				<div class="card-static card overflow-hidden p-0!">
					<div class="flex items-center justify-between px-4 pt-3">
						<h3 class="text-base font-semibold text-primary">{home_demoEmbedTitle()}</h3>
						<div class="flex items-center gap-3">
							<a
								href="https://demo.scanopy.net/share/a1b2c3d4-e5f6-7890-abcd-ef1234567890?utm_source=app&utm_medium=in_app&utm_campaign=demo_embed&utm_content=home_link"
								target="_blank"
								rel="noopener noreferrer"
								class="text-sm text-link hover:underline"
							>
								View full screen
							</a>
							<button
								onclick={dismissDemoTopology}
								class="text-sm text-tertiary hover:text-secondary"
							>
								{common_dismiss()}
							</button>
						</div>
					</div>
					<p class="px-4 pb-2 text-sm text-secondary">
						{home_demoEmbedSubtitle()}
					</p>
					<div class="h-[400px] w-full">
						<iframe
							src="https://demo.scanopy.net/share/a1b2c3d4-e5f6-7890-abcd-ef1234567890/embed?utm_source=app&utm_medium=in_app&utm_campaign=demo_embed&utm_content=home_embed"
							width="100%"
							height="100%"
							frameborder="0"
							style="border: none;"
							title={home_demoNetworkTopology()}
						></iframe>
					</div>
				</div>
			</section>
		{/if}
	{/if}
</div>

<ReferralSourceModal
	isOpen={answering === 'AnswerReferralSource'}
	onClose={() => (answering = null)}
/>
<ProfileModal isOpen={answering === 'AnswerProfile'} onClose={() => (answering = null)} />
