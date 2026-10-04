<script lang="ts">
	import type { Service, ServiceVirtualization } from '$lib/features/services/types/base';
	import {
		ServiceDisplay,
		type ServiceDisplayContext
	} from '$lib/shared/components/forms/selection/display/ServiceDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import { hostDisplayContext } from '$lib/features/hosts/host-picker.svelte';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import type { Host } from '$lib/features/hosts/types/base';
	import { serviceDefinitions } from '$lib/shared/stores/metadata';
	import {
		common_containers,
		hosts_virtualization_addContainer,
		hosts_virtualization_containerHelp,
		hosts_virtualization_containerHosts,
		hosts_virtualization_containerHostsHelp,
		hosts_virtualization_noContainersYet
	} from '$lib/paraglide/messages';

	interface Props {
		service: Service;
		/** Effective host list (saved + staged edits) from VirtualizationForm. */
		hosts: Host[];
		/** Effective service list (saved + staged edits) from VirtualizationForm. */
		services: Service[];
		onChange: (updatedService: Service) => void;
	}

	let { service, hosts, services, onChange }: Props = $props();

	// Containers on a macvlan or ipvlan network are hosts of their own under this runtime. Only
	// discovery links them, so they are listed read-only.
	let containerHosts = $derived(hosts.filter((h) => h.virtualization_service_id === service.id));
	const ipAddressesQuery = useIPAddressesQuery();

	let serviceMetadata = $derived(serviceDefinitions.getItem(service.service_definition));

	// Derived from the effective service list keyed on this manager — updates as
	// containers are added/removed and resets when a different manager is selected.
	let managedContainers = $derived(
		services.filter((s) => s.virtualization_service_id === service.id)
	);

	let containerIds = $derived(managedContainers.map((s) => s.id));

	// Filter out services on other hosts and already managed containers
	let selectableContainers = $derived(
		services.filter(
			(s) => s.host_id === service.host_id && s.id !== service.id && !containerIds.includes(s.id)
		)
	);

	function handleAddContainer(serviceId: string) {
		const containerizedService = services.find(
			(s) => s.host_id === service.host_id && s.id == serviceId
		);

		const variant = serviceMetadata?.metadata.virtualization_variant;
		if (containerizedService && variant) {
			const updatedService: Service = {
				...containerizedService,
				virtualization_metadata: {
					type: variant,
					details: {
						container_id: null,
						container_name: null
					}
				} as ServiceVirtualization,
				virtualization_service_id: service.id
			};

			// Stage the change; managedContainers re-derives from the effective list.
			onChange(updatedService);
		}
	}

	function handleRemoveContainer(index: number) {
		const removedContainer = managedContainers.at(index);

		if (removedContainer) {
			const updatedService = {
				...removedContainer,
				virtualization_metadata: null,
				virtualization_service_id: null
			};

			onChange(updatedService);
		}
	}

	// Every managed container runs in this runtime, which the manager card beside the list names,
	// so a per-row runtime tag repeats it. Options keep it: they can come from any runtime.
	const containerContext: ServiceDisplayContext = { hideTags: ['virtualization'] };
</script>

<div class="space-y-6">
	<ListManager
		label={common_containers()}
		helpText={hosts_virtualization_containerHelp({ serviceName: serviceMetadata?.name ?? '' })}
		placeholder={hosts_virtualization_addContainer()}
		emptyMessage={hosts_virtualization_noContainersYet()}
		allowReorder={false}
		allowDuplicates={false}
		allowItemEdit={() => false}
		showSearch={true}
		options={selectableContainers}
		items={managedContainers}
		getItemContext={() => containerContext}
		optionDisplayComponent={ServiceDisplay}
		itemDisplayComponent={ServiceDisplay}
		onAdd={handleAddContainer}
		onRemove={handleRemoveContainer}
	/>

	{#if containerHosts.length > 0}
		<ListManager
			label={hosts_virtualization_containerHosts()}
			helpText={hosts_virtualization_containerHostsHelp({
				serviceName: serviceMetadata?.name ?? ''
			})}
			allowReorder={false}
			allowAddFromOptions={false}
			allowItemEdit={() => false}
			allowItemRemove={() => false}
			options={[] as Host[]}
			items={containerHosts}
			getItemContext={() => hostDisplayContext(ipAddressesQuery.data ?? [], services)}
			optionDisplayComponent={HostDisplay}
			itemDisplayComponent={HostDisplay}
		/>
	{/if}
</div>
