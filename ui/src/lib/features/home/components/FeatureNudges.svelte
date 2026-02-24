<script lang="ts">
	import type { components } from '$lib/api/schema';
	import FeatureNudge from './FeatureNudge.svelte';
	import { openModal } from '$lib/shared/stores/modal-registry';
	import { optionsPanelExpanded } from '$lib/features/topology/queries';
	import { onMount } from 'svelte';

	type Organization = components['schemas']['Organization'];
	type TelemetryOperation = components['schemas']['TelemetryOperation'];
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
	const has = (op: TelemetryOperation) => onboarding.includes(op);

	const isPaidPlan = $derived(planType != null && planType !== 'Free' && planType !== 'Demo');
	const isProPlus = $derived(isPaidPlan && planType !== 'Starter');
	const isTeamPlus = $derived(isProPlus && planType !== 'Pro');

	interface Nudge {
		id: string;
		title: string;
		description: string;
		actionLabel: string;
		action: () => void;
		visible: boolean;
	}

	let nudges = $derived.by((): Nudge[] => {
		const all: Nudge[] = [
			{
				id: 'tags',
				title: 'Organize with Tags',
				description: 'Add tags to group and filter your hosts, services, and other entities.',
				actionLabel: 'Go to Tags',
				action: () => {
					onNavigate('tags');
					openModal('tag-editor');
				},
				visible: !has('FirstTagCreated')
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
				visible: has('FirstTopologyRebuild') && !has('FirstGroupCreated')
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
				visible: has('FirstTopologyRebuild') && !has('FirstGroupCreated')
			},
			{
				id: 'snmp',
				title: 'Enable SNMP Discovery',
				description: 'Add SNMP credentials to discover detailed interface and device information.',
				actionLabel: 'Add SNMP Credential',
				action: () => {
					onNavigate('snmp-credentials');
					openModal('snmp-credential-editor');
				},
				visible: !has('FirstSnmpCredentialCreated')
			},
			{
				id: 'scheduled-free',
				title: 'Schedule Automatic Scans',
				description: 'Upgrade to automatically discover network changes on a schedule.',
				actionLabel: 'View Plans',
				action: () => openModal('billing-plan'),
				visible: planType === 'Free'
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
				visible: isProPlus && !has('FirstUserApiKeyCreated')
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
				visible: isProPlus && dashboard.networks.length === 1
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
				visible: isProPlus
			},
			{
				id: 'invite-team',
				title: 'Invite Your Team',
				description: 'Collaborate with your team by inviting members to your organization.',
				actionLabel: 'Invite Members',
				action: () => openModal('settings', { tab: 'team' }),
				visible: isTeamPlus && !has('InviteSent')
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
					{onDismiss}
				/>
			{/each}
		</div>
	</section>
{/if}
