<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { hostnameFormat, max, required } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import InfoCard from '$lib/shared/components/data/InfoCard.svelte';
	import InfoRow from '$lib/shared/components/data/InfoRow.svelte';
	import {
		common_contact,
		common_description,
		common_hostname,
		common_location,
		common_name,
		common_placeholderHostname,
		hosts_details_descriptionPlaceholder,
		hosts_details_namePlaceholder,
		hosts_snmp_chassisId,
		hosts_snmp_managementUrl,
		hosts_snmp_sysDescr,
		hosts_snmp_sysObjectId,
		hosts_snmp_systemInfo
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing?: boolean;
	}

	let { form, formData = $bindable(), isEditing = false }: Props = $props();

	// Track network_id separately (not a form field, so needs manual sync)
	let selectedNetworkId = $state(formData.network_id);
	$effect(() => {
		formData.network_id = selectedNetworkId;
	});

	// Check if host has any SNMP system info
	let hasSnmpInfo = $derived(
		!!(
			formData.sys_descr ||
			formData.sys_object_id ||
			formData.sys_location ||
			formData.sys_contact ||
			formData.chassis_id ||
			formData.management_url
		)
	);
</script>

<div class="space-y-6 p-6">
	<div class="flex gap-6" class:flex-col={!isEditing || !hasSnmpInfo}>
		<!-- Form fields column -->
		<div class="min-w-0 space-y-6" class:flex-[3]={isEditing && hasSnmpInfo}>
			<div class="grid grid-cols-2 gap-6">
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }: { value: string }) => required(value) || max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_name()}
							id="name"
							placeholder={hosts_details_namePlaceholder()}
							required={true}
							{field}
						/>
					{/snippet}
				</form.Field>

				<form.Field
					name="hostname"
					validators={{
						onBlur: ({ value }: { value: string }) => hostnameFormat(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={common_hostname()}
							id="hostname"
							placeholder={common_placeholderHostname()}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>

			<SelectNetwork bind:selectedNetworkId />

			<form.Field
				name="description"
				validators={{
					onBlur: ({ value }: { value: string }) => max(500)(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextArea
						label={common_description()}
						id="description"
						placeholder={hosts_details_descriptionPlaceholder()}
						{field}
					/>
				{/snippet}
			</form.Field>

			<TagPicker bind:selectedTagIds={formData.tags} />
		</div>

		<!-- SNMP System Info column (only when editing and has data) -->
		{#if isEditing && hasSnmpInfo}
			<div class="flex-[2]">
				<InfoCard title={hosts_snmp_systemInfo()}>
					<InfoRow label={hosts_snmp_sysDescr()}>{formData.sys_descr || '-'}</InfoRow>
					<InfoRow label={hosts_snmp_sysObjectId()} mono>{formData.sys_object_id || '-'}</InfoRow>
					<InfoRow label={common_location()}>{formData.sys_location || '-'}</InfoRow>
					<InfoRow label={common_contact()}>{formData.sys_contact || '-'}</InfoRow>
					<InfoRow label={hosts_snmp_chassisId()} mono>{formData.chassis_id || '-'}</InfoRow>
					<InfoRow label={hosts_snmp_managementUrl()}>
						{#if formData.management_url}
							<!-- eslint-disable svelte/no-navigation-without-resolve -->
							<a
								href={formData.management_url}
								target="_blank"
								rel="external noopener noreferrer"
								class="break-all text-blue-400 hover:text-blue-300"
							>
								{formData.management_url}
							</a>
							<!-- eslint-enable svelte/no-navigation-without-resolve -->
						{:else}
							-
						{/if}
					</InfoRow>
				</InfoCard>
			</div>
		{/if}
	</div>
</div>
