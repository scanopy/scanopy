<script lang="ts" module>
	import type { IconComponent } from '$lib/shared/utils/types';
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	/** A row's fix: the labelled button that opens where the item is fixed. */
	export interface SetupFix {
		label: string;
		run: () => void;
	}

	/** The loaded entities a row names, so a chip opens the record rather than its id alone. */
	export interface SetupLookups {
		daemon: (id: string) => Record<string, unknown> | undefined;
		site: (id: string) => Record<string, unknown> | undefined;
		host: (
			id: string
		) => (Record<string, unknown> & Parameters<typeof hostDisplayName>[0]) | undefined;
		credential: CredentialLookup;
	}

	/** One chip under a row: an entity the item concerns, with `+N` when it stands for several. */
	interface Subject {
		type: EntityDiscriminants;
		id: string;
		label: string;
		data: object;
		badge?: string;
		icon?: IconComponent | null;
	}
</script>

<script lang="ts">
	import { X } from '@lucide/svelte';
	import type { components } from '$lib/api/schema';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import RowActions from '$lib/shared/components/data/table/RowActions.svelte';
	import { entityRef, type RowAction } from '$lib/shared/components/data/types';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
	import SessionProgress from '$lib/features/discovery/components/cards/SessionProgress.svelte';
	import { isFullScan } from '$lib/features/discovery/utils/fullScan';
	import { buildWarningReport } from '$lib/features/discovery/utils/warnings';
	import { actionFor } from '$lib/features/discovery/utils/warningActions';
	import type { CredentialLookup } from '$lib/features/discovery/utils/snmpCheck';
	import {
		integrationWithoutCredentialSentence,
		integrationWithoutCredentialTitle
	} from '$lib/features/discovery/utils/warnings';
	import {
		createIntegrationCredentialLabel,
		openCredentialCreate
	} from '$lib/features/credentials/utils/prefill';
	import {
		activeView,
		showDependencyTutorial,
		showViewSwitcherHint
	} from '$lib/features/topology/queries';
	import { navigateToEntity, openModal } from '$lib/shared/stores/modal-registry';
	import { openTab } from '$lib/shared/stores/url-state';
	import { daemonHealths, setupItems } from '$lib/shared/stores/metadata';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		common_dismiss,
		common_getStarted,
		common_openScanThatFoundIt,
		common_view,
		common_viewTopology,
		credentials_createCredential,
		daemons_upgradeDaemon,
		gettingStarted_emailNotification,
		gettingStarted_havingTrouble,
		gettingStarted_stepDaemonLabel,
		gettingStarted_stepDiscoveryLabel,
		home_nudges_apiKeysAction,
		home_nudges_dependenciesAction,
		home_nudges_explorePerspectivesAction,
		home_nudges_inviteTeamAction,
		home_nudges_multiSiteAction,
		home_nudges_shareAction,
		home_nudges_tagsAction,
		home_nudges_targetlessCredentialAction,
		home_nudges_targetlessCredentialDescription,
		home_nudges_targetlessCredentialTitle,
		home_nudges_unconfirmedDescription,
		home_nudges_unconfirmedReviewHosts,
		home_nudges_unconfirmedReviewSubnets,
		common_answer
	} from '$lib/paraglide/messages';
	import { isDismissable, type SetupItemType, type SetupListEntry } from '../setup-list';

	type DiscoveryUpdatePayload = components['schemas']['DiscoveryUpdatePayload'];

	let {
		entry,
		setup,
		lookups,
		session = null,
		hasEmail = false,
		onDismiss,
		onAnswer,
		onTroubleshoot
	}: {
		entry: SetupListEntry;
		/** The org's finished and dismissed item types. */
		setup: SetupItemType[];
		lookups: SetupLookups;
		/** The running first scan, for a RunFirstScan row. */
		session?: DiscoveryUpdatePayload | null;
		hasEmail?: boolean;
		onDismiss: (itemType: SetupItemType) => void;
		/** Opens the question a question item asks. */
		onAnswer: (itemType: 'AnswerReferralSource' | 'AnswerProfile') => void;
		/** Opens the daemon troubleshooting help. */
		onTroubleshoot: () => void;
	} = $props();

	let item = $derived(entry.item);
	let Icon = $derived(setupItems.getIconComponent(item.type));
	let iconColor = $derived(setupItems.getColorHelper(item.type).icon);

	function go(tab: string, modal?: string, opts?: Parameters<typeof openModal>[1]) {
		window.location.hash = tab;
		if (modal) openModal(modal, opts);
	}

	function chip(
		type: EntityDiscriminants,
		id: string,
		label: string,
		loaded: object | undefined
	): Subject {
		return { type, id, label, data: loaded ?? { id, name: label } };
	}

	/** What the row says and offers, by item type. */
	let view = $derived.by(
		(): { title: string; sentence: string; subjects: Subject[]; fixes: SetupFix[] } => {
			const title = setupItems.getName(item.type);
			const sentence = setupItems.getDescription(item.type);
			switch (item.type) {
				case 'DaemonHealth': {
					const { daemon_id: id, name, health } = item;
					const fix: SetupFix =
						health === 'awaiting_connection'
							? { label: gettingStarted_havingTrouble(), run: onTroubleshoot }
							: health === 'outdated' || health === 'deprecated' || health === 'unsupported'
								? {
										label: daemons_upgradeDaemon(),
										run: () => go('daemons', 'upgrade-daemon', { id })
									}
								: {
										label: common_view(),
										run: () => navigateToEntity('Daemon', id, lookups.daemon(id))
									};
					return {
						title,
						sentence: daemonHealths.getDescription(health),
						subjects: [chip('Daemon', id, name, lookups.daemon(id))],
						fixes: [fix]
					};
				}
				case 'UncoveredIntegration': {
					const { integration, host_ids, site_ids } = item;
					const [first] = host_ids;
					const host = first ? lookups.host(first) : undefined;
					const subjects: Subject[] =
						first && host
							? [
									{
										...chip('Host', first, hostDisplayName(host), host),
										badge: host_ids.length > 1 ? `+${host_ids.length - 1}` : undefined
									}
								]
							: [];
					return {
						title: integrationWithoutCredentialTitle(integration),
						sentence: integrationWithoutCredentialSentence(integration, host_ids.length),
						subjects,
						fixes: [
							{
								label: createIntegrationCredentialLabel(integration),
								run: () =>
									openCredentialCreate(integration, { hostIds: host_ids, siteIds: site_ids })
							}
						]
					};
				}
				case 'TargetlessCredential': {
					const { credential_id: id, name } = item;
					return {
						title: home_nudges_targetlessCredentialTitle({ name }),
						sentence: home_nudges_targetlessCredentialDescription(),
						subjects: [chip('Credential', id, name, lookups.credential(id))],
						fixes: [
							{
								label: home_nudges_targetlessCredentialAction(),
								run: () => go('credentials', 'credential-editor', { id, tab: 'assignments' })
							}
						]
					};
				}
				case 'SiteWithoutDaemon': {
					const { site_id: id, name } = item;
					return {
						title,
						sentence,
						subjects: [chip('Site', id, name, lookups.site(id))],
						fixes: [
							{
								label: gettingStarted_stepDaemonLabel(),
								run: () => go('daemons', 'create-daemon', { prefill: { sites: id } })
							}
						]
					};
				}
				case 'RunWarnings': {
					const entry = buildWarningReport(
						item.warnings,
						(type, id) => (type === 'Credential' ? lookups.credential(id)?.name : undefined),
						item.daemon_name,
						lookups.credential
					)[0]?.entries[0];
					const action = entry
						? actionFor(entry, {
								daemonId: item.daemon_id,
								siteId: item.site_id,
								scanId: item.scan_id ?? null,
								credentialOf: lookups.credential
							})
						: null;
					const discoveryId = item.discovery_id;
					return {
						title: entry?.title ?? title,
						sentence: entry?.details[0]?.sentence ?? entry?.devices[0]?.sentence ?? sentence,
						subjects: [
							chip('Daemon', item.daemon_id, item.daemon_name, lookups.daemon(item.daemon_id))
						],
						fixes: [
							action
								? { label: action.label, run: action.run }
								: {
										label: common_openScanThatFoundIt(),
										run: () =>
											go('discovery-history', 'discovery-history-detail', { id: discoveryId })
									}
						]
					};
				}
				case 'InstallDaemon':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{ label: gettingStarted_stepDaemonLabel(), run: () => go('daemons', 'create-daemon') }
						]
					};
				case 'RunFirstScan':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{ label: gettingStarted_stepDiscoveryLabel(), run: () => go('discovery-scans') }
						]
					};
				case 'OpenTopology':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [{ label: common_viewTopology(), run: () => go('topology') }]
					};
				case 'AnswerReferralSource':
				case 'AnswerProfile': {
					const type = item.type;
					return {
						title,
						sentence,
						subjects: [],
						fixes: [{ label: common_answer(), run: () => onAnswer(type) }]
					};
				}
				case 'ReviewUnconfirmed': {
					const { hosts, subnets } = item;
					// Each opens its list filtered to the Inferred source, which is exactly the
					// unconfirmed rows. The server counts only what the user may review.
					const review = (tab: string) => () =>
						openTab(tab, new URLSearchParams({ sources: 'Inferred' }));
					return {
						title,
						sentence: home_nudges_unconfirmedDescription({ hosts, subnets }),
						subjects: [],
						fixes: [
							...(hosts > 0
								? [{ label: home_nudges_unconfirmedReviewHosts(), run: review('hosts') }]
								: []),
							...(subnets > 0
								? [{ label: home_nudges_unconfirmedReviewSubnets(), run: review('subnets') }]
								: [])
						]
					};
				}
				case 'ExplorePerspectives':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{
								label: home_nudges_explorePerspectivesAction(),
								run: () => {
									showViewSwitcherHint.set(true);
									go('topology');
								}
							}
						]
					};
				case 'ApplicationTags':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{
								label: common_getStarted(),
								run: () => {
									activeView.set('Application');
									go('topology');
								}
							}
						]
					};
				case 'Dependencies':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{
								label: home_nudges_dependenciesAction(),
								run: () => {
									activeView.set(setup.includes('ApplicationTags') ? 'Application' : 'Workloads');
									showDependencyTutorial.set(true);
									go('topology');
								}
							}
						]
					};
				case 'InterfaceCredential':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{
								label: credentials_createCredential(),
								run: () => go('credentials', 'credential-editor')
							}
						]
					};
				case 'Tags':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [{ label: home_nudges_tagsAction(), run: () => go('tags', 'tag-editor') }]
					};
				case 'InviteTeam':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{ label: home_nudges_inviteTeamAction(), run: () => go('users', 'invite-user') }
						]
					};
				case 'MultiSite':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [{ label: home_nudges_multiSiteAction(), run: () => go('sites', 'site-editor') }]
					};
				case 'ApiKeys':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{ label: home_nudges_apiKeysAction(), run: () => go('api-keys', 'user-api-key') }
						]
					};
				case 'Share':
					return {
						title,
						sentence,
						subjects: [],
						fixes: [
							{ label: home_nudges_shareAction(), run: () => go('topology', 'topology-share') }
						]
					};
			}
		}
	);

	function runFix(fix: SetupFix) {
		trackEvent('setup_item_action_clicked', { item_type: item.type });
		fix.run();
	}

	let rowActions = $derived<RowAction[]>(
		isDismissable(entry)
			? [{ label: common_dismiss(), icon: X, onClick: () => onDismiss(item.type) }]
			: []
	);
</script>

<!--
	Each row is its own bordered surface, so its buttons read with its text. Every row keeps the
	dismiss slot, filled or empty, so the buttons share one right edge.
-->
<li
	class="flex items-center gap-4 rounded-xl border px-5 py-4"
	style="border-color: var(--color-border); background: var(--color-bg-surface)"
>
	<div class="min-w-0 flex-1 space-y-1">
		<p class="text-sm font-medium text-primary">{view.title}</p>
		<p class="text-sm text-tertiary">{view.sentence}</p>
		{#if view.subjects.length > 0 || item.type === 'DaemonHealth'}
			<div class="flex flex-wrap items-center gap-1">
				{#each view.subjects as subject (subject.id)}
					<EntityTag
						entityRef={entityRef(subject.type, subject.id, subject.data)}
						label={subject.label}
						badge={subject.badge}
					/>
				{/each}
				{#if item.type === 'DaemonHealth'}
					<Tag {...daemonHealths.getTag(item.health)} />
				{/if}
			</div>
		{/if}
		{#if item.type === 'RunFirstScan' && session}
			<div class="space-y-1 pt-1">
				<DiscoveryEstimation
					phase={session.phase}
					hosts_discovered={session.hosts_discovered}
					estimated_remaining_secs={session.estimated_remaining_secs}
					is_full_scan={isFullScan(session.discovery_type)}
				/>
				<SessionProgress
					session_id={session.session_id}
					phase={session.phase}
					progress={session.progress}
				/>
				{#if hasEmail}
					<p class="text-xs text-secondary">{gettingStarted_emailNotification()}</p>
				{/if}
			</div>
		{/if}
	</div>
	<div class="flex shrink-0 flex-wrap items-center justify-end gap-2">
		{#each view.fixes as fix (fix.label)}
			<button
				type="button"
				class="btn-secondary flex items-center gap-2 text-sm"
				onclick={() => runFix(fix)}
			>
				<Icon size={16} class="shrink-0 {iconColor}" />
				{fix.label}
			</button>
		{/each}
	</div>
	<div class="flex w-8 shrink-0 justify-center">
		{#if rowActions.length > 0}
			<RowActions actions={rowActions} />
		{/if}
	</div>
</li>
