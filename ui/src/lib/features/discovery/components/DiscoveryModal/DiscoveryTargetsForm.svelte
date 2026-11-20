<script lang="ts">
	import { useSubnetsQuery } from '#lib/features/subnets/queries.ts';
	import { SubnetDisplay } from '#lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import { subnetTypes } from '#lib/shared/stores/metadata.ts';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';
	import type { SubnetResponse } from '#lib/features/subnets/types/base.ts';
	import { disabledWhen } from '#lib/shared/components/data/types.ts';
	import { docsUrl } from '#lib/shared/utils/docs.ts';
	import { withRoutedTarget } from '../../utils/targets';
	import {
		common_subnets,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_daemonNetworkAlwaysScanned,
		discovery_leaveOutNetwork,
		discovery_leaveOutNetworkLinkText,
		discovery_planningDaemonsLinkText,
		discovery_routedNetworksFaster,
		discovery_routedNetworksLarge,
		discovery_selectSubnet,
		discovery_targetsEmpty,
		discovery_targetsHelp,
		discovery_targetsRoutedHelp
	} from '#lib/paraglide/messages.js';

	interface Props {
		formData: Discovery;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { formData = $bindable(), daemonHostId, daemon }: Props = $props();

	const subnetsQuery = useSubnetsQuery();

	let subnetsData = $derived(subnetsQuery.data ?? []);
	let hasTargets = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
	);
	let routedIds = $derived(
		formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'
			? (formData.discovery_type.subnet_ids ?? [])
			: []
	);

	function scanEligible(s: SubnetResponse): boolean {
		return subnetTypes.getMetadata(s.subnet_type).network_scan_discovery_eligible;
	}

	function isDaemonNetwork(s: { id: string }): boolean {
		return daemon.interfaced_subnet_ids.includes(s.id);
	}

	// Every scan sweeps the daemon's own networks, so they lead the list and can't be removed.
	let daemonNetworks = $derived(subnetsData.filter((s) => isDaemonNetwork(s) && scanEligible(s)));

	let routedSubnets = $derived(
		routedIds
			.map((id) => subnetsData.find((s) => s.id === id))
			.filter((s): s is SubnetResponse => s !== undefined && !isDaemonNetwork(s))
	);

	let items = $derived([...daemonNetworks, ...routedSubnets]);

	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				s.site_id == formData.site_id &&
				!routedIds.includes(s.id) &&
				!isDaemonNetwork(s) &&
				scanEligible(s)
		)
	);

	function getCidrPrefix(cidr: string): number | null {
		const parts = cidr.split('/');
		if (parts.length !== 2) return null;
		const prefix = parseInt(parts[1], 10);
		return isNaN(prefix) ? null : prefix;
	}

	function formatIpCount(count: number): string {
		if (count >= 1_000_000) return `~${Math.round(count / 1_000_000)}M IPs`;
		if (count >= 1_000) return `~${Math.round(count / 1_000)}K IPs`;
		return `${count} IPs`;
	}

	// Routed networks are probed address by address, so a large one takes hours.
	let largeRoutedSubnets = $derived(
		routedSubnets
			.filter((s) => {
				const prefix = getCidrPrefix(s.cidr);
				return prefix !== null && prefix <= 12;
			})
			.map((s) => {
				const prefix = getCidrPrefix(s.cidr)!;
				const ipCount = Math.pow(2, 32 - prefix);
				const name = s.name !== s.cidr ? s.name + ` (${s.cidr})` : s.cidr;
				return `${name} — ${formatIpCount(ipCount)}`;
			})
	);

	function handleRemove(index: number) {
		const removed = items[index];
		if (!removed || isDaemonNetwork(removed)) return;
		if (formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') {
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: routedIds.filter((id) => id !== removed.id)
			};
		}
	}
</script>

{#snippet targetsHelp()}
	<!-- One size throughout: DocsHint draws its sentence at text-xs. -->
	<div class="flex flex-wrap items-baseline gap-x-1 text-xs">
		<span>{discovery_targetsHelp()}</span>
		<DocsHint
			text={discovery_leaveOutNetwork()}
			href={docsUrl(
				'/docs/guides/networking/multiple-daemons/#restricting-daemons-to-specific-interfaces'
			)}
			linkText={discovery_leaveOutNetworkLinkText()}
		/>
		<span>{discovery_targetsRoutedHelp()}</span>
	</div>
{/snippet}

<div class="space-y-4">
	{#if daemonHostId == null}
		<InlineWarning title={discovery_daemonHostMissing()} body={discovery_daemonHostMissingHelp()} />
	{/if}

	{#if hasTargets}
		{#if largeRoutedSubnets.length > 0}
			<InlineWarning
				title=""
				body={discovery_routedNetworksLarge({ subnets: largeRoutedSubnets.join('\n') })}
			>
				<DocsHint
					class="mt-1"
					text={discovery_routedNetworksFaster()}
					href={docsUrl('/docs/plan-your-deployment/daemons/')}
					linkText={discovery_planningDaemonsLinkText()}
				/>
			</InlineWarning>
		{/if}
		<ListManager
			label={common_subnets()}
			helpSnippet={targetsHelp}
			placeholder={discovery_selectSubnet()}
			emptyMessage={discovery_targetsEmpty()}
			allowReorder={false}
			allowItemEdit={false}
			showSearch={true}
			options={availableSubnets}
			{items}
			optionDisplayComponent={SubnetDisplay}
			itemDisplayComponent={SubnetDisplay}
			getItemDisabledReason={(s) =>
				isDaemonNetwork(s) ? discovery_daemonNetworkAlwaysScanned() : null}
			itemRemoveUnavailable={(s) =>
				disabledWhen(isDaemonNetwork(s), discovery_daemonNetworkAlwaysScanned())}
			onAdd={(id) => (formData = withRoutedTarget(formData, id))}
			onRemove={handleRemove}
		/>
	{/if}
</div>
