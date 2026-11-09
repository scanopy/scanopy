<script lang="ts" module>
	import { hostDisplayName } from '$lib/features/hosts/host-display-name';

	/** A row's fix: what clicking the row does, and its label. */
	export interface SetupFix {
		label: string;
		run: () => void;
	}

	/** The loaded entities a row names or opens. */
	export interface SetupLookups {
		daemon: (id: string) => Record<string, unknown> | undefined;
		host: (
			id: string
		) => (Record<string, unknown> & Parameters<typeof hostDisplayName>[0]) | undefined;
		site: (id: string) => Record<string, unknown> | undefined;
		credential: CredentialLookup;
	}

	/**
	 * One chip beside a row's title: the entity the item is about, which the title doesn't name,
	 * with `+N` when it stands for several. Its own tooltip and click open that entity.
	 */
	interface Subject {
		type: EntityDiscriminants;
		id: string;
		label: string;
		data: object | undefined;
		badge?: string;
	}
</script>

<script lang="ts">
	import { ChevronRight, CircleCheck } from '@lucide/svelte';
	import type { components } from '$lib/api/schema';
	import type { EntityDiscriminants } from '$lib/api/entities';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import DiscoveryEstimation from '$lib/features/discovery/components/DiscoveryEstimation.svelte';
	import SessionProgress from '$lib/features/discovery/components/cards/SessionProgress.svelte';
	import { isFullScan } from '$lib/features/discovery/utils/fullScan';
	import {
		buildWarningReport,
		credentialIdsOf,
		hostIdsOf,
		remedyOf
	} from '$lib/features/discovery/utils/warnings';
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
	import { daemonHealths, entities, setupItems } from '$lib/shared/stores/metadata';
	import { trackEvent } from '$lib/shared/utils/analytics';
	import {
		common_getStarted,
		common_openScanThatFoundIt,
		common_viewTopology,
		credentials_createCredential,
		daemons_upgradeDaemon,
		discovery_fixSeeDeviceChecks,
		gettingStarted_emailNotification,
		gettingStarted_stepDaemonLabel,
		home_setup_installDaemonOnSite,
		home_setup_openDaemon,
		home_setup_troubleshootConnection,
		gettingStarted_stepDiscoveryLabel,
		home_nudges_apiKeysAction,
		home_nudges_dependenciesAction,
		home_nudges_exploreViewsAction,
		home_nudges_inviteTeamAction,
		home_nudges_multiSiteAction,
		home_nudges_shareAction,
		home_nudges_tagsAction,
		home_nudges_targetlessCredentialAction,
		home_nudges_targetlessCredentialDescription,
		home_nudges_unconfirmedHostsDescription,
		home_nudges_unconfirmedReviewHosts,
		home_nudges_unconfirmedReviewSubnets,
		home_nudges_unconfirmedSubnetsDescription,
		common_answer
	} from '$lib/paraglide/messages';
	import type { SetupItemType, SetupListEntry } from '../setup-list';

	type DiscoveryUpdatePayload = components['schemas']['DiscoveryUpdatePayload'];

	let {
		entry,
		setup,
		lookups,
		session = null,
		hasEmail = false,
		onAnswer,
		onTroubleshoot
	}: {
		entry: SetupListEntry;
		/** The org's done Setup item types. */
		setup: SetupItemType[];
		lookups: SetupLookups;
		/** The running first scan, for a RunFirstScan row. */
		session?: DiscoveryUpdatePayload | null;
		hasEmail?: boolean;
		/** Opens the question a question item asks. */
		onAnswer: (itemType: 'AnswerReferralSource' | 'AnswerProfile') => void;
		/** Opens the daemon troubleshooting help. */
		onTroubleshoot: () => void;
	} = $props();

	let item = $derived(entry.item);
	let done = $derived(entry.state === 'Done');
	let Icon = $derived(setupItems.getIconComponent(item.type));
	let iconColors = $derived(setupItems.getColorHelper(item.type));

	function go(tab: string, modal?: string, opts?: Parameters<typeof openModal>[1]) {
		window.location.hash = tab;
		if (modal) openModal(modal, opts);
	}

	/** The first of `ids` as a chip, with `+N` for the rest. Empty when there are none. */
	function firstOf(
		type: EntityDiscriminants,
		ids: string[],
		resolve: (id: string) => { label: string; data: object | undefined } | undefined
	): Subject[] {
		const [first] = ids;
		const resolved = first ? resolve(first) : undefined;
		if (!first || !resolved) return [];
		return [
			{
				type,
				id: first,
				...resolved,
				badge: ids.length > 1 ? `+${ids.length - 1}` : undefined
			}
		];
	}

	function hostSubjects(ids: string[]): Subject[] {
		return firstOf('Host', ids, (id) => {
			const host = lookups.host(id);
			return host ? { label: hostDisplayName(host), data: host } : undefined;
		});
	}

	function credentialSubjects(ids: string[]): Subject[] {
		return firstOf('Credential', ids, (id) => {
			const credential = lookups.credential(id);
			return credential ? { label: credential.name, data: credential } : undefined;
		});
	}

	/** Opens a list filtered to the Inferred source, which is exactly its unconfirmed rows. */
	function reviewInferred(tab: string) {
		return () => openTab(tab, new URLSearchParams({ sources: 'Inferred' }));
	}

	/** What the row says and does, by item type. */
	let view = $derived.by(
		(): { title: string; sentence: string; subjects: Subject[]; fix: SetupFix } => {
			const title = setupItems.getName(item.type);
			const sentence = setupItems.getDescription(item.type);
			const plain = (fix: SetupFix) => ({ title, sentence, subjects: [], fix });
			switch (item.type) {
				case 'DaemonHealth': {
					const { daemon_id: id, name, health } = item;
					// A daemon that isn't talking to the server gets the connection help; one on an
					// old release gets the upgrade; an unknown version opens the daemon itself.
					const fix: SetupFix =
						health === 'awaiting_connection' || health === 'unreachable' || health === 'standby'
							? { label: home_setup_troubleshootConnection(), run: onTroubleshoot }
							: health === 'outdated' || health === 'deprecated' || health === 'unsupported'
								? {
										label: daemons_upgradeDaemon(),
										run: () => go('daemons', 'upgrade-daemon', { id })
									}
								: {
										label: home_setup_openDaemon(),
										run: () => navigateToEntity('Daemon', id, lookups.daemon(id))
									};
					return {
						title,
						sentence: daemonHealths.getDescription(health),
						subjects: [{ type: 'Daemon', id, label: name, data: lookups.daemon(id) }],
						fix
					};
				}
				case 'UncoveredIntegration': {
					const { integration, host_ids, site_ids } = item;
					return {
						title: integrationWithoutCredentialTitle(integration),
						sentence: integrationWithoutCredentialSentence(integration, host_ids.length),
						subjects: hostSubjects(host_ids),
						fix: {
							label: createIntegrationCredentialLabel(integration),
							run: () => openCredentialCreate(integration, { hostIds: host_ids, siteIds: site_ids })
						}
					};
				}
				case 'TargetlessCredential': {
					// The title stays generic; the chip names the credential, once.
					const { credential_id: id, name } = item;
					return {
						title,
						sentence: home_nudges_targetlessCredentialDescription(),
						subjects: [{ type: 'Credential', id, label: name, data: lookups.credential(id) }],
						fix: {
							label: home_nudges_targetlessCredentialAction(),
							run: () => go('credentials', 'credential-editor', { id, tab: 'assignments' })
						}
					};
				}
				case 'SiteWithoutDaemon': {
					const { site_id: id, name } = item;
					return {
						title,
						sentence,
						subjects: [{ type: 'Site', id, label: name, data: lookups.site(id) }],
						fix: {
							label: home_setup_installDaemonOnSite(),
							run: () => go('daemons', 'create-daemon', { prefill: { sites: id } })
						}
					};
				}
				case 'RunWarnings': {
					const report = buildWarningReport(
						item.warnings,
						(type, id) => (type === 'Credential' ? lookups.credential(id)?.name : undefined),
						item.daemon_name,
						lookups.credential
					)[0]?.entries[0];
					const action = report
						? actionFor(report, {
								daemonId: item.daemon_id,
								siteId: item.site_id,
								scanId: item.scan_id ?? null,
								credentialOf: lookups.credential
							})
						: null;
					const discoveryId = item.discovery_id;
					// The credentials and hosts the warning is about. A scan-level warning names
					// neither, and then the chip is the daemon whose scan it was.
					const named = report
						? [...credentialSubjects(credentialIdsOf(report)), ...hostSubjects(hostIdsOf(report))]
						: [];
					return {
						title: report?.title ?? title,
						sentence: report?.details[0]?.sentence ?? report?.devices[0]?.sentence ?? sentence,
						subjects:
							named.length > 0
								? named
								: [
										{
											type: 'Daemon',
											id: item.daemon_id,
											label: item.daemon_name,
											data: lookups.daemon(item.daemon_id)
										}
									],
						// Without a fix in Scanopy, the run's report is the destination: for a
						// device-side problem it holds the commands to check the device with.
						fix: action
							? { label: action.label, run: action.run }
							: {
									label:
										remedyOf(item.code) === 'CheckTheDevice'
											? discovery_fixSeeDeviceChecks()
											: common_openScanThatFoundIt(),
									run: () =>
										go('discovery-history', 'discovery-history-detail', { id: discoveryId })
								}
					};
				}
				case 'UnconfirmedHosts':
					return {
						title,
						sentence: home_nudges_unconfirmedHostsDescription({ count: item.count }),
						subjects: [],
						fix: { label: home_nudges_unconfirmedReviewHosts(), run: reviewInferred('hosts') }
					};
				case 'UnconfirmedSubnets':
					return {
						title,
						sentence: home_nudges_unconfirmedSubnetsDescription({ count: item.count }),
						subjects: [],
						fix: { label: home_nudges_unconfirmedReviewSubnets(), run: reviewInferred('subnets') }
					};
				case 'InstallDaemon':
					return plain({
						label: gettingStarted_stepDaemonLabel(),
						run: () => go('daemons', 'create-daemon')
					});
				case 'RunFirstScan':
					return plain({
						label: gettingStarted_stepDiscoveryLabel(),
						run: () => go('discovery-scans')
					});
				case 'OpenTopology':
					return plain({ label: common_viewTopology(), run: () => go('topology') });
				case 'AnswerReferralSource':
				case 'AnswerProfile': {
					const type = item.type;
					return plain({ label: common_answer(), run: () => onAnswer(type) });
				}
				case 'ExploreViews':
					return plain({
						label: home_nudges_exploreViewsAction(),
						run: () => {
							showViewSwitcherHint.set(true);
							go('topology');
						}
					});
				case 'ApplicationTags':
					return plain({
						label: common_getStarted(),
						run: () => {
							activeView.set('Application');
							go('topology');
						}
					});
				case 'Dependencies':
					return plain({
						label: home_nudges_dependenciesAction(),
						run: () => {
							activeView.set(setup.includes('ApplicationTags') ? 'Application' : 'Workloads');
							showDependencyTutorial.set(true);
							go('topology');
						}
					});
				case 'InterfaceCredential':
					return plain({
						label: credentials_createCredential(),
						run: () => go('credentials', 'credential-editor')
					});
				case 'Tags':
					return plain({ label: home_nudges_tagsAction(), run: () => go('tags', 'tag-editor') });
				case 'InviteTeam':
					return plain({
						label: home_nudges_inviteTeamAction(),
						run: () => go('users', 'invite-user')
					});
				case 'MultiSite':
					return plain({
						label: home_nudges_multiSiteAction(),
						run: () => go('sites', 'site-editor')
					});
				case 'ApiKeys':
					return plain({
						label: home_nudges_apiKeysAction(),
						run: () => go('api-keys', 'user-api-key')
					});
				case 'Share':
					return plain({
						label: home_nudges_shareAction(),
						run: () => go('topology', 'topology-share')
					});
			}
		}
	);

	function runFix() {
		trackEvent('setup_item_action_clicked', { item_type: item.type });
		view.fix.run();
	}
</script>

{#snippet body()}
	<!-- The icon tile the app's other interactive cards use (SupportTab, ConfirmEntityModal). -->
	<div
		class="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg {done
			? ''
			: iconColors.bg}"
	>
		{#if done}
			<CircleCheck class="h-5 w-5 text-success" />
		{:else}
			<Icon class="h-5 w-5 {iconColors.icon}" />
		{/if}
	</div>
	<div class="min-w-0 flex-1 space-y-1">
		<!-- The title names the problem; the chips beside it name what it's about. Chips sit above
		     the card's click target, so they hover and open on their own. -->
		<div class="flex flex-wrap items-center gap-x-2 gap-y-1">
			<p class="text-sm font-medium {done ? 'text-secondary' : 'text-primary'}">{view.title}</p>
			{#if !done}
				{#each view.subjects as subject (subject.id)}
					<span class="pointer-events-auto relative z-10">
						<EntityTag
							entityRef={entityRef(
								subject.type,
								subject.id,
								subject.data ?? { id: subject.id, name: subject.label }
							)}
							label={subject.label}
							color={entities.getColorHelper(subject.type).color}
							badge={subject.badge}
						/>
					</span>
				{/each}
				{#if item.type === 'DaemonHealth'}
					<Tag {...daemonHealths.getTag(item.health)} />
				{/if}
			{/if}
		</div>
		{#if !done}
			<p class="text-sm text-tertiary">{view.sentence}</p>
		{/if}
		{#if !done && item.type === 'RunFirstScan' && session}
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
{/snippet}

<!-- An open row is an interactive card (`card` without `card-static` rings on hover, as the
     app's other clickable cards do). A transparent button covers the card and runs the fix; the
     content sits over it with pointer events off, except the entity chips, which keep their own
     tooltip and click. The label on the right says what the card click does. A done row is a
     static card that only reports. -->
<li>
	{#if done}
		<div class="card card-static flex items-center gap-3 p-4 opacity-70">
			{@render body()}
		</div>
	{:else}
		<div
			class="card relative flex cursor-pointer items-center gap-3 p-4 has-focus-visible:ring-2 has-focus-visible:ring-blue-500"
		>
			<button
				type="button"
				class="absolute inset-0 cursor-pointer rounded-2xl focus-visible:outline-none"
				aria-label="{view.title}: {view.fix.label}"
				onclick={runFix}
			></button>
			<div class="pointer-events-none relative flex min-w-0 flex-1 items-center gap-3">
				{@render body()}
				<span class="flex shrink-0 items-center gap-1 text-sm text-secondary">
					{view.fix.label}
					<ChevronRight size={16} />
				</span>
			</div>
		</div>
	{/if}
</li>
