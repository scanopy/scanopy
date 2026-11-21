<script lang="ts">
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { Pencil } from '@lucide/svelte';
	import { max } from '#lib/shared/components/forms/validators.ts';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import {
		common_name,
		hosts_identity_editName,
		subnets_nameFromCidr,
		subnets_nameOverrideHelp
	} from '#lib/paraglide/messages.js';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		/** The range as typed, which names the subnet while no name is given. */
		cidr: string;
		/** The name the form opened with, blank when the subnet is named after its range. */
		initialName: string;
	}

	let { form, cidr, initialName }: Props = $props();

	// The name field's live value, so the name shown follows it as the user types. TanStack's field
	// state is not tracked by `$derived`, so this follows the input instead.
	let liveName = $state('');
	// The name field stays hidden until the edit button opens it, unless the subnet has a name.
	let editingName = $state(false);

	$effect.pre(() => {
		liveName = initialName;
		editingName = false;
	});
</script>

<div class="space-y-3">
	<div class="flex flex-wrap items-center gap-x-3 gap-y-2">
		{#if liveName.trim()}
			<span class="text-lg font-semibold break-all text-primary">{liveName}</span>
		{:else}
			<span class="text-lg font-semibold break-all text-primary">{cidr}</span>
			<Tag label={subnets_nameFromCidr()} color="Gray" />
		{/if}
		<button
			type="button"
			class="rounded-sm p-1 text-secondary transition-colors hover:bg-white/10 hover:text-primary"
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
				if (event.target instanceof HTMLInputElement) liveName = event.target.value;
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
						label={common_name()}
						id="name"
						placeholder={cidr}
						helpText={subnets_nameOverrideHelp()}
						{field}
					/>
				{/snippet}
			</form.Field>
		</div>
	{/if}
</div>
