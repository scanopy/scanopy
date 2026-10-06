<script lang="ts">
	import type { IPAddress } from '$lib/features/hosts/types/base';
	import { untrack } from 'svelte';
	import {
		ipAddressFormat,
		ipAddressInCidrFormat,
		max,
		required
	} from '$lib/shared/components/forms/validators';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import {
		common_ipAddress,
		common_name,
		hosts_ipAddresses_ipMustBeWithin,
		hosts_ipAddresses_subnet
	} from '$lib/paraglide/messages';

	interface Props {
		iface: IPAddress;
		subnet: Subnet;
		onChange?: (updatedIface: IPAddress) => void;
	}

	let { iface, subnet, onChange = () => {} }: Props = $props();

	// Local state for form fields (untrack: $effect below handles prop changes)
	let ipAddress = $state(untrack(() => iface.ip_address || ''));
	let name = $state(untrack(() => iface.name || ''));

	// Error states
	let ipError = $state<string | undefined>(undefined);
	let nameError = $state<string | undefined>(undefined);

	// Track current interface ID to reset when interface changes
	let currentIPAddressId = $state(untrack(() => iface.id));

	// Reset fields when interface changes
	$effect(() => {
		if (iface.id !== currentIPAddressId) {
			currentIPAddressId = iface.id;
			ipAddress = iface.ip_address || '';
			name = iface.name || '';
			ipError = undefined;
			nameError = undefined;
		}
	});

	// Validate and update on change
	function handleIpChange(e: Event) {
		const value = (e.target as HTMLInputElement).value;
		ipAddress = value;
		ipError = ipAddressFormat(value) || ipAddressInCidrFormat(subnet.cidr)(value);
		triggerOnChange();
	}

	function handleNameChange(e: Event) {
		const value = (e.target as HTMLInputElement).value;
		name = value;
		nameError = required(value) || max(100)(value);
		triggerOnChange();
	}

	function triggerOnChange() {
		const updatedIface: IPAddress = {
			...iface,
			ip_address: ipAddress,
			name: name
		};

		// Only trigger onChange if values actually changed
		if (updatedIface.ip_address !== iface.ip_address || updatedIface.name !== iface.name) {
			onChange(updatedIface);
		}
	}
</script>

{#if subnet}
	<div class="space-y-6">
		<ConfigHeader
			title={hosts_ipAddresses_subnet({ name: subnet?.name ? subnet.name : subnet.cidr })}
			subtitle={subnet?.description}
		/>

		<div class="space-y-4">
			<div>
				<label for="interface_{iface.id}" class="text-secondary mb-1 block text-sm font-medium">
					{common_name()} <span class="text-red-400">*</span>
				</label>
				<input
					type="text"
					id="interface_{iface.id}"
					class="input-field w-full"
					placeholder="en0"
					value={name}
					oninput={handleNameChange}
				/>
				{#if nameError}
					<p class="mt-1 text-xs text-red-400">{nameError}</p>
				{/if}
			</div>

			<div>
				<label for="interface_ip_{iface.id}" class="text-secondary mb-1 block text-sm font-medium">
					{common_ipAddress()} <span class="text-red-400">*</span>
				</label>
				<input
					type="text"
					id="interface_ip_{iface.id}"
					class="input-field w-full"
					placeholder={subnet.cidr.includes(':') ? '2001:db8::1' : '192.168.1.100'}
					value={ipAddress}
					oninput={handleIpChange}
				/>
				{#if ipError}
					<p class="mt-1 text-xs text-red-400">{ipError}</p>
				{:else}
					<p class="text-tertiary mt-1 text-xs">
						{hosts_ipAddresses_ipMustBeWithin({ cidr: subnet.cidr })}
					</p>
				{/if}
			</div>
		</div>
	</div>
{/if}
