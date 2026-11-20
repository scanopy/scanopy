<script lang="ts">
	import { useCreateSubnetMutation, useSubnetsQuery } from '#lib/features/subnets/queries.ts';
	import { SubnetDisplay } from '#lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '#lib/shared/components/forms/selection/ListManager.svelte';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '#lib/shared/components/feedback/InlineWarning.svelte';
	import DocsHint from '#lib/shared/components/feedback/DocsHint.svelte';
	import { subnetTypes } from '#lib/shared/stores/metadata.ts';
	import type { Daemon } from '#lib/features/daemons/types/base.ts';
	import type { Subnet, SubnetResponse } from '#lib/features/subnets/types/base.ts';
	import SubnetEditModal from '#lib/features/subnets/components/SubnetEditModal/SubnetEditModal.svelte';
	import { disabledWhen } from '#lib/shared/components/data/types.ts';
	import { docsUrl } from '#lib/shared/utils/docs.ts';
	import { withRoutedTarget } from '../../utils/targets';
	import {
		discovery_addNetwork,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_daemonNetworkAlwaysScanned,
		discovery_daemonNetworks,
		discovery_daemonNetworksEmpty,
		discovery_daemonNetworksHelp,
		discovery_leaveOutNetwork,
		discovery_leaveOutNetworkLinkText,
		discovery_routedNetworks,
		discovery_routedNetworksEmpty,
		discovery_routedNetworksHelp,
		discovery_routedNetworksLarge,
		discovery_selectSubnet
	} from '#lib/paraglide/messages.js';

	interface Props {
		formData: Discovery;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { formData = $bindable(), daemonHostId, daemon }: Props = $props();

	const subnetsQuery = useSubnetsQuery();
	const createSubnetMutation = useCreateSubnetMutation();

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

	// Every scan sweeps these. The list follows the daemon's interfaces, not this scan.
	let daemonNetworks = $derived(
		subnetsData.filter((s) => daemon.interfaced_subnet_ids.includes(s.id) && scanEligible(s))
	);

	let routedSubnets = $derived(
		routedIds
			.map((id) => subnetsData.find((s) => s.id === id))
			.filter((s): s is SubnetResponse => s !== undefined)
	);

	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				s.site_id == formData.site_id &&
				!routedIds.includes(s.id) &&
				!daemon.interfaced_subnet_ids.includes(s.id) &&
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

	function setRoutedIds(ids: string[]) {
		if (formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') {
			formData.discovery_type = { ...formData.discovery_type, subnet_ids: ids };
		}
	}

	function addRouted(subnetId: string) {
		formData = withRoutedTarget(formData, subnetId);
	}

	// "Add a network" creates a subnet over this modal and adds it to the routed list, which saves
	// with the scan.
	let addingNetwork = $state(false);

	async function handleNetworkCreated(subnet: Subnet) {
		try {
			const created = await createSubnetMutation.mutateAsync({ subnet, purpose: 'Scan' });
			addRouted(created.id);
			addingNetwork = false;
		} catch {
			// The API client shows the error.
		}
	}

	function handleExistingNetwork(subnet: SubnetResponse) {
		addRouted(subnet.id);
		addingNetwork = false;
	}
</script>

{#snippet leaveOutHint()}
	<p>{discovery_daemonNetworksHelp()}</p>
	<div class="flex flex-wrap items-baseline gap-x-1">
		<DocsHint
			text={discovery_leaveOutNetwork()}
			href={docsUrl(
				'/docs/guides/networking/multiple-daemons/#restricting-daemons-to-specific-interfaces'
			)}
			linkText={discovery_leaveOutNetworkLinkText()}
		/>
	</div>
{/snippet}

<div class="space-y-6">
	{#if daemonHostId == null}
		<InlineWarning title={discovery_daemonHostMissing()} body={discovery_daemonHostMissingHelp()} />
	{/if}

	{#if hasTargets}
		<ListManager
			label={discovery_daemonNetworks()}
			helpSnippet={leaveOutHint}
			emptyMessage={discovery_daemonNetworksEmpty()}
			allowReorder={false}
			allowAddFromOptions={false}
			allowItemEdit={false}
			items={daemonNetworks}
			optionDisplayComponent={SubnetDisplay}
			itemDisplayComponent={SubnetDisplay}
			itemRemoveUnavailable={() => disabledWhen(true, discovery_daemonNetworkAlwaysScanned())}
		/>

		{#if largeRoutedSubnets.length > 0}
			<InlineWarning
				title=""
				body={discovery_routedNetworksLarge({ subnets: largeRoutedSubnets.join('\n') })}
			/>
		{/if}
		<ListManager
			label={discovery_routedNetworks()}
			helpText={discovery_routedNetworksHelp()}
			placeholder={discovery_selectSubnet()}
			emptyMessage={discovery_routedNetworksEmpty()}
			allowReorder={false}
			allowItemEdit={false}
			allowCreateNew={true}
			createNewLabel={discovery_addNetwork()}
			onCreateNew={() => (addingNetwork = true)}
			showSearch={true}
			options={availableSubnets}
			items={routedSubnets}
			optionDisplayComponent={SubnetDisplay}
			itemDisplayComponent={SubnetDisplay}
			onAdd={addRouted}
			onRemove={(index) => setRoutedIds(routedIds.filter((_, i) => i !== index))}
		/>
	{/if}
</div>

<!-- Unnamed, so the discovery editor underneath stays open. -->
<SubnetEditModal
	isOpen={addingNetwork}
	fixedScan={{ siteId: formData.site_id, label: `${daemon.name}: ${formData.name}` }}
	onCreate={handleNetworkCreated}
	onUseExisting={handleExistingNetwork}
	onUpdate={() => {}}
	onClose={() => (addingNetwork = false)}
/>
