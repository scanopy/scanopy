<script lang="ts" generics="T">
	import type { CardAction } from './types';
	import type { EntityColumn } from './table/columns';
	import Tag from './Tag.svelte';
	import FieldValue from './FieldValue.svelte';
	import { getFieldValue } from './controls/fieldValues';
	import { tooltip } from '$lib/shared/actions/tooltip';
	import type { IconComponent } from '$lib/shared/utils/types';

	/**
	 * One card, for any entity.
	 *
	 * There is no per-entity card component, for the same reason there is no
	 * per-entity table: both views render the one field definition the tab
	 * declares. Anything an entity needs to show is a field, so it appears in
	 * both views or in neither — a card cannot hold content the table has never
	 * heard of, which is how schedule, progress and status went missing before.
	 *
	 * Chrome is field metadata rather than card code: the title is the `primary`
	 * field, the tag beside it is the `statusTag` field, and the line underneath
	 * is the `subtitle` field.
	 */
	let {
		item,
		columns,
		actions = [],
		getIcon = null,
		getLink = null,
		selected = false,
		selectable = true,
		onSelectionChange = () => {}
	}: {
		item: T;
		columns: EntityColumn<T>[];
		actions?: CardAction[];
		/** Per row, since a host's icon comes from its first service and a service's from its type. */
		getIcon?: ((item: T) => { icon: IconComponent | null; color?: string }) | null;
		getLink?: ((item: T) => string | undefined) | null;
		selected?: boolean;
		selectable?: boolean;
		onSelectionChange?: (selected: boolean) => void;
	} = $props();

	let primaryColumn = $derived(columns.find((c) => c.display.primary) ?? columns[0] ?? null);
	let statusColumn = $derived(columns.find((c) => c.display.statusTag) ?? null);
	let subtitleColumn = $derived(columns.find((c) => c.display.subtitle) ?? null);

	/** Everything the header already accounts for is not repeated in the body. */
	let bodyColumns = $derived(
		columns.filter((c) => !c.display.primary && !c.display.statusTag && !c.display.subtitle)
	);

	function text(column: EntityColumn<T> | null): string {
		if (!column) return '';
		const value = getFieldValue(item, column.field);
		if (value === null || value === undefined) return '';
		return Array.isArray(value) ? value.join(', ') : String(value);
	}

	let title = $derived(text(primaryColumn));
	let subtitle = $derived(text(subtitleColumn));
	let link = $derived(getLink?.(item));
	let iconSpec = $derived(getIcon?.(item) ?? null);

	let status = $derived.by(() => {
		if (!statusColumn) return null;
		const [first] = statusColumn.display.getItems?.(item) ?? [];
		return first ?? null;
	});

	function handleCheckboxChange(e: Event) {
		onSelectionChange((e.target as HTMLInputElement).checked);
	}
</script>

<div class="card flex h-full flex-col {selected ? 'card-selected' : ''}">
	{#if selectable}
		<div class="absolute right-4 top-4 flex-shrink-0">
			<input
				type="checkbox"
				checked={selected}
				onchange={handleCheckboxChange}
				onclick={(e) => e.stopPropagation()}
				class="checkbox-card h-5 w-5"
			/>
		</div>
	{/if}

	<!-- Header -->
	<div class="mb-4 flex items-start">
		<div class="flex items-center space-x-3">
			{#if iconSpec?.icon}
				<iconSpec.icon size={28} class={iconSpec.color ?? 'text-blue-400'} />
			{/if}
			<div class="min-w-0 flex-1">
				<div class="flex items-center gap-2">
					<div class="min-w-0">
						{#if link}
							<!--
								`resolve()` is for internal SvelteKit routes. This is the entity's
								own address on the network — an absolute URL to somewhere this app
								has no route for, opened in a new tab.
							-->
							<!-- eslint-disable svelte/no-navigation-without-resolve -->
							<a
								href={link}
								class="text-primary hover:text-info text-lg font-semibold"
								target="_blank"
								rel="noreferrer"
							>
								{title}
							</a>
							<!-- eslint-enable svelte/no-navigation-without-resolve -->
						{:else}
							<h3 class="text-primary text-lg font-semibold">{title}</h3>
						{/if}
					</div>
					{#if status}
						<div class="flex-shrink-0">
							<Tag
								label={status.label}
								color={status.color}
								icon={status.icon}
								href={status.href ?? ''}
								title={status.title ?? ''}
							/>
						</div>
					{/if}
				</div>
				{#if subtitle}
					<p class="text-secondary text-sm">{subtitle}</p>
				{/if}
			</div>
		</div>
	</div>

	<!-- Fields -->
	<div class="flex-grow space-y-3">
		{#each bodyColumns as column (column.id)}
			<div class="flex flex-wrap items-center gap-2 text-sm">
				<span class="text-secondary">{column.label}:</span>
				<FieldValue {item} {column} />
			</div>
		{/each}
	</div>

	<!-- Actions -->
	{#if actions.length > 0}
		<div class="card-divider-h mt-4 flex items-center justify-between pt-4">
			{#each actions as action (action.label)}
				{@const cls = action.class ? action.class : 'btn-icon'}
				{@const explicitTooltip =
					typeof action.tooltip === 'function'
						? action.tooltip(!!action.disabled)
						: (action.tooltip ?? null)}
				<!--
					The label floats in a tooltip rather than growing inside the button.
					An in-flow label has to span its neighbours to fit its text, which
					made the widest action cover the ones beside it. Matches the table.
				-->
				<button
					onclick={action.onClick}
					disabled={action.disabled}
					use:tooltip
					data-tooltip={explicitTooltip ?? action.label}
					aria-label={action.label}
					class="{cls} disabled:cursor-not-allowed disabled:opacity-50"
				>
					{#if action.forceLabel}
						<!-- Always-labelled action: the label is the affordance, not a hover
						     reveal, so it renders in flow and the button sizes to it. -->
						<div class="flex items-center justify-center whitespace-nowrap">
							<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
							<span class="ml-2">{action.label}</span>
						</div>
					{:else}
						<action.icon size={16} class="flex-shrink-0 {action.animation || ''}" />
					{/if}
				</button>
			{/each}
		</div>
	{/if}
</div>
