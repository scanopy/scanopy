<script lang="ts">
	import { ChevronDown, X } from 'lucide-svelte';
	import { createIconComponent } from '$lib/shared/utils/styling';
	import Popover from '$lib/shared/components/data/Popover.svelte';
	import {
		common_close,
		common_none,
		common_noIconsMatch,
		common_searchPlaceholder
	} from '$lib/paraglide/messages';

	/**
	 * Pick one icon from a list of lucide icon names, or none.
	 *
	 * The grid is windowed: only the rows in view (plus a margin) are rendered, so offering the
	 * whole lucide set costs a few dozen buttons rather than well over a thousand.
	 */
	let {
		value,
		icons,
		onChange,
		id,
		label,
		helpText = '',
		disabled = false
	}: {
		value: string | null;
		/** The names on offer, as the backend stores them (kebab-case). */
		icons: string[];
		onChange: (icon: string | null) => void;
		id: string;
		label: string;
		helpText?: string;
		disabled?: boolean;
	} = $props();

	const COLUMNS = 10;
	const ROW_HEIGHT = 36;
	const VIEW_HEIGHT = 216;
	const OVERSCAN_ROWS = 2;

	let open = $state(false);
	let trigger: HTMLButtonElement | undefined = $state();
	let query = $state('');
	let scrollTop = $state(0);
	let scroller: HTMLDivElement | undefined = $state();

	let filtered = $derived.by(() => {
		const needle = query.trim().toLowerCase().replace(/\s+/g, '-');
		return needle ? icons.filter((name) => name.includes(needle)) : icons;
	});

	let rowCount = $derived(Math.ceil(filtered.length / COLUMNS));
	let firstRow = $derived(Math.max(0, Math.floor(scrollTop / ROW_HEIGHT) - OVERSCAN_ROWS));
	let lastRow = $derived(
		Math.min(rowCount, Math.ceil((scrollTop + VIEW_HEIGHT) / ROW_HEIGHT) + OVERSCAN_ROWS)
	);
	let visible = $derived(filtered.slice(firstRow * COLUMNS, lastRow * COLUMNS));

	let SelectedIcon = $derived(value ? createIconComponent(value) : null);

	function handleQuery(next: string) {
		query = next;
		scrollTop = 0;
		if (scroller) scroller.scrollTop = 0;
	}

	function close() {
		open = false;
		query = '';
		scrollTop = 0;
	}

	function choose(name: string | null) {
		onChange(name);
		close();
	}
</script>

<div class="space-y-2">
	<label for={id} class="text-secondary block text-sm font-medium">{label}</label>
	<div class="flex items-center gap-2">
		<button
			bind:this={trigger}
			{id}
			type="button"
			aria-haspopup="dialog"
			aria-expanded={open}
			class="input-field flex w-40 items-center justify-between gap-2"
			{disabled}
			onclick={() => (open = !open)}
		>
			<span class="flex items-center gap-2">
				{#if SelectedIcon}
					<SelectedIcon class="h-4 w-4" />
					<span class="text-primary truncate text-sm">{value}</span>
				{:else}
					<span class="text-tertiary text-sm">{common_none()}</span>
				{/if}
			</span>
			<ChevronDown class="text-tertiary h-4 w-4 shrink-0" />
		</button>
		{#if value && !disabled}
			<button
				type="button"
				class="btn-icon"
				title={common_none()}
				aria-label={common_none()}
				onclick={() => choose(null)}
			>
				<X class="h-4 w-4" />
			</button>
		{/if}
	</div>

	<!-- Portalled out of the surrounding modal and floated beside the trigger, so opening it
	     never grows the modal and pushes the fields below it out of view. -->
	<Popover
		triggerElement={trigger ?? null}
		isOpen={open && !disabled}
		onClose={close}
		role="dialog"
		ariaLabel={label}
		maxWidth="420px"
	>
		<div class="w-[380px] space-y-2">
			<div class="flex items-center gap-2">
				<input
					type="text"
					class="input-field w-full"
					placeholder={common_searchPlaceholder()}
					value={query}
					oninput={(e) => handleQuery(e.currentTarget.value)}
				/>
				<button type="button" class="btn-icon" aria-label={common_close()} onclick={close}>
					<X class="h-4 w-4" />
				</button>
			</div>
			{#if filtered.length === 0}
				<p class="text-tertiary px-1 py-2 text-xs">{common_noIconsMatch({ query })}</p>
			{:else}
				<div
					bind:this={scroller}
					class="overflow-y-auto"
					style="height: {VIEW_HEIGHT}px;"
					onscroll={(e) => (scrollTop = e.currentTarget.scrollTop)}
				>
					<div class="relative" style="height: {rowCount * ROW_HEIGHT}px;">
						<div
							class="absolute left-0 right-0 grid"
							style="top: {firstRow *
								ROW_HEIGHT}px; grid-template-columns: repeat({COLUMNS}, minmax(0, 1fr));"
						>
							{#each visible as name (name)}
								{@const Icon = createIconComponent(name)}
								<button
									type="button"
									class="flex items-center justify-center rounded-md transition-colors hover:bg-gray-200 dark:hover:bg-gray-700"
									class:ring-2={name === value}
									class:ring-blue-500={name === value}
									style="height: {ROW_HEIGHT}px;"
									title={name}
									aria-label={name}
									onclick={() => choose(name)}
								>
									<Icon class="text-secondary h-4 w-4" />
								</button>
							{/each}
						</div>
					</div>
				</div>
			{/if}
		</div>
	</Popover>

	{#if helpText}
		<p class="text-tertiary text-xs">{helpText}</p>
	{/if}
</div>
