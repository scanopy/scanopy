<script lang="ts">
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { IPAddressDisplay } from './IPAddressDisplay.svelte';
	import { ipAddressBindingOptions } from './bindingOptions';
	import { formatIPAddress } from '$lib/features/hosts/address-labels';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import type { IPAddressBinding, Service } from '$lib/features/services/types/base';
	import {
		hosts_bindings_hostNotFound,
		hosts_bindings_ipAddressBinding,
		hosts_bindings_noIPAddresses,
		hosts_bindings_serviceNotFound,
		hosts_bindings_unknownIPAddress
	} from '$lib/paraglide/messages';

	// TanStack Query hooks
	const ipAddressesQuery = useIPAddressesQuery();
	const subnetsQuery = useSubnetsQuery();
	let ipAddressesData = $derived(ipAddressesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	interface Props {
		binding: IPAddressBinding;
		onUpdate?: (updates: Partial<IPAddressBinding>) => void;
		service?: Service;
		host?: HostFormData;
	}

	let { binding, onUpdate = () => {}, service = undefined, host = undefined }: Props = $props();

	// IP address binding must have an ip_address_id - look up from host form data first (for unsaved hosts),
	// then fall back to query data (for saved hosts)
	let ipAddr = $derived(
		binding.ip_address_id
			? (host?.ip_addresses.find((i) => i.id === binding.ip_address_id) ??
					ipAddressesData.find((i) => i.id === binding.ip_address_id))
			: null
	);

	let ipAddressOptions = $derived(
		ipAddressBindingOptions(host?.ip_addresses ?? [], binding, service)
	);
	let disabledReasons = $derived(
		new Map(ipAddressOptions.map((o) => [IPAddressDisplay.getId(o.item), o.disabledReason]))
	);

	// Shown at once, before the parent hands back the updated binding.
	let selectedValue = $derived(binding.ip_address_id ?? null);

	function handleSelect(newValue: string) {
		selectedValue = newValue;
		if (newValue !== binding.ip_address_id) {
			onUpdate({ ip_address_id: newValue });
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="flex-1" onclick={(e) => e.stopPropagation()}>
	<div class="text-secondary mb-1 block text-xs font-medium">
		{hosts_bindings_ipAddressBinding()}
	</div>

	{#if !service}
		<div class="text-danger rounded border border-red-600 bg-red-900/20 px-2 py-1 text-xs">
			{hosts_bindings_serviceNotFound()}
		</div>
	{:else if !host}
		<div class="text-danger rounded border border-red-600 bg-red-900/20 px-2 py-1 text-xs">
			{hosts_bindings_hostNotFound()}
		</div>
	{:else}
		<div class="flex gap-3">
			<div class="flex-1">
				{#if host.ip_addresses && host.ip_addresses.length === 0}
					<InlineWarning title="" body={hosts_bindings_noIPAddresses()} />
				{:else if host.ip_addresses && host.ip_addresses.length === 1}
					<!-- Single IP address - show as read-only -->
					<div
						class="text-secondary rounded px-2 py-1 text-sm"
						style="border: 1px solid var(--color-border-input); background: var(--color-bg-input)"
					>
						{ipAddr
							? formatIPAddress(ipAddr, isContainerSubnetFn)
							: hosts_bindings_unknownIPAddress()}
					</div>
				{:else if host.ip_addresses.length > 0}
					<!-- Multiple IP addresses - show as dropdown -->
					<RichSelect
						{selectedValue}
						options={ipAddressOptions.map((o) => o.item)}
						displayComponent={IPAddressDisplay}
						getOptionContext={(option) => ({
							subnets: subnetsData,
							disabledReason: disabledReasons.get(IPAddressDisplay.getId(option)) ?? null
						})}
						onSelect={handleSelect}
					/>
				{/if}
			</div>
		</div>
	{/if}
</div>
