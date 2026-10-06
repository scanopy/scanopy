<script lang="ts">
	import type { IPAddress } from '$lib/features/hosts/types/base';
	import {
		required,
		ipAddressFormat,
		ipAddressInCidrFormat,
		macFormat,
		max
	} from '$lib/shared/components/forms/validators';
	import EntityTag from '$lib/shared/components/data/EntityTag.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import type { Site } from '$lib/features/sites/types';
	import { getFreshnessTag } from '$lib/shared/utils/freshness';
	import { entityRef } from '$lib/shared/components/data/types';
	import type { Subnet } from '$lib/features/subnets/types/base';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import ConfigHeader from '$lib/shared/components/forms/config/ConfigHeader.svelte';
	import { entities } from '$lib/shared/stores/metadata';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import {
		common_ipAddress,
		common_macAddress,
		common_name,
		common_placeholderIPAddress,
		common_placeholderIpAddress,
		hosts_ipAddresses_ipMustBeWithin,
		hosts_ipAddresses_macFormat,
		hosts_ipAddresses_macReadOnly
	} from '$lib/paraglide/messages';

	interface Props {
		iface: IPAddress;
		subnet: Subnet;
		index: number;
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		onChange?: (iface: IPAddress) => void;
		isEditing?: boolean;
		/** The address's site, whose window judges its staleness. Without it, no Stale tag. */
		site?: Site;
	}

	let {
		iface,
		subnet,
		index,
		form,
		onChange = () => {},
		isEditing = false,
		site = undefined
	}: Props = $props();

	let staleTag = $derived(
		getFreshnessTag(iface, site, {
			entityTypeLabel: entities.getName('IPAddress') || undefined
		})
	);

	// Field names for this interface in the form array
	let ipFieldName = $derived(`ip_addresses[${index}].ip_address`);
	let macFieldName = $derived(`ip_addresses[${index}].mac_address`);
	let nameFieldName = $derived(`ip_addresses[${index}].name`);

	// Notify parent of changes for real-time sync
	function handleNameChange(value: string) {
		onChange({ ...iface, name: value || null });
	}

	function handleIpChange(value: string) {
		onChange({ ...iface, ip_address: value });
	}

	function handleMacChange(value: string) {
		// Absence is `undefined` now that the MAC carries its source: the pair is either present
		// with a provenance or not present at all, so there is no null in between.
		onChange({ ...iface, mac_address: value || undefined });
	}
</script>

{#if subnet}
	<div class="space-y-6">
		<ConfigHeader subtitle={subnet?.description || null}>
			{#snippet heading()}
				{common_ipAddress()} on
				<EntityTag
					entityRef={entityRef('Subnet', subnet.id, subnet)}
					label={subnet?.name && subnet.name !== subnet.cidr
						? `${subnet.name} (${subnet.cidr})`
						: subnet.cidr}
					icon={entities.getIconComponent('Subnet')}
					color={entities.getColorHelper('Subnet').color}
				/>
				{#if staleTag}
					<Tag {...staleTag} />
				{/if}
			{/snippet}
		</ConfigHeader>

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
						placeholder={common_placeholderIPAddress()}
						{field}
					/>
				{/snippet}
			</form.Field>

			<div class="grid grid-cols-2 items-start gap-4">
				<form.Field
					name={ipFieldName}
					validators={{
						onBlur: ({ value }: { value: string }) =>
							required(value) ||
							ipAddressFormat(value) ||
							ipAddressInCidrFormat(subnet.cidr)(value),
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
							placeholder={subnet.cidr.includes(':')
								? '2001:db8::1'
								: common_placeholderIpAddress()}
							required={true}
							helpText={hosts_ipAddresses_ipMustBeWithin({ cidr: subnet.cidr })}
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
							helpText={isEditing ? hosts_ipAddresses_macReadOnly() : hosts_ipAddresses_macFormat()}
							disabled={isEditing}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>
		</div>
	</div>
{/if}
