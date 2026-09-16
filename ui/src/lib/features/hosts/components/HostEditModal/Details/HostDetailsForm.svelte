<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { max } from '$lib/shared/components/forms/validators';
	import TextArea from '$lib/shared/components/forms/input/TextArea.svelte';
	import SelectNetwork from '$lib/features/networks/components/SelectNetwork.svelte';
	import TagPicker from '$lib/features/tags/components/TagPicker.svelte';
	import IdentitySection from './IdentitySection.svelte';
	import DeviceFactsSection from './DeviceFactsSection.svelte';
	import {
		common_description,
		hosts_details_descriptionPlaceholder
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing?: boolean;
	}

	let { form, formData = $bindable(), isEditing = false }: Props = $props();

	// network_id is read/written directly against formData — no local
	// snapshot. A prior `$state(formData.network_id)` mirror captured the
	// value once at mount, went stale when HostEditor reassigned formData
	// via resetForm(host), and then got clobbered by SelectNetwork's
	// auto-default (first network) on the falsy initial capture.
</script>

<div class="space-y-6 p-6">
	<div class="card card-static space-y-6">
		<IdentitySection {form} {formData} {isEditing} />

		<!-- Create only: an update keeps the host's existing network whatever the request says. -->
		{#if !isEditing}
			<SelectNetwork
				selectedNetworkId={formData.network_id}
				onNetworkChange={(id) => (formData.network_id = id)}
			/>
		{/if}

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

	{#if isEditing}
		<DeviceFactsSection host={formData} />
	{/if}
</div>
