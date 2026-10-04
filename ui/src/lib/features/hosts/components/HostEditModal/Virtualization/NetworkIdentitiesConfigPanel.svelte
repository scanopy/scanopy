<script lang="ts">
	import type { Service } from '$lib/features/services/types/base';
	import type { Host } from '$lib/features/hosts/types/base';
	import { HostDisplay } from '$lib/shared/components/forms/selection/display/HostDisplay.svelte';
	import ListManager from '$lib/shared/components/forms/selection/ListManager.svelte';
	import { hostDisplayContext } from '$lib/features/hosts/host-picker.svelte';
	import { useIPAddressesQuery } from '$lib/features/ip-addresses/queries';
	import {
		hosts_virtualization_networkIdentities,
		hosts_virtualization_networkIdentitiesHelp,
		hosts_virtualization_noNetworkIdentities
	} from '$lib/paraglide/messages';

	interface Props {
		service: Service;
		/** Effective host list (saved + staged edits) from VirtualizationForm. */
		hosts: Host[];
		/** Effective service list (saved + staged edits), for host service context. */
		services: Service[];
	}

	let { service, hosts, services }: Props = $props();

	// Only the Proxmox integration links a network identity to its host, so the list is read-only.
	let identityHosts = $derived(hosts.filter((h) => h.virtualization_service_id === service.id));
	const ipAddressesQuery = useIPAddressesQuery();
</script>

<div class="space-y-6">
	<ListManager
		label={hosts_virtualization_networkIdentities()}
		helpText={hosts_virtualization_networkIdentitiesHelp()}
		emptyMessage={hosts_virtualization_noNetworkIdentities()}
		allowReorder={false}
		allowAddFromOptions={false}
		allowItemEdit={() => false}
		allowItemRemove={() => false}
		options={[] as Host[]}
		items={identityHosts}
		getItemContext={() =>
			hostDisplayContext(ipAddressesQuery.data ?? [], services, { hideTags: ['guest'] })}
		optionDisplayComponent={HostDisplay}
		itemDisplayComponent={HostDisplay}
	/>
</div>
