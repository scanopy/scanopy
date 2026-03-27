<script lang="ts">
	import type { Interface } from '$lib/features/hosts/types/base';
	import {
		required,
		ipAddressFormat,
		ipAddressInCidrFormat,
		macFormat,
		max
	} from '$lib/shared/components/forms/validators';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_ipAddress,
		common_macAddress,
		common_name,
		common_placeholderInterface,
		common_placeholderIpAddress,
		hosts_interfaces_ipMustBeWithin,
		hosts_interfaces_macFormat,
		hosts_interfaces_macReadOnly
	} from '$lib/paraglide/messages';

	interface Props {
		iface: Interface;
		subnet: Subnet;
		index: number;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (iface: Interface) => void;
		isEditing?: boolean;
	}

	let { iface, subnet, index, form, onChange = () => {}, isEditing = false }: Props = $props();

	// Field names for this interface in the form array
	let ipFieldName = $derived(`interfaces[${index}].ip_address`);
	let macFieldName = $derived(`interfaces[${index}].mac_address`);
	let nameFieldName = $derived(`interfaces[${index}].name`);

	// Notify parent of changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...iface, name: value || null });
	}

	function handleIpChange(value: string) {
		onChange({ ...iface, ip_address: value });
	}

	function handleMacChange(value: string) {
		onChange({ ...iface, mac_address: value || null });
	}
</script>

{#if subnet}
	<div class="space-y-6">
		<div class="border-b border-gray-600 pb-4">
			<h3 class="text-primary flex items-center gap-1.5 text-sm font-medium">
				Interface on
				<EntityTag
					entityRef={entityRef('Subnet', subnet.id, subnet)}
					label={subnet?.name && subnet.name !== subnet.cidr
						? `${subnet.name} (${subnet.cidr})`
						: subnet.cidr}
					icon={entities.getIconComponent('Subnet')}
					color={entities.getColorHelper('Subnet').color}
				/>
			</h3>
			{#if subnet?.description}
				<p class="text-secondary mt-1 text-sm">{subnet.description}</p>
			{/if}
		</div>

		<div class="space-y-4">
			<form.Field
				name={nameFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => max(100)(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleNameChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_name()}
						id="interface_{iface.id}"
						placeholder={common_placeholderInterface()}
						{field}
					/>
				{/snippet}
			</form.Field>

			<form.Field
				name={ipFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) =>
						required(value) || ipAddressFormat(value) || ipAddressInCidrFormat(subnet.cidr)(value),
					onChange: ({ value }: { value: string }) =>
						required(value) || ipAddressFormat(value) || ipAddressInCidrFormat(subnet.cidr)(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleIpChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_ipAddress()}
						id="interface_ip_{iface.id}"
						placeholder={subnet.cidr.includes(':') ? '2001:db8::1' : common_placeholderIpAddress()}
						required={true}
						helpText={hosts_interfaces_ipMustBeWithin({ cidr: subnet.cidr })}
						{field}
					/>
				{/snippet}
			</form.Field>

			<form.Field
				name={macFieldName}
				validators={{
					onBlur: ({ value }: { value: string }) => macFormat(value)
				}}
				listeners={{
					onChange: ({ value }: { value: string }) => handleMacChange(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_macAddress()}
						id="interface_mac_{iface.id}"
						placeholder="00:1B:44:11:3A:B7"
						helpText={isEditing ? hosts_interfaces_macReadOnly() : hosts_interfaces_macFormat()}
						disabled={isEditing}
						{field}
					/>
				{/snippet}
			</form.Field>
		</div>
	</div>
{/if}
