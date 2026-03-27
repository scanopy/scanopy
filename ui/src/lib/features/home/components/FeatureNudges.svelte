<script lang="ts">
	import type { components } from '$lib/api/schema';
	import FeatureNudge from './FeatureNudge.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { upgradeContext } from '$lib/features/billing/stores';
	import { optionsPanelExpanded } from '$lib/features/topology/queries';
	import { entities, billingPlans } from '$lib/shared/stores/metadata';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useDaemonsQuery } from '$lib/features/daemons/queries';
	import type { IconComponent } from '$lib/shared/utils/types';
	import { onMount } from 'svelte';
	import {
		home_nudges_unclaimedPortsAction,
		home_nudges_unclaimedPortsDescription,
		home_nudges_unclaimedPortsTitle
	} from '$lib/paraglide/messages';

	type Organization = components['schemas']['Organization'];
	type OnboardingOperation = components['schemas']['OnboardingOperation'];
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

	const servicesQuery = useServicesCacheQuery();
	const daemonsQuery = useDaemonsQuery();
	let hasUnclaimedPorts = $derived(
		(servicesQuery.data ?? []).some((s) => s.service_definition === 'Unclaimed Open Ports')
	);
	let hasUnreachableDaemons = $derived(
		(daemonsQuery.data ?? []).some((d) => d.standby === true || d.is_unreachable === true)
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
	interface Nudge {
		id: string;
		title: string;
		description: string;
		actionLabel: string;
		action: () => void;
		visible: boolean;
		icon: IconComponent;
		iconColor: string;
	}

	let nudges = $derived.by((): Nudge[] => {
		const all: Nudge[] = [
			{
				id: 'unclaimed-ports',
				title: home_nudges_unclaimedPortsTitle(),
				description: home_nudges_unclaimedPortsDescription(),
				actionLabel: home_nudges_unclaimedPortsAction(),
				action: () => {
					window.open(
						'https://scanopy.net/docs/using-scanopy/network-data/#unclaimed-open-ports',
						'_blank'
					);
				},
				visible: has('FirstDiscoveryCompleted') && hasUnclaimedPorts,
				icon: entities.getIconComponent('Port'),
				iconColor: entities.getColorHelper('Port').icon
			},
			{
				id: 'daemon-attention',
				title: 'Daemon needs attention',
				description:
					'One or more daemons are offline or unreachable. Scheduled scans targeting these daemons will be skipped until connectivity is restored.',
				actionLabel: 'View Daemons',
				action: () => {
					onNavigate('daemons');
				},
				visible: hasUnreachableDaemons,
				icon: entities.getIconComponent('Daemon'),
				iconColor: entities.getColorHelper('Daemon').icon
			},
			{
				id: 'tags',
				title: 'Organize with Tags',
				description: 'Add tags to group and filter your hosts, services, and other entities.',
				actionLabel: 'Go to Tags',
				action: () => {
					onNavigate('tags');
					openModal('tag-editor');
				},
				visible: has('FirstDiscoveryCompleted') && !has('FirstTagCreated'),
				icon: entities.getIconComponent('Tag'),
				iconColor: entities.getColorHelper('Tag').icon
			},
			{
				id: 'topology-customize',
				title: 'Customize Your Topology',
				description:
					'Use the options panel to filter nodes, hide edges, and organize your network view.',
				actionLabel: 'Open Options',
				action: () => {
					onNavigate('topology');
					optionsPanelExpanded.set(true);
				},
				visible: has('FirstTopologyRebuild') && !has('FirstGroupCreated'),
				icon: entities.getIconComponent('Topology'),
				iconColor: entities.getColorHelper('Topology').icon
			},
			{
				id: 'groups',
				title: 'Create a Group',
				description:
					'Group related services together on the topology to keep your network view organized.',
				actionLabel: 'Create Group',
				action: () => {
					onNavigate('groups');
					openModal('group-editor');
				},
				visible: has('FirstTopologyRebuild') && !has('FirstGroupCreated'),
				icon: entities.getIconComponent('Group'),
				iconColor: entities.getColorHelper('Group').icon
			},
			{
				id: 'snmp',
				title: 'Enable SNMP Discovery',
				description: 'Add SNMP credentials to discover detailed interface and device information.',
				actionLabel: 'Add SNMP Credential',
				action: () => {
					onNavigate('credentials');
					openModal('credential-editor');
				},
				visible: !has('FirstSnmpCredentialCreated'),
				icon: entities.getIconComponent('Credential'),
				iconColor: entities.getColorHelper('Credential').icon
			},
			{
				id: 'scheduled-free',
				title: 'Schedule Automatic Scans',
				description: 'Upgrade to automatically discover network changes on a schedule.',
				actionLabel: 'View Plans',
				action: () => {
					upgradeContext.set(null);
					openModal('billing-plan');
				},
				visible: !features.scheduled_discovery,
				icon: entities.getIconComponent('Discovery'),
				iconColor: entities.getColorHelper('Discovery').icon
			},
			{
				id: 'api-keys',
				title: 'Automate with the API',
				description: 'Create an API key to integrate Scanopy with your tools and workflows.',
				actionLabel: 'Create API Key',
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
				id: 'multi-network',
				title: 'Add Another Network',
				description: 'Monitor multiple sites or environments by adding a second network.',
				actionLabel: 'Add Network',
				action: () => {
					onNavigate('networks');
					openModal('network-editor');
				},
				visible:
					(organization.plan?.included_networks === null ||
						(organization.plan?.included_networks ?? 0) > 1 ||
						(organization.plan?.network_cents ?? 0) > 0) &&
					dashboard.networks.length === 1,
				icon: entities.getIconComponent('Network'),
				iconColor: entities.getColorHelper('Network').icon
			},
			{
				id: 'share',
				title: 'Share Your Topology',
				description: 'Create a live link or embed to share your network topology with others.',
				actionLabel: 'Create Share',
				action: () => {
					onNavigate('topology');
					openModal('topology-share');
				},
				visible: features.share_views && has('FirstTopologyRebuild'),
				icon: entities.getIconComponent('Share'),
				iconColor: entities.getColorHelper('Share').icon
			},
			{
				id: 'invite-team',
				title: 'Invite Your Team',
				description: 'Collaborate with your team by inviting members to your organization.',
				actionLabel: 'Invite Members',
				action: () => {
					onNavigate('users');
					openModal('invite-user');
				},
				visible:
					((organization.plan?.included_seats ?? 0) > 1 ||
						(organization.plan?.seat_cents ?? 0) > 0) &&
					!has('InviteSent'),
				icon: entities.getIconComponent('User'),
				iconColor: entities.getColorHelper('User').icon
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
		<h3 class="text-primary mb-3 text-base font-semibold">Suggestions</h3>
		<div class="grid gap-4 sm:grid-cols-2">
			{#each nudges.filter((n) => visibleNudgeIds.includes(n.id)) as nudge (nudge.id)}
				<FeatureNudge
					id={nudge.id}
					title={nudge.title}
					description={nudge.description}
					actionLabel={nudge.actionLabel}
					onAction={nudge.action}
					Icon={nudge.icon}
					iconColor={nudge.iconColor}
					{onDismiss}
				/>
			{/each}
		</div>
	</section>
{/if}
