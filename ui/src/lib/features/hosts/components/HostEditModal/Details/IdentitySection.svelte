<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { untrack } from 'svelte';
	import { Pencil } from 'lucide-svelte';
	import type { HostFormData } from '$lib/features/hosts/types/base';
	import { discoveredName, overrideOf, rungLabel } from '$lib/features/hosts/host-identity';
	import { hostnameFormat, max } from '$lib/shared/components/forms/validators';
	import TextInput from '$lib/shared/components/forms/input/TextInput.svelte';
	import Tag from '$lib/shared/components/data/Tag.svelte';
	import AttributeSourceTag from '$lib/shared/components/data/AttributeSourceTag.svelte';
	import { attributeSourceDescription } from '$lib/shared/utils/attribute-source';
	import {
		common_hostname,
		common_name,
		common_placeholderHostname,
		hosts_details_namePlaceholder,
		hosts_identity_editName,
		hosts_identity_fromRung,
		hosts_identity_manualOverride,
		hosts_identity_nothingDiscovered,
		hosts_identity_overrideHelp,
		hosts_identity_overrideLabel,
		hosts_unnamedHost
	} from '$lib/paraglide/messages';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		formData: HostFormData;
		isEditing: boolean;
	}

	let { form, formData, isEditing }: Props = $props();

	// Server data, so it stays what was discovered whatever the override field holds.
	let discovered = $derived(discoveredName(formData.name_ladder ?? []));

	// The override field's live value, so the name above follows it as the user types. TanStack's
	// field state is not tracked by `$derived`, so this follows the input instead. It resets when the
	// editor loads a different host, keyed on the id: submit-time sync writes `formData.name` and
	// must not reset it.
	let liveOverride = $derived.by(() => {
		void formData.id;
		return untrack(() => overrideOf(formData.name ?? '', formData.name_source));
	});

	// The override field stays hidden until the edit button opens it, and closes again when the
	// editor loads a different host.
	let editingName = $derived.by(() => {
		void formData.id;
		return false;
	});
</script>

<div class="space-y-5">
	{#if isEditing}
		<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
			{#if liveOverride.trim()}
				<span class="text-primary break-all text-lg font-semibold">{liveOverride}</span>
				<Tag
					label={hosts_identity_manualOverride()}
					color="Gray"
					title={attributeSourceDescription('Manual')}
				/>
			{:else if discovered}
				<span class="text-primary break-all text-lg font-semibold">{discovered.value}</span>
				{#if discovered.rung !== 'Name'}
					<Tag label={hosts_identity_fromRung({ rung: rungLabel(discovered.rung) })} color="Gray" />
				{/if}
				{#if discovered.source}
					<AttributeSourceTag source={discovered.source} />
				{/if}
			{:else}
				<span class="text-secondary text-lg font-semibold">{hosts_unnamedHost()}</span>
				<span class="text-secondary text-sm">{hosts_identity_nothingDiscovered()}</span>
			{/if}
			<button
				type="button"
				class="text-secondary hover:text-primary rounded p-1 transition-colors hover:bg-white/10"
				aria-label={hosts_identity_editName()}
				title={hosts_identity_editName()}
				aria-expanded={editingName}
				onclick={() => (editingName = !editingName)}
			>
				<Pencil class="h-4 w-4" />
			</button>
		</div>

		{#if editingName}
			<div
				oninput={(event) => {
					if (event.target instanceof HTMLInputElement) liveOverride = event.target.value;
				}}
			>
				<form.Field
					name="name"
					validators={{
						onBlur: ({ value }: { value: string }) => max(100)(value)
					}}
				>
					{#snippet children(field: AnyFieldApi)}
						<TextInput
							label={hosts_identity_overrideLabel()}
							id="name"
							placeholder={discovered?.value ?? hosts_unnamedHost()}
							helpText={hosts_identity_overrideHelp()}
							{field}
						/>
					{/snippet}
				</form.Field>
			</div>
		{/if}
	{:else}
		<div class="grid grid-cols-2 gap-6">
			<form.Field
				name="name"
				validators={{
					onBlur: ({ value }: { value: string }) => max(100)(value)
				}}
			>
				{#snippet children(field: AnyFieldApi)}
					<TextInput
						label={common_name()}
						id="name"
						placeholder={hosts_details_namePlaceholder()}
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
	{/if}
</div>
