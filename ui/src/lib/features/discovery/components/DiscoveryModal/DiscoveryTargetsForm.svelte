<script lang="ts">
	import { useSubnetsQuery } from '$lib/features/subnets/queries';
	import { SubnetDisplay } from '$lib/shared/components/forms/selection/display/SubnetDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import type { Discovery } from '../../types/base';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import InlineInfo from '$lib/shared/components/feedback/InlineInfo.svelte';
	import { subnetTypes } from '$lib/shared/stores/metadata';
	import type { Daemon } from '$lib/features/daemons/types/base';
	import {
		discovery_allSubnetsScanned,
		discovery_daemonHostMissing,
		discovery_daemonHostMissingHelp,
		discovery_nonInterfacedSubnet,
		discovery_nonInterfacedSubnetWarning,
		discovery_nonInterfacedSubnetSlow,
		discovery_selectSubnet,
		discovery_targetSubnets,
		discovery_targetSubnetsHelp
	} from '$lib/paraglide/messages';

	interface Props {
		formData: Discovery;
		daemonHostId: string | null;
		daemon: Daemon;
	}

	let { formData = $bindable(), daemonHostId, daemon }: Props = $props();

	const subnetsQuery = useSubnetsQuery();

	let subnetsData = $derived(subnetsQuery.data ?? []);

	let availableSubnets = $derived(
		subnetsData.filter(
			(s) =>
				(formData.discovery_type.type === 'Network' ||
					formData.discovery_type.type === 'Unified') &&
				s.site_id == formData.site_id &&
				!formData.discovery_type.subnet_ids?.includes(s.id) &&
				subnetTypes.getMetadata(s.subnet_type).network_scan_discovery_eligible
		)
	);

	let selectedSubnets = $derived(
		(formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') &&
			formData.discovery_type.subnet_ids
			? formData.discovery_type.subnet_ids
					.map((id) => subnetsData.find((s) => s.id === id))
					.filter(Boolean)
			: []
	);

	function getCidrPrefix(cidr: string): number | null {
		const parts = cidr.split('/');
		if (parts.length !== 2) return null;
		const prefix = parseInt(parts[1], 10);
		return isNaN(prefix) ? null : prefix;
	}

	let nonInterfacedSubnetData = $derived(
		(formData.discovery_type.type == 'Network' || formData.discovery_type.type == 'Unified') &&
			formData.discovery_type.subnet_ids &&
			formData.discovery_type.subnet_ids.length > 0
			? formData.discovery_type.subnet_ids
					.filter((s) => !daemon.interfaced_subnet_ids.includes(s))
					.map((s) => subnetsData.find((subnet) => subnet.id == s))
					.filter((s) => s != undefined)
			: []
	);

	let nonInterfacedSubnetNames = $derived(
		nonInterfacedSubnetData.map((s) => (s.name !== s.cidr ? s.name + ` (${s.cidr})` : s.cidr))
	);

	function formatIpCount(count: number): string {
		if (count >= 1_000_000) return `~${Math.round(count / 1_000_000)}M IPs`;
		if (count >= 1_000) return `~${Math.round(count / 1_000)}K IPs`;
		return `${count} IPs`;
	}

	let largeNonInterfacedSubnets = $derived(
		nonInterfacedSubnetData
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

	let hasLargeNonInterfacedSubnet = $derived(largeNonInterfacedSubnets.length > 0);

	function handleAddSubnet(subnetId: string) {
		if (formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') {
			const currentIds = formData.discovery_type.subnet_ids || [];
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: [...currentIds, subnetId]
			};
		}
	}

	function handleRemoveSubnet(index: number) {
		if (
			(formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified') &&
			formData.discovery_type.subnet_ids
		) {
			formData.discovery_type = {
				...formData.discovery_type,
				subnet_ids: formData.discovery_type.subnet_ids.filter((_, i) => i !== index)
			};
		}
	}
</script>

<div class="space-y-4">
	{#if daemonHostId == null}
		<InlineWarning title={discovery_daemonHostMissing()} body={discovery_daemonHostMissingHelp()} />
	{/if}

	{#if formData.discovery_type.type === 'Network' || formData.discovery_type.type === 'Unified'}
		<div class="card">
			<ListManager
				label={discovery_targetSubnets()}
				helpText={discovery_targetSubnetsHelp()}
				placeholder={discovery_selectSubnet()}
				emptyMessage={discovery_allSubnetsScanned()}
				allowReorder={false}
				allowItemEdit={() => false}
				showSearch={true}
				options={availableSubnets}
				items={selectedSubnets}
				optionDisplayComponent={SubnetDisplay}
				itemDisplayComponent={SubnetDisplay}
				onAdd={handleAddSubnet}
				onRemove={handleRemoveSubnet}
			/>
		</div>
		{#if nonInterfacedSubnetNames.length > 0}
			{#if hasLargeNonInterfacedSubnet}
				<InlineWarning
					title={discovery_nonInterfacedSubnet()}
					body={discovery_nonInterfacedSubnetSlow({
						subnets: nonInterfacedSubnetNames.join('\n'),
						largeSubnets: largeNonInterfacedSubnets.join('\n')
					})}
				/>
			{:else}
				<InlineInfo
					title={discovery_nonInterfacedSubnet()}
					body={discovery_nonInterfacedSubnetWarning({
						subnets: nonInterfacedSubnetNames.join('\n')
					})}
				/>
			{/if}
		{/if}
	{/if}
</div>
