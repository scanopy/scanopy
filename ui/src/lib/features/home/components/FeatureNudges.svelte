<script lang="ts">
	import type { components } from '$lib/api/schema';
	import FeatureNudge, { type NudgeAction } from './FeatureNudge.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import {
		activeView,
		showViewSwitcherHint,
		showDependencyTutorial
	} from '$lib/features/topology/queries';
	import {
		entities,
		billingPlans,
		credentialIntegrations,
		entitySources,
		interfaceIntegrationsList
	} from '$lib/shared/stores/metadata';
	import {
		createIntegrationCredentialLabel,
		openCredentialCreate
	} from '$lib/features/credentials/utils/prefill';
	import {
		integrationWithoutCredentialSentence,
		integrationWithoutCredentialTitle
	} from '$lib/features/discovery/utils/warnings';
	import { useCan } from '$lib/features/auth/access';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import { hasSunsetWarning } from '$lib/features/daemons/utils';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { onMount } from 'svelte';
	import { CircleDashedCheck } from '@lucide/svelte';
	import { openTab } from '$lib/shared/stores/url-state';
	import {
		common_getStarted,
		home_nudges_apiKeysAction,
		home_nudges_apiKeysDescription,
		home_nudges_apiKeysTitle,
		home_nudges_applicationTagsDescription,
		home_nudges_applicationTagsTitle,
		home_nudges_daemonAttentionAction,
		home_nudges_daemonAttentionDescription,
		home_nudges_daemonAttentionTitle,
		home_nudges_dependenciesAction,
		home_nudges_dependenciesDescription,
		home_nudges_dependenciesTitle,
		home_nudges_explorePerspectivesAction,
		home_nudges_explorePerspectivesDescription,
		home_nudges_explorePerspectivesTitle,
		home_nudges_inviteTeamAction,
		home_nudges_inviteTeamDescription,
		home_nudges_inviteTeamTitle,
		home_nudges_multiSiteAction,
		home_nudges_multiSiteDescription,
		home_nudges_multiSiteTitle,
		home_nudges_shareAction,
		home_nudges_shareDescription,
		home_nudges_shareTitle,
		credentials_createCredential,
		home_nudges_interfacesDescription,
		home_nudges_interfacesTitle,
		home_nudges_tagsAction,
		home_nudges_tagsDescription,
		home_nudges_tagsTitle,
		home_nudges_targetlessCredentialAction,
		home_nudges_targetlessCredentialDescription,
		home_nudges_targetlessCredentialTitle,
		home_nudges_unconfirmedReviewHosts,
		home_nudges_unconfirmedReviewSubnets,
		home_nudges_unconfirmedDescription,
		home_nudges_unconfirmedTitle
	} from '$lib/paraglide/messages';

	type Organization = components['schemas']['Organization'];
	type OnboardingOperation = components['schemas']['OnboardingOperationDiscriminants'];
	type DashboardSummary = components['schemas']['DashboardSummary'];

	let {
		organization,
		dashboard,
		onNavigate
	}: {
		organization: Organization;
		dashboard: DashboardSummary;
		onNavigate: (tab: string) => void;
	} = $props();

	const can = useCan();
	const daemonsQuery = useDaemonsQuery();
	// Surface the daemon nudge for connectivity issues AND for an upcoming/active
	// version sunset — an aging daemon needs attention just as much as an
	// unreachable one, and this is the population most likely to have one.
	let hasDaemonsNeedingAttention = $derived(
		(daemonsQuery.data ?? []).some(
			(d) => d.standby === true || d.is_unreachable === true || hasSunsetWarning(d)
		)
	);

	let mounted = $state(false);
	let dismissCount = $state(0);
	onMount(() => {
		mounted = true;
	});

	function onDismiss() {
		dismissCount++;
	}

	const planType = $derived(organization.plan?.type ?? null);
	const onboarding = $derived(organization.onboarding ?? []);
	const has = (op: OnboardingOperation) => onboarding.includes(op);

	const planMetadata = $derived(billingPlans.getMetadata(planType));
	const features = $derived(planMetadata?.features ?? {});
	/** A nudge with one action, or with several buttons side by side. */
	type Nudge = {
		id: string;
		title: string;
		description: string;
		visible: boolean;
		icon: IconComponent;
		iconColor: string;
		dismissOnAction?: boolean;
	} & ({ actionLabel: string; action: () => void } | { actions: NudgeAction[] });

	let nudges = $derived.by((): Nudge[] => {
		const all: Nudge[] = [
			{
				id: 'daemon-attention',
				title: home_nudges_daemonAttentionTitle(),
				description: home_nudges_daemonAttentionDescription(),
				actionLabel: home_nudges_daemonAttentionAction(),
				action: () => {
					onNavigate('daemons');
				},
				visible: hasDaemonsNeedingAttention,
				icon: entities.getIconComponent('Daemon'),
				iconColor: entities.getColorHelper('Daemon').icon
			},
			// One per integration the latest scans found with no credential, most hosts first. Gone
			// once a covering credential is saved, so acting on it does not dismiss it.
			...dashboard.uncovered_integrations.map(({ integration, host_ids, site_ids }): Nudge => ({
				id: `integration-${integration}`,
				title: integrationWithoutCredentialTitle(integration),
				description: integrationWithoutCredentialSentence(integration, host_ids.length),
				actionLabel: createIntegrationCredentialLabel(integration),
				action: () => openCredentialCreate(integration, { hostIds: host_ids, siteIds: site_ids }),
				visible: has('FirstDiscoveryCompleted') && can('Credential', 'Write'),
				icon: credentialIntegrations.getIconComponent(integration),
				iconColor: credentialIntegrations.getColorHelper(integration).icon,
				dismissOnAction: false
			})),
			// One per credential no scan sends to a daemon. Gone once it has a target, so acting on it
			// does not dismiss it.
			// Hosts and subnets known only from another device's report. Each button opens its list
			// filtered to the Inferred source, which is exactly the unconfirmed rows.
			{
				id: 'unconfirmed-entities',
				title: home_nudges_unconfirmedTitle(),
				description: home_nudges_unconfirmedDescription({
					hosts: dashboard.unconfirmed_host_count,
					subnets: dashboard.unconfirmed_subnet_count
				}),
				actions: [
					{
						tab: 'hosts',
						count: dashboard.unconfirmed_host_count,
						label: home_nudges_unconfirmedReviewHosts(),
						editable: can('Host', 'Write')
					},
					{
						tab: 'subnets',
						count: dashboard.unconfirmed_subnet_count,
						label: home_nudges_unconfirmedReviewSubnets(),
						editable: can('Subnet', 'Write')
					}
				]
					.filter(({ count, editable }) => count > 0 && editable)
					.map(({ tab, label }) => ({
						label,
						onAction: () => openTab(tab, new URLSearchParams({ sources: 'Inferred' }))
					})),
				visible:
					(dashboard.unconfirmed_host_count > 0 && can('Host', 'Write')) ||
					(dashboard.unconfirmed_subnet_count > 0 && can('Subnet', 'Write')),
				icon: CircleDashedCheck,
				iconColor: entitySources.getColorHelper('Inferred').icon,
				dismissOnAction: false
			},
			...dashboard.targetless_credentials.map(({ id, name, integration }): Nudge => ({
				id: `targetless-credential-${id}`,
				title: home_nudges_targetlessCredentialTitle({ name }),
				description: home_nudges_targetlessCredentialDescription(),
				actionLabel: home_nudges_targetlessCredentialAction(),
				action: () => {
					onNavigate('credentials');
					openModal('credential-editor', { id, tab: 'assignments' });
				},
				visible: can('Credential', 'Write'),
				icon: credentialIntegrations.getIconComponent(integration),
				iconColor: credentialIntegrations.getColorHelper(integration).icon,
				dismissOnAction: false
			})),
			{
				id: 'explore-perspectives',
				title: home_nudges_explorePerspectivesTitle(),
				description: home_nudges_explorePerspectivesDescription(),
				actionLabel: home_nudges_explorePerspectivesAction(),
				action: () => {
					showViewSwitcherHint.set(true);
					onNavigate('topology');
				},
				visible: has('FirstTopologyRebuild'),
				icon: entities.getIconComponent('Topology'),
				iconColor: entities.getColorHelper('Topology').icon
			},
			{
				id: 'application-tags',
				title: home_nudges_applicationTagsTitle(),
				description: home_nudges_applicationTagsDescription(),
				actionLabel: common_getStarted(),
				action: () => {
					activeView.set('Application');
					onNavigate('topology');
				},
				visible:
					has('FirstTopologyRebuild') && !has('FirstApplicationTagCreated') && can('Tag', 'Write'),
				icon: entities.getIconComponent('Tag'),
				iconColor: entities.getColorHelper('Tag').icon
			},
			{
				id: 'dependencies',
				title: home_nudges_dependenciesTitle(),
				description: home_nudges_dependenciesDescription(),
				actionLabel: home_nudges_dependenciesAction(),
				action: () => {
					activeView.set(has('FirstApplicationTagCreated') ? 'Application' : 'Workloads');
					showDependencyTutorial.set(true);
					onNavigate('topology');
				},
				visible:
					has('FirstTopologyRebuild') &&
					!has('FirstDependencyCreated') &&
					can('Dependency', 'Write'),
				icon: entities.getIconComponent('Dependency'),
				iconColor: entities.getColorHelper('Dependency').icon
			},
			{
				id: 'snmp',
				title: home_nudges_interfacesTitle(),
				description: home_nudges_interfacesDescription({
					integrations: interfaceIntegrationsList()
				}),
				actionLabel: credentials_createCredential(),
				action: () => {
					onNavigate('credentials');
					openModal('credential-editor');
				},
				visible:
					has('FirstDiscoveryCompleted') &&
					!has('FirstInterfaceCredentialCreated') &&
					can('Credential', 'Write'),
				icon: entities.getIconComponent('Credential'),
				iconColor: entities.getColorHelper('Credential').icon
			},
			{
				id: 'tags',
				title: home_nudges_tagsTitle(),
				description: home_nudges_tagsDescription(),
				actionLabel: home_nudges_tagsAction(),
				action: () => {
					onNavigate('tags');
					openModal('tag-editor');
				},
				visible: has('FirstDiscoveryCompleted') && !has('FirstTagCreated') && can('Tag', 'Write'),
				icon: entities.getIconComponent('Tag'),
				iconColor: entities.getColorHelper('Tag').icon
			},
			{
				id: 'invite-team',
				title: home_nudges_inviteTeamTitle(),
				description: home_nudges_inviteTeamDescription(),
				actionLabel: home_nudges_inviteTeamAction(),
				action: () => {
					onNavigate('users');
					openModal('invite-user');
				},
				visible:
					((organization.plan?.included_seats ?? 0) > 1 ||
						(organization.plan?.seat_cents ?? 0) > 0) &&
					!has('InviteSent') &&
					can('Member', 'Write'),
				icon: entities.getIconComponent('User'),
				iconColor: entities.getColorHelper('User').icon
			},
			{
				id: 'multi-site',
				title: home_nudges_multiSiteTitle(),
				description: home_nudges_multiSiteDescription(),
				actionLabel: home_nudges_multiSiteAction(),
				action: () => {
					onNavigate('sites');
					openModal('site-editor');
				},
				visible:
					(organization.plan?.included_sites === null ||
						(organization.plan?.included_sites ?? 0) > 1 ||
						(organization.plan?.site_cents ?? 0) > 0) &&
					dashboard.sites.length === 1 &&
					can('NewSite', 'Write'),
				icon: entities.getIconComponent('Site'),
				iconColor: entities.getColorHelper('Site').icon
			},
			{
				id: 'api-keys',
				title: home_nudges_apiKeysTitle(),
				description: home_nudges_apiKeysDescription(),
				actionLabel: home_nudges_apiKeysAction(),
				action: () => {
					onNavigate('api-keys');
					openModal('user-api-key');
				},
				visible:
					features.api_access && has('FirstDiscoveryCompleted') && !has('FirstUserApiKeyCreated'),
				icon: entities.getIconComponent('UserApiKey'),
				iconColor: entities.getColorHelper('UserApiKey').icon
			},
			{
				id: 'share',
				title: home_nudges_shareTitle(),
				description: home_nudges_shareDescription(),
				actionLabel: home_nudges_shareAction(),
				action: () => {
					onNavigate('topology');
					openModal('topology-share');
				},
				visible: features.sharing && has('FirstTopologyRebuild') && can('Share', 'Write'),
				icon: entities.getIconComponent('Share'),
				iconColor: entities.getColorHelper('Share').icon
			}
		];

		return all.filter((n) => n.visible);
	});

	// Check localStorage for dismissed nudges and limit to 2
	// dismissCount is a reactive dependency so this recomputes when a nudge is dismissed
	let visibleNudgeIds = $derived.by((): string[] => {
		void dismissCount;
		if (!mounted) return [];
		const visible: string[] = [];
		for (const nudge of nudges) {
			if (localStorage.getItem(`nudge-dismissed:${nudge.id}`) !== 'true') {
				visible.push(nudge.id);
			}
			if (visible.length >= 2) break;
		}
		return visible;
	});
</script>

{#if visibleNudgeIds.length > 0}
	<section>
		<h3 class="mb-3 text-base font-semibold text-primary">Suggestions</h3>
		<div class="grid gap-4 sm:grid-cols-2">
			{#each nudges.filter((n) => visibleNudgeIds.includes(n.id)) as nudge (nudge.id)}
				<FeatureNudge
					id={nudge.id}
					title={nudge.title}
					description={nudge.description}
					actions={'actions' in nudge
						? nudge.actions
						: [{ label: nudge.actionLabel, onAction: nudge.action }]}
					Icon={nudge.icon}
					iconColor={nudge.iconColor}
					dismissOnAction={nudge.dismissOnAction}
					{onDismiss}
				/>
			{/each}
		</div>
	</section>
{/if}
