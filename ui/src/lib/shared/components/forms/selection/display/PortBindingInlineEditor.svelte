<script lang="ts">
	import { untrack } from 'svelte';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { useServicesCacheQuery } from '$lib/features/services/queries';
	import { useSubnetsQuery, isContainerSubnet } from '$lib/features/subnets/queries';
	import type { PortBinding, Service } from '$lib/features/services/types/base';
	import InlineDanger from '$lib/shared/components/feedback/InlineDanger.svelte';
	import InlineWarning from '$lib/shared/components/feedback/InlineWarning.svelte';
	import RichSelect from '$lib/shared/components/forms/selection/RichSelect.svelte';
	import { ALL_IP_ADDRESSES_ID, IPAddressDisplay } from './IPAddressDisplay.svelte';
	import { PortDisplay } from './PortDisplay.svelte';
	import {
		conflictingPortService,
		portBindingInterfaceOptions,
		portBindingPortOptions
	} from './bindingOptions';
	import {
		common_ipAddress,
		common_port,
		hosts_bindings_hostNotFound,
		hosts_bindings_noIPAddresses,
		hosts_bindings_noPorts,
		hosts_bindings_portBinding,
		hosts_bindings_serviceNotFound,
		hosts_ports_noAvailableOnInterface
	} from '$lib/paraglide/messages';

	// TanStack Query hooks
	const servicesQuery = useServicesCacheQuery();
	const subnetsQuery = useSubnetsQuery();
	let servicesData = $derived(servicesQuery.data ?? []);
	let subnetsData = $derived(subnetsQuery.data ?? []);

	// Helper to check if subnet is a container subnet
	let isContainerSubnetFn = $derived((subnetId: string) => {
		const subnet = subnetsData.find((s) => s.id === subnetId);
		return subnet ? isContainerSubnet(subnet) : false;
	});

	interface Props {
		binding: PortBinding;
		onUpdate?: (updates: Partial<PortBinding>) => void;
		service?: Service;
		host?: HostFormData;
		services?: Service[];
	}

	let {
		binding,
		onUpdate = () => {},
		service = undefined,
		host = undefined,
		services = undefined
	}: Props = $props();

	// Use services from props (current editing state) if provided, otherwise fall back to global cache
	let effectiveServicesData = $derived(services ?? servicesData);

	let ipAddressOptions = $derived(
		portBindingInterfaceOptions(host?.ip_addresses ?? [], binding, service)
	);
	let ipAddressReasons = $derived(
		new Map(ipAddressOptions.map((o) => [IPAddressDisplay.getId(o.item), o.disabledReason]))
	);

	let portOptions = $derived(
		portBindingPortOptions(host?.ports ?? [], binding, service, effectiveServicesData)
	);
	let portReasons = $derived(
		new Map(portOptions.map((o) => [PortDisplay.getId(o.item), o.disabledReason]))
	);

	// Local state for the selected values, shown at once before the parent hands back the
	// updated binding. ALL_IP_ADDRESSES (a null id) is held as its option id.
	let selectedInterface = $state(untrack(() => binding.ip_address_id ?? ALL_IP_ADDRESSES_ID));
	let selectedPort = $state(untrack(() => binding.port_id ?? ''));

	// Sync local state when binding changes externally
	$effect(() => {
		selectedInterface = binding.ip_address_id ?? ALL_IP_ADDRESSES_ID;
		selectedPort = binding.port_id ?? '';
	});

	// Check if there are any valid (non-disabled) port options
	let hasValidPortOptions = $derived(portOptions.some((opt) => opt.disabledReason === null));

	function conflictOn(portId: string, interfaceId: string | null) {
		return conflictingPortService(portId, interfaceId, binding, service, effectiveServicesData);
	}

	function handleInterfaceChange(newValue: string) {
		selectedInterface = newValue;

		const interfaceId = newValue === ALL_IP_ADDRESSES_ID ? null : newValue;
		if (interfaceId !== binding.ip_address_id) {
			// Check if current port is still valid on the new interface
			const currentPortConflict = binding.port_id ? conflictOn(binding.port_id, interfaceId) : null;

			if (currentPortConflict || !binding.port_id) {
				// Current port conflicts on new interface OR no port selected - find first valid port
				const firstValidPort = host?.ports.find((p) => !conflictOn(p.id, interfaceId));
				// Reset to valid port, or empty if none available
				onUpdate({ ip_address_id: interfaceId, port_id: firstValidPort?.id ?? '' });
			} else {
				onUpdate({ ip_address_id: interfaceId });
			}
		}
	}

	function handlePortChange(newValue: string) {
		selectedPort = newValue;

		if (newValue !== binding.port_id) {
			onUpdate({ port_id: newValue });
		}
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div class="flex-1" onclick={(e) => e.stopPropagation()}>
	<div class="text-secondary mb-1 block text-xs font-medium">{hosts_bindings_portBinding()}</div>

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
			{#if host.ip_addresses && host.ip_addresses.length === 0}
				<div class="flex-1">
					<InlineWarning title="" body={hosts_bindings_noIPAddresses()} />
				</div>
			{:else if host.ip_addresses.length > 0}
				<div class="min-w-0 flex-1">
					<div class="text-tertiary mb-1 block text-xs">{common_ipAddress()}</div>
					<RichSelect
						selectedValue={selectedInterface}
						options={ipAddressOptions.map((o) => o.item)}
						displayComponent={IPAddressDisplay}
						getOptionContext={(option) => ({
							subnets: subnetsData,
							disabledReason: ipAddressReasons.get(IPAddressDisplay.getId(option)) ?? null
						})}
						onSelect={handleInterfaceChange}
					/>
				</div>
			{/if}

			{#if host.ports.length === 0}
				<div class="flex-1">
					<div
						class="rounded border border-yellow-600 bg-yellow-900/20 px-2 py-1 text-xs text-warning"
					>
						{hosts_bindings_noPorts()}
					</div>
				</div>
			{:else if !hasValidPortOptions}
				<div class="flex-1">
					<div class="text-tertiary mb-1 block text-xs">{common_port()}</div>
					<InlineDanger title={hosts_ports_noAvailableOnInterface()} />
				</div>
			{:else}
				<div class="min-w-0 flex-1">
					<div class="text-tertiary mb-1 block text-xs">{common_port()}</div>
					<RichSelect
						selectedValue={selectedPort}
						options={portOptions.map((o) => o.item)}
						displayComponent={PortDisplay}
						getOptionContext={(option) => ({
							currentServices: effectiveServicesData,
							ip_addresses: host?.ip_addresses ?? [],
							isContainerSubnet: isContainerSubnetFn,
							disabledReason: portReasons.get(PortDisplay.getId(option)) ?? null
						})}
						onSelect={handlePortChange}
					/>
				</div>
			{/if}
		</div>
	{/if}
</div>
