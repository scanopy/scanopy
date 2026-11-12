<!--
	One device fact a person can set: the value with its source, and a pencil that opens the field
	under the row. The same pattern as the host's name (`IdentitySection`): the discovered value
	stays in view, and what a person types is shown in its place, tagged as an override, as they
	type.

	Always rendered, empty or not, so a device no scan identified still offers somewhere to type
	what it is. What the save sends is decided in `device-labels.ts`.
-->
<script lang="ts">
	import type { Snippet } from 'svelte';
	import { untrack } from 'svelte';
	import type { AnyFieldApi } from '@tanstack/svelte-form';
	import { Pencil } from '@lucide/svelte';
	import TextInput from '#lib/shared/components/forms/input/TextInput.svelte';
	import RichSelect from '#lib/shared/components/forms/selection/RichSelect.svelte';
	import {
		DeviceTypeDisplay,
		deviceTypeOptions
	} from '#lib/features/hosts/components/DeviceTypeDisplay.ts';
	import Tag from '#lib/shared/components/data/Tag.svelte';
	import { max } from '#lib/shared/components/forms/validators.ts';
	import { attributeSourceTag } from '#lib/shared/utils/attribute-source.ts';
	import { deviceTypes } from '#lib/shared/stores/metadata.ts';
	import { hosts_deviceFacts_edit } from '#lib/paraglide/messages.js';

	interface Props {
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		form: { Field: any };
		/** The form field this row edits. */
		name: string;
		label: string;
		/** A free-text fact, or the device type, picked from the device-types fixture. */
		kind: 'text' | 'deviceType';
		/** The value the form opened with, which the shown value is compared against. */
		initial: string;
		/** Changes when the editor loads another host, which closes the row and resets it. */
		hostId: string | undefined;
		/** The stored value as the facts card draws it. `—` when there is none. */
		display: Snippet;
		/** The stored value's source tag. */
		source: Snippet;
	}

	let { form, name, label, kind, initial, hostId, display, source }: Props = $props();

	// TanStack field state is not tracked by `$derived`, so the shown value follows the input.
	let live = $derived.by(() => {
		void hostId;
		return untrack(() => initial);
	});
	let editing = $derived.by(() => {
		void hostId;
		return false;
	});

	let changed = $derived(live !== initial);
	let options = $derived(kind === 'deviceType' ? deviceTypeOptions() : []);
	let liveText = $derived(kind === 'deviceType' ? deviceTypes.getName(live) : live.trim());
</script>

<div class="space-y-2">
	<div class="flex flex-wrap items-center gap-x-4 gap-y-1">
		<span class="w-40 shrink-0 text-sm text-secondary">{label}</span>
		<span class="min-w-0 flex-1 text-sm wrap-break-word text-primary">
			{#if changed}
				{#if liveText}{liveText}{:else}<span class="text-secondary">—</span>{/if}
			{:else}
				{@render display()}
			{/if}
		</span>
		<span class="shrink-0">
			{#if changed}
				<Tag {...attributeSourceTag('Manual')} />
			{:else}
				{@render source()}
			{/if}
		</span>
		<button
			type="button"
			class="shrink-0 rounded-sm p-1 text-secondary transition-colors hover:bg-white/10 hover:text-primary"
			aria-label={hosts_deviceFacts_edit({ field: label })}
			title={hosts_deviceFacts_edit({ field: label })}
			aria-expanded={editing}
			onclick={() => (editing = !editing)}
		>
			<Pencil class="h-4 w-4" />
		</button>
	</div>

	{#if editing}
		<div
			oninput={(event) => {
				if (event.target instanceof HTMLInputElement) live = event.target.value;
			}}
		>
			<form.Field
				{name}
				validators={kind === 'text'
					? { onBlur: ({ value }: { value: string }) => max(200)(value) }
					: undefined}
			>
				{#snippet children(field: AnyFieldApi)}
					{#if kind === 'deviceType'}
						<RichSelect
							selectedValue={live}
							{options}
							showSearch
							displayComponent={DeviceTypeDisplay}
							onSelect={(value) => {
								field.handleChange(value);
								live = value;
							}}
						/>
					{:else}
						<TextInput label="" ariaLabel={label} id={name} {field} />
					{/if}
				{/snippet}
			</form.Field>
		</div>
	{/if}
</div>
